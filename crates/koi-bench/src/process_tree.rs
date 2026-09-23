// Process containment requires platform FFI (Job Objects on Windows,
// kill(-pgid) elsewhere) — the workspace's unsafe_code lint is discharged
// here intentionally; keep the surface reviewed, not sprawling.
#![allow(unsafe_code)]
use std::{
    io,
    path::Path,
    process::{Child, ChildStdin, Command},
};

#[cfg(windows)]
use std::process::Stdio;

#[cfg(windows)]
use super::framing::read_bounded_frame;
#[cfg(windows)]
use super::protocol::RESPONSE_PREFIX;

#[cfg(windows)]
const WORKER_PROGRAM_ENV: &str = "KOI_BENCHMARK_WORKER_PROGRAM";
#[cfg(windows)]
const WORKER_ARGUMENTS_ENV: &str = "KOI_BENCHMARK_WORKER_ARGUMENTS";
#[cfg(windows)]
const LAUNCH_TOKEN: u8 = b'!';

#[cfg(all(windows, not(test)))]
pub(super) fn contained_worker_command(program: &Path, arguments: &[String]) -> io::Result<Command> {
    let launcher = std::env::current_exe()?;
    if launcher.file_stem().and_then(|stem| stem.to_str()) != Some("koi-bench") {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows benchmark containment requires the koi-bench executable",
        ));
    }
    let encoded_arguments = serde_json::to_string(arguments).map_err(io::Error::other)?;
    let mut command = Command::new(launcher);
    command
        .arg("--worker-launcher")
        .env(WORKER_PROGRAM_ENV, program)
        .env(WORKER_ARGUMENTS_ENV, encoded_arguments);
    Ok(command)
}

#[cfg(any(unix, all(windows, test)))]
pub(super) fn contained_worker_command(program: &Path, arguments: &[String]) -> io::Result<Command> {
    let mut command = Command::new(program);
    command.args(arguments);
    Ok(command)
}

#[cfg(not(any(windows, unix)))]
pub(super) fn contained_worker_command(_program: &Path, _arguments: &[String]) -> io::Result<Command> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "benchmark worker containment is supported only on Windows and Unix",
    ))
}

#[cfg(all(windows, not(test)))]
pub(super) fn activate_worker(input: &mut ChildStdin) -> io::Result<()> {
    use std::io::Write;

    input.write_all(&[LAUNCH_TOKEN])?;
    input.flush()
}

#[cfg(any(unix, all(windows, test)))]
pub(super) fn activate_worker(_input: &mut ChildStdin) -> io::Result<()> {
    Ok(())
}

#[cfg(not(any(windows, unix)))]
pub(super) fn activate_worker(_input: &mut ChildStdin) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "benchmark worker containment is supported only on Windows and Unix",
    ))
}

#[cfg(windows)]
pub fn run_worker_launcher() -> anyhow::Result<()> {
    use anyhow::{bail, Context};

    let program = std::env::var_os(WORKER_PROGRAM_ENV).context("worker launcher is missing its program")?;
    let arguments = std::env::var(WORKER_ARGUMENTS_ENV).context("worker launcher is missing its arguments")?;
    let arguments: Vec<String> = serde_json::from_str(&arguments).context("worker launcher arguments are invalid")?;
    let token = windows_job::read_activation_token().context("worker launcher activation failed")?;
    if token != LAUNCH_TOKEN {
        bail!("worker launcher received an invalid activation token");
    }

    let mut child = Command::new(program)
        .args(arguments)
        .env_remove(WORKER_PROGRAM_ENV)
        .env_remove(WORKER_ARGUMENTS_ENV)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("worker launcher failed to run the worker")?;
    let Some(worker_stdout) = child.stdout.take() else {
        let _ = child.wait();
        bail!("worker launcher could not capture the worker stdout");
    };

    // The referee's stdout reader admits only prefixed protocol frames. Pinned
    // legacy engine artifacts legitimately print solver diagnostics to stdout,
    // so the trusted launcher filters at this trust boundary: protocol frames
    // pass through verbatim, everything else is demoted to stderr.
    let forwarder =
        std::thread::spawn(move || forward_protocol_frames(worker_stdout, &mut io::stdout(), &mut io::stderr()));

    let status = child.wait().context("worker launcher failed to wait for the worker")?;
    let _ = forwarder.join();
    if !status.success() {
        bail!("worker exited with {status}");
    }
    Ok(())
}

