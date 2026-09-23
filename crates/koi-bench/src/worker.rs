//! Worker process client: spawn, negotiate, and transact decisions under
//! containment and a per-transaction liveness bound.
//!
//! A worker is a `koi-bench worker` subprocess speaking newline-delimited
//! JSON: the referee writes a `DecisionRequest` to stdin, the worker
//! answers one `@@KOI_BENCH@@`-prefixed `DecisionResponse` on stdout.
//! Reads run on a dedicated thread into a channel so `decide` can enforce
//! `worker_timeout_seconds` — a hung worker is killed through its process
//! tree (Job Object on Windows, process group on Unix), never awaited.

use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::artifact::{spawn_arguments, ResolvedArtifact};
use crate::framing::read_bounded_frame;
use crate::process_tree::{activate_worker, contained_worker_command, ProcessTree};
use crate::protocol::{
    validate_response, DecisionRequest, DecisionResponse, NegotiationRequest, NegotiationResponse, WorkerReply,
    RESPONSE_PREFIX,
};

/// How a worker transaction ended. `Timeout`/`Crashed`/`BadFrame` are
/// worker-side failures (the leg forfeits); `Io` failures on the referee's
/// own handles are still worker failures — the channel is the worker's.
pub enum DecideError {
    /// The worker exceeded the transaction timeout; the process was killed.
    Timeout,
    /// The worker's stdout closed or the process exited mid-transaction.
    Crashed(String),
    /// The worker produced a frame that is not a valid response.
    BadFrame(String),
    /// The worker answered, but the response failed validation or carried
    /// an error payload.
    Protocol(String),
}

impl std::fmt::Display for DecideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "worker exceeded the transaction timeout"),
            Self::Crashed(detail) => write!(f, "worker crashed: {detail}"),
            Self::BadFrame(detail) => write!(f, "worker produced an invalid frame: {detail}"),
            Self::Protocol(detail) => write!(f, "worker protocol violation: {detail}"),
        }
    }
}

/// One write handed to the stdin writer thread; the ack carries the write
/// result back so `send` can time-bounds it.
type WriteJob = (Vec<u8>, SyncSender<std::io::Result<()>>);

/// A running worker process with its stdout reader and stdin writer
/// threads. Both channel endpoints let `decide` bound the whole
/// transaction — a worker that stops draining stdin can no longer stall
/// the referee in `write_all`.
pub struct WorkerClient {
    child: Child,
    writer: SyncSender<WriteJob>,
    frames: Receiver<std::io::Result<String>>,
    _reader: JoinHandle<()>,
    _writer: JoinHandle<()>,
    tree: ProcessTree,
    timeout: Duration,
    /// What the worker attested at negotiation.
    pub negotiated: NegotiationResponse,
}

/// Frames emitted by the stdout reader: only `RESPONSE_PREFIX`-prefixed
/// lines are forwarded — a worker's stray stdout chatter is demoted, never
/// parsed as protocol.
fn stdout_reader(stdout: ChildStdout, sink: SyncSender<std::io::Result<String>>) {
    let mut reader = BufReader::new(stdout);
    loop {
        match read_bounded_frame(&mut reader) {
            Ok(Some(line)) => {
                if line.starts_with(RESPONSE_PREFIX) {
                    if sink.send(Ok(line)).is_err() {
                        return;
                    }
                } else if !line.trim().is_empty() {
                    log::debug!("[worker stdout] {line}");
                }
            }
            Ok(None) => return,
            Err(error) => {
                let _ = sink.send(Err(std::io::Error::new(std::io::ErrorKind::InvalidData, error)));
                return;
            }
        }
    }
}

impl WorkerClient {
    /// Spawns and negotiates a worker for `resolved`. The negotiation reply
    /// attests build identity and protocol support; the caller records it
    /// into the artifact's expected stamps.
    pub fn start(resolved: &ResolvedArtifact, timeout: Duration) -> Result<Self> {
        // The image is hashed at resolve time; re-verify before every
        // spawn so a binary swapped mid-run cannot ride the recorded
        // identity into fresh evidence.
        resolved.verify_image()?;
        let arguments = spawn_arguments(resolved);
        let mut command = contained_worker_command(&resolved.executable, &arguments)
            .map_err(|error| anyhow::anyhow!("failed to build contained worker command: {error}"))?;
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        ProcessTree::configure(&mut command);
        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn worker {}", resolved.executable.display()))?;
        let tree = ProcessTree::attach(&child).context("failed to attach worker to a process tree")?;
        let mut stdin = child.stdin.take().context("worker stdin was not piped")?;
        let stdout = child.stdout.take().context("worker stdout was not piped")?;
        activate_worker(&mut stdin).context("worker activation failed")?;

        let (sender, frames) = sync_channel(64);
        let reader = std::thread::spawn(move || stdout_reader(stdout, sender));
        let (write_jobs, job_rx) = sync_channel::<WriteJob>(1);
        let writer = std::thread::spawn(move || Self::stdin_writer(stdin, job_rx));

        let mut client = Self {
            child,
            writer: write_jobs,
            frames,
            _reader: reader,
            _writer: writer,
            tree,
            timeout,
            negotiated: NegotiationResponse::new(0),
        };
        client.negotiate()?;
        Ok(client)
    }

    /// The stdin writer's end of the pipe: writes are serialized through
    /// here so `send` waits on an ack channel it can abandon on timeout —
    /// a wedged `write_all` (worker stopped draining) no longer hangs the
    /// referee.
    fn stdin_writer(mut stdin: ChildStdin, jobs: Receiver<WriteJob>) {
        while let Ok((bytes, ack)) = jobs.recv() {
            let outcome = stdin
                .write_all(&bytes)
                .and_then(|()| stdin.write_all(b"\n"))
                .and_then(|()| stdin.flush());
            if ack.send(outcome).is_err() {
                return;
            }
        }
    }

    /// Kill the process and reap it — the tree termination covers grouped
    /// descendants, the direct `kill` covers a child that escaped its
    /// process group (e.g. via `setsid` on Unix).
    fn kill(&mut self) {
        self.tree.terminate();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Queues one framed request line on the writer thread and waits for
    /// the write to land, bounded by `budget`. A timeout retires the
    /// worker exactly like a hung read.
    fn send(&mut self, payload: &impl Serialize, budget: Duration) -> std::result::Result<(), DecideError> {
        let bytes = serde_json::to_vec(payload)
            .map_err(|error| DecideError::BadFrame(format!("request serialization failed: {error}")))?;
        let (ack_tx, ack_rx) = sync_channel(0);
        self.writer
            .send((bytes, ack_tx))
            .map_err(|_| DecideError::Crashed("stdin writer thread is dead".to_owned()))?;
        match ack_rx.recv_timeout(budget) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(DecideError::Crashed(format!("stdin write failed: {error}"))),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                self.kill();
                Err(DecideError::Timeout)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(DecideError::Crashed("stdin writer died mid-write".to_owned()))
            }
        }
    }

    /// Reads one prefixed response frame within `budget` of the
    /// transaction start — the send and the read share one deadline.
    fn recv(&mut self, budget: Duration) -> std::result::Result<String, DecideError> {
        match self.frames.recv_timeout(budget) {
            Ok(Ok(line)) => Ok(line),
            Ok(Err(error)) => Err(DecideError::BadFrame(error.to_string())),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Kill the hung worker — a killed worker is never reused.
                self.kill();
                Err(DecideError::Timeout)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                let status = self
                    .child
                    .try_wait()
                    .map(|status| format!("exited with {status:?}"))
                    .unwrap_or_else(|_| "stdout closed".to_owned());
                Err(DecideError::Crashed(status))
            }
        }
    }

    /// The pre-flight negotiation transaction.
    fn negotiate(&mut self) -> Result<()> {
        self.send(&NegotiationRequest::new(0), self.timeout)
            .map_err(|error| anyhow::anyhow!("failed to send negotiation request: {error}"))?;
        let line = self
            .recv(self.timeout)
            .map_err(|error| anyhow::anyhow!("worker negotiation failed: {error}"))?;
        let payload = line
            .strip_prefix(RESPONSE_PREFIX)
            .context("worker reply lacked the response prefix")?;
        match WorkerReply::parse(payload) {
            Ok(WorkerReply::Negotiation(response)) => {
                if response.request_id != 0 {
                    bail!("worker negotiation id mismatch: received {}", response.request_id);
                }
                if !response
                    .supported_protocols
                    .contains(&crate::protocol::WORKER_PROTOCOL_VERSION)
                {
                    bail!(
                        "worker cannot serve protocol {}: it offers {:?}",
                        crate::protocol::WORKER_PROTOCOL_VERSION,
                        response.supported_protocols
                    );
                }
                self.negotiated = response;
                Ok(())
            }
            Ok(WorkerReply::Decision(_)) => {
                bail!("worker answered a negotiation request with a decision frame")
            }
            Err(error) => bail!("worker negotiation frame invalid: {error}"),
        }
    }

    /// One decision transaction: write the request, wait for the response,
    /// validate it against the request's expected identity.
    ///
    /// Returns the response and the referee-measured latency. Any
    /// `DecideError` is the caller's signal to forfeit the decision for the
    /// offending artifact and `restart` this client before the next leg.
    pub fn decide(
        &mut self,
        request: &DecisionRequest,
        expected_commit: &str,
        expected_tree: &str,
        expected_clean: bool,
    ) -> (std::result::Result<DecisionResponse, DecideError>, f64) {
        let started = Instant::now();
        let result = (|| {
            // A frame queued before this request was sent can only be a
            // pre-computed or stray response — accepting it would defeat
            // latency accounting and scramble request/response pairing.
            if self.frames.try_recv().is_ok() {
                return Err(DecideError::BadFrame(
                    "response frame arrived before its request was sent".to_owned(),
                ));
            }
            // One deadline across send + recv: a worker cannot stall the
            // transaction on either half.
            let remaining = self.timeout.saturating_sub(started.elapsed());
            self.send(request, remaining)?;
            let remaining = self.timeout.saturating_sub(started.elapsed());
            let line = self.recv(remaining)?;
            let payload = line
                .strip_prefix(RESPONSE_PREFIX)
                .ok_or_else(|| DecideError::BadFrame("missing response prefix".to_owned()))?;
            let reply = WorkerReply::parse(payload).map_err(DecideError::BadFrame)?;
            let WorkerReply::Decision(response) = reply else {
                return Err(DecideError::BadFrame("negotiation frame in a decision slot".to_owned()));
            };
            validate_response(
                &response,
                request.request_id,
                request.protocol_version,
                expected_commit,
                expected_tree,
                expected_clean,
            )
            .map_err(|error| DecideError::Protocol(error.to_string()))?;
            if let Some(error) = &response.error {
                return Err(DecideError::Protocol(format!("worker reported: {error}")));
            }
            Ok(response)
        })();
        (result, started.elapsed().as_secs_f64() * 1000.0)
    }
}