/// Forwards only prefixed protocol frames from the worker to the referee.
/// Legacy workers print solver diagnostics to stdout; the launcher reroutes
/// them to `diagnostics` so the referee's strict reader sees protocol frames
/// only. Returns when the worker's stdout closes.
#[cfg(windows)]
fn forward_protocol_frames(
    worker_stdout: std::process::ChildStdout,
    forward: &mut impl std::io::Write,
    diagnostics: &mut impl std::io::Write,
) {
    use std::io::{BufRead, BufReader};

    fn drain_to_newline(reader: &mut impl BufRead) -> io::Result<()> {
        loop {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                return Ok(());
            }
            match available.iter().position(|&byte| byte == b'\n') {
                Some(newline) => {
                    reader.consume(newline + 1);
                    return Ok(());
                }
                None => {
                    let buffered = available.len();
                    reader.consume(buffered);
                }
            }
        }
    }

    let mut reader = BufReader::new(worker_stdout);
    loop {
        match read_bounded_frame(&mut reader) {
            Ok(Some(line)) => {
                if line.starts_with(RESPONSE_PREFIX) {
                    if writeln!(forward, "{line}").and_then(|()| forward.flush()).is_err() {
                        return;
                    }
                } else if !line.trim().is_empty() {
                    let _ = writeln!(diagnostics, "[worker] {line}");
                }
            }
            Ok(None) => return,
            Err(_) => {
                if drain_to_newline(&mut reader).is_err() {
                    return;
                }
                let _ = writeln!(diagnostics, "[worker] … [oversized stdout line discarded]");
            }
        }
    }
}

#[cfg(not(windows))]
pub fn run_worker_launcher() -> anyhow::Result<()> {
    anyhow::bail!("the benchmark worker launcher is only available on Windows")
}

#[cfg(windows)]
mod windows_job {
    use std::{ffi::c_void, io, mem::size_of, os::windows::io::AsRawHandle, process::Child, ptr};

    type Handle = *mut c_void;

    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: i32 = 9;

    #[repr(C)]
    #[derive(Default)]
    struct BasicLimitInformation {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct IoCounters {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimitInformation {
        basic_limit_information: BasicLimitInformation,
        io_info: IoCounters,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    #[cfg(target_pointer_width = "64")]
    const _: () = assert!(size_of::<BasicLimitInformation>() == 64);
    #[cfg(target_pointer_width = "64")]
    const _: () = assert!(size_of::<IoCounters>() == 48);
    #[cfg(target_pointer_width = "64")]
    const _: () = assert!(size_of::<ExtendedLimitInformation>() == 144);

    #[link(name = "kernel32")]
    #[allow(non_snake_case)]
    unsafe extern "system" {
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn CloseHandle(object: Handle) -> i32;
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> Handle;
        fn GetStdHandle(std_handle: u32) -> Handle;
        fn ReadFile(
            file: Handle,
            buffer: *mut c_void,
            bytes_to_read: u32,
            bytes_read: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn SetInformationJobObject(
            job: Handle,
            information_class: i32,
            information: *const c_void,
            information_length: u32,
        ) -> i32;
        fn TerminateJobObject(job: Handle, exit_code: u32) -> i32;
    }

    pub(super) struct Job {
        handle: Handle,
    }

    pub(super) fn read_activation_token() -> io::Result<u8> {
        const STD_INPUT_HANDLE: u32 = (-10_i32) as u32;

        // SAFETY: this queries the launcher's process-owned standard-input
        // handle without transferring ownership.
        let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if input.is_null() || input == (-1_isize) as Handle {
            return Err(io::Error::last_os_error());
        }
        let mut token = 0_u8;
        let mut bytes_read = 0_u32;
        // SAFETY: `token` is writable for exactly one byte, `bytes_read` is a
        // valid out pointer, and this synchronous call uses no OVERLAPPED state.
        let succeeded = unsafe {
            ReadFile(
                input,
                ptr::from_mut(&mut token).cast(),
                1,
                ptr::from_mut(&mut bytes_read),
                ptr::null_mut(),
            )
        };
        if succeeded == 0 {
            return Err(io::Error::last_os_error());
        }
        if bytes_read != 1 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "worker launcher activation stream closed",
            ));
        }
        Ok(token)
    }