impl Drop for WorkerClient {
    fn drop(&mut self) {
        self.kill();
    }
}

/// A lazily-started pool keyed by (executable, entrant label). Each
/// entrant owns its own worker process even when both share a binary —
/// one process serving both seats would see both private hands, a
/// cross-seat visibility channel no honest benchmark should offer.
#[derive(Default)]
pub struct WorkerPool {
    clients: std::collections::HashMap<(PathBuf, String), WorkerClient>,
}

fn pool_key(resolved: &ResolvedArtifact) -> (PathBuf, String) {
    (resolved.executable.clone(), resolved.label.clone())
}

impl WorkerPool {
    /// The client serving `resolved`, started on first use. A start
    /// failure is a referee-side hard error — an artifact that cannot
    /// launch cannot be forfeited meaningfully.
    pub fn worker_for(&mut self, resolved: &ResolvedArtifact, timeout: Duration) -> Result<&mut WorkerClient> {
        let key = pool_key(resolved);
        if !self.clients.contains_key(&key) {
            let client = WorkerClient::start(resolved, timeout)
                .with_context(|| format!("failed to start worker for {}", resolved.label))?;
            self.clients.insert(key.clone(), client);
        }
        Ok(self.clients.get_mut(&key).expect("inserted"))
    }

    /// Starts every entrant's worker and returns each executable's
    /// negotiated build stamps — failing pre-run rather than mid-leg. The
    /// caller pins the stamps onto its artifacts via `record_negotiated`.
    pub fn preflight(
        &mut self,
        artifacts: &[&ResolvedArtifact],
        timeout: Duration,
    ) -> Result<Vec<(PathBuf, String, String, bool)>> {
        let mut negotiated = Vec::with_capacity(artifacts.len());
        for resolved in artifacts {
            let client = self.worker_for(resolved, timeout)?;
            negotiated.push((
                resolved.executable.clone(),
                client.negotiated.worker_build_commit.clone(),
                client.negotiated.worker_build_tree.clone(),
                client.negotiated.worker_build_dirty,
            ));
        }
        Ok(negotiated)
    }

    /// Retires the client for `resolved` after a worker-side failure:
    /// drops the process (the client kills its tree on drop) so the next
    /// `worker_for` lazily respawns — and re-hashes the binary — then.
    /// Deliberately not a respawn: a failed respawn must never swallow the
    /// triggering violation's forfeit.
    pub fn retire(&mut self, resolved: &ResolvedArtifact) {
        self.clients.remove(&pool_key(resolved));
    }

    /// Kills every client — used on abort paths so no worker outlives the
    /// referee.
    pub fn shutdown(&mut self) {
        self.clients.clear();
    }
}