    impl Job {
        pub(super) fn attach(child: &Child) -> io::Result<Self> {
            // SAFETY: null attributes and name request a private job object with
            // default security. The returned owned handle is closed in Drop.
            let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let job = Self { handle };
            let mut limits = ExtendedLimitInformation::default();
            limits.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let limits_size = u32::try_from(size_of::<ExtendedLimitInformation>())
                .map_err(|_| io::Error::other("Windows job limit structure is unexpectedly large"))?;
            // SAFETY: the job is live, `limits` has the documented class-9 ABI,
            // and the exact structure size is supplied.
            let configured = unsafe {
                SetInformationJobObject(
                    job.handle,
                    JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                    ptr::from_ref(&limits).cast(),
                    limits_size,
                )
            };
            if configured == 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: std's Child owns a live process handle with CreateProcess's
            // process-control rights. Descendants inherit this job by default.
            let assigned = unsafe { AssignProcessToJobObject(job.handle, child.as_raw_handle() as Handle) };
            if assigned == 0 {
                let source = io::Error::last_os_error();
                return Err(io::Error::new(
                    source.kind(),
                    format!("host must permit nested Windows Job Objects: {source}"),
                ));
            }
            Ok(job)
        }

        pub(super) fn terminate(&self) {
            // SAFETY: `self.handle` remains owned and live until this guard drops.
            let _ = unsafe { TerminateJobObject(self.handle, 1) };
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: this is the unique owned job handle. Kill-on-close ensures
            // that any process still in the job cannot outlive the guard.
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }
}

#[cfg(windows)]
pub(super) struct ProcessTree {
    job: windows_job::Job,
}

#[cfg(windows)]
impl ProcessTree {
    pub(super) fn configure(_command: &mut Command) {}

    pub(super) fn attach(child: &Child) -> io::Result<Self> {
        windows_job::Job::attach(child).map(|job| Self { job })
    }

    pub(super) fn terminate(&self) {
        self.job.terminate();
    }
}

#[cfg(unix)]
pub(super) struct ProcessTree {
    process_group: i32,
}

#[cfg(unix)]
impl ProcessTree {
    pub(super) fn configure(command: &mut Command) {
        use std::os::unix::process::CommandExt;

        command.process_group(0);
    }

    pub(super) fn attach(child: &Child) -> io::Result<Self> {
        let process_group =
            i32::try_from(child.id()).map_err(|_| io::Error::other("worker process id exceeds the Unix pid range"))?;
        Ok(Self { process_group })
    }

    pub(super) fn terminate(&self) {
        const SIGKILL: i32 = 9;

        unsafe extern "C" {
            fn kill(process_group: i32, signal: i32) -> i32;
        }

        // SAFETY: the worker was placed in a new process group whose id is the
        // child's positive pid. A negative id addresses that entire group.
        let _ = unsafe { kill(-self.process_group, SIGKILL) };
    }
}

#[cfg(not(any(windows, unix)))]
pub(super) struct ProcessTree;

#[cfg(not(any(windows, unix)))]
impl ProcessTree {
    pub(super) fn configure(_command: &mut Command) {}

    pub(super) fn attach(_child: &Child) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "benchmark worker containment is supported only on Windows and Unix",
        ))
    }

    pub(super) fn terminate(&self) {}
}

#[cfg(all(test, windows))]
mod tests {
    use std::process::{Command, Stdio};

    use super::*;

    /// The launcher's stdout filter must pass protocol frames verbatim and
    /// demote legacy solver diagnostics, so the referee's strict reader never
    /// sees an unprefixed line from a pinned legacy artifact.
    #[test]
    fn launcher_forwarding_admits_only_prefixed_protocol_frames() {
        let prefix = RESPONSE_PREFIX;
        let script = format!(
            "[Console]::Out.WriteLine('chatty diagnostic'); \
             [Console]::Out.WriteLine(''); \
             [Console]::Out.WriteLine('{prefix}{{\"ok\":true}}'); \
             [Console]::Out.WriteLine('xxxxx');"
        );
        let mut child = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("mock stdout-polluting worker must start");
        let stdout = child.stdout.take().expect("mock worker stdout must be piped");
        let status = child.wait().expect("mock worker must finish");
        assert!(status.success());

        let mut forwarded = Vec::new();
        let mut diagnostics = Vec::new();
        forward_protocol_frames(stdout, &mut forwarded, &mut diagnostics);

        assert_eq!(
            String::from_utf8(forwarded).unwrap(),
            format!("{prefix}{{\"ok\":true}}\n")
        );
        let diagnostics = String::from_utf8(diagnostics).unwrap();
        assert!(diagnostics.contains("chatty diagnostic"));
        assert!(!diagnostics.contains(prefix));
    }
}
