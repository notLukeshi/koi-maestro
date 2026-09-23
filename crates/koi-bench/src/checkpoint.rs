//! Checkpoint persistence: an append-only `legs.jsonl` binding the
//! manifest hash and both artifact identities.
//!
//! Resume semantics are fail-closed:
//! - The first line is a header; a header that does not match the running
//!   manifest hash and artifact pair is a foreign checkpoint — hard error.
//! - Each subsequent complete line is one `SeedResult`. A corrupt line
//!   that is not the file's last is a hard error.
//! - A torn final line (crash mid-append) is truncated away and replay
//!   resumes from the last intact cluster.
//!
//! Appends are single `writeln!` + `flush` calls, so the only corruptible
//! line is the last.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::manifest::ArtifactIdentity;
use crate::runner::SeedResult;

/// The checkpoint format tag on the header line.
const CHECKPOINT_KIND: &str = "koi-bench-legs";
const CHECKPOINT_FORMAT: u32 = 1;

/// The binding every resumed line is trusted under: manifest + artifacts.
/// `run_secret` seeds deal derivation (the panel seed alone must not
/// reconstruct a deck a worker can predict); it is generated on a fresh
/// checkpoint and *adopted* on resume, so it is excluded from the
/// header-equality binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointHeader {
    pub kind: String,
    pub format: u32,
    pub manifest_hash_fx64: String,
    pub baseline: ArtifactIdentity,
    pub candidate: ArtifactIdentity,
    /// OS-entropy run secret mixing into every deal derivation. Persisted
    /// here so a resumed run replays identical decks; a worker that reads
    /// the checkpoint file mid-run can still recover decks — deck secrecy
    /// against a filesystem-snooping same-user worker requires OS-level
    /// sandboxing, which is out of the harness's scope.
    pub run_secret: u64,
}

impl CheckpointHeader {
    /// The fields that bind a checkpoint to this run — everything but the
    /// adopted `run_secret`.
    fn binds(&self, manifest_hash_fx64: &str, baseline: &ArtifactIdentity, candidate: &ArtifactIdentity) -> bool {
        self.kind == CHECKPOINT_KIND
            && self.format == CHECKPOINT_FORMAT
            && self.manifest_hash_fx64 == manifest_hash_fx64
            && self.baseline == *baseline
            && self.candidate == *candidate
    }
}

/// Fresh entropy for a run's deal derivations — never derived from the
/// panel or build stamps, so a worker cannot predict decks from public
/// protocol data alone.
fn generate_run_secret() -> u64 {
    rand::random::<u64>()
}

/// An open checkpoint: the header it is bound to plus the replayed
/// completed clusters in panel order (only the played subset).
pub struct Checkpoint {
    path: PathBuf,
    file: File,
    /// The adopted (or freshly generated) run secret — the runner mixes it
    /// into every `deal_from_seed` call.
    run_secret: u64,
    /// Clusters replayed from disk at open. Live appends are durably on
    /// disk but not re-retained here — the runner holds them in its own
    /// results vector, so evidence is never doubled in memory.
    completed: Vec<SeedResult>,
}

impl Checkpoint {
    /// Opens or creates `path` bound to this run's manifest hash, artifact
    /// pair, and `panel` (the manifest's declared seed order).
    ///
    /// - Missing file: created; a fresh `run_secret` is generated into the
    ///   header line.
    /// - Present file: the stored header must bind this manifest and
    ///   artifact pair (the stored `run_secret` is *adopted* so resumed
    ///   clusters replay identical decks); complete lines replay into
    ///   `completed`, a torn tail line is truncated.
    /// - Replayed entries must form a strict prefix of `panel` in declared
    ///   order — an out-of-panel, duplicated, or reordered seed means the
    ///   checkpoint is foreign or tampered and fails closed.
    /// - Any final line lacking a newline terminator is torn (a crash
    ///   between writes) and truncated — even when its JSON would parse;
    ///   appending after it would merge the next entry into it and brick
    ///   the file.
    ///
    /// Threat model: the checkpoint defends against crashes, torn writes,
    /// and resume-mismatch. Entries are not MAC'd — a writer with
    /// filesystem access to `path` can still forge entries (and read
    /// `run_secret`). Canonical evidence treats the output directory as
    /// trusted for the run's duration; hostile same-user workers are
    /// outside the harness's containment scope.
    pub fn open(
        path: &Path,
        manifest_hash_fx64: &str,
        baseline: &ArtifactIdentity,
        candidate: &ArtifactIdentity,
        panel: &[u64],
    ) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create checkpoint directory {}", parent.display()))?;
        }

        if !path.exists() {
            let run_secret = generate_run_secret();
            let header = CheckpointHeader {
                kind: CHECKPOINT_KIND.to_owned(),
                format: CHECKPOINT_FORMAT,
                manifest_hash_fx64: manifest_hash_fx64.to_owned(),
                baseline: baseline.clone(),
                candidate: candidate.clone(),
                run_secret,
            };
            // Header + terminator go out in one write so a crash can never
            // leave a newline-less header behind.
            let mut header_line = serde_json::to_string(&header).context("checkpoint header serialize")?;
            header_line.push('\n');
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .with_context(|| format!("failed to create checkpoint {}", path.display()))?;
            file.write_all(header_line.as_bytes())
                .and_then(|()| file.flush())
                .context("failed to write checkpoint header")?;
            let file = OpenOptions::new()
                .append(true)
                .open(path)
                .context("failed to reopen checkpoint for append")?;
            return Ok(Self {
                path: path.to_path_buf(),
                file,
                run_secret,
                completed: Vec::new(),
            });
        }

        // Streamed replay — a checkpoint is large enough that whole-file
        // reads are a memory spike; byte offsets are tracked so a torn
        // tail can be truncated by length.
        let file = File::open(path).with_context(|| format!("failed to open checkpoint {}", path.display()))?;
        let mut reader = BufReader::new(file);
        let mut line = Vec::new();
        let header_len = reader
            .read_until(b'\n', &mut line)
            .context("failed to read checkpoint header")?;
        if header_len == 0 {
            bail!("checkpoint {} is empty — refusing to trust it", path.display());
        }
        let stored: CheckpointHeader = serde_json::from_slice(line.trim_ascii_end())
            .with_context(|| format!("checkpoint {} has a corrupt header", path.display()))?;
        if !stored.binds(manifest_hash_fx64, baseline, candidate) {
            bail!(
                "checkpoint {} belongs to a different run (manifest or artifact binding mismatch) — \
                 refusing to resume",
                path.display()
            );
        }

        // Remaining lines: each complete line must be the next declared
        // panel seed; an unterminated tail is torn and truncated.
        let mut completed = Vec::new();
        let mut offset = header_len;
        let mut valid_len = offset;
        loop {
            line.clear();
            let read = reader
                .read_until(b'\n', &mut line)
                .context("failed to read checkpoint line")?;
            if read == 0 || line.last() != Some(&b'\n') {
                break; // clean EOF, or torn tail — truncate at this line's start
            }
            let result: SeedResult = serde_json::from_slice(line.trim_ascii_end()).with_context(|| {
                format!(
                    "checkpoint {} has a corrupt complete line at byte {offset}",
                    path.display()
                )
            })?;
            let position = completed.len();
            match panel.get(position) {
                Some(&expected) if expected == result.seed => {}
                Some(&expected) => bail!(
                    "checkpoint {} replays seed {} where the panel expects {} at \
                     position {position} — foreign or tampered evidence",
                    path.display(),
                    result.seed,
                    expected
                ),
                None => bail!(
                    "checkpoint {} holds more clusters than the panel declares (extra seed {})",
                    path.display(),
                    result.seed
                ),
            }
            completed.push(result);
            offset += read;
            valid_len = offset;
        }
        let file_len = std::fs::metadata(path)
            .with_context(|| format!("failed to stat checkpoint {}", path.display()))?
            .len() as usize;
        if valid_len < file_len {
            let file = OpenOptions::new()
                .write(true)
                .open(path)
                .context("failed to open checkpoint for truncation")?;
            file.set_len(valid_len as u64)
                .context("failed to truncate torn checkpoint tail")?;
        }

        let file = OpenOptions::new()
            .append(true)
            .open(path)
            .context("failed to reopen checkpoint for append")?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            run_secret: stored.run_secret,
            completed,
        })
    }

    /// The run's deal-derivation secret — generated on a fresh checkpoint,
    /// adopted from the stored header on resume.
    pub fn run_secret(&self) -> u64 {
        self.run_secret
    }

    /// The clusters replayed from disk at open, in panel order. Entries
    /// appended live are on disk but not re-retained here.
    pub fn completed(&self) -> &[SeedResult] {
        &self.completed
    }

    /// Seeds whose clusters were already recorded when the file opened.
    pub fn completed_seeds(&self) -> std::collections::BTreeSet<u64> {
        self.completed.iter().map(|result| result.seed).collect()
    }

    /// Appends one completed cluster: the serialized line and its
    /// terminator go out in a single `write_all`, so a crash mid-append
    /// leaves at most a torn tail (truncated on the next open), never a
    /// merged line. `flush` pushes the userspace buffer to the OS; the
    /// crash contract deliberately does not pay `fsync` per cluster.
    pub fn append(&mut self, result: &SeedResult) -> Result<()> {
        let mut line = serde_json::to_string(result).context("checkpoint entry serialize")?;
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .and_then(|()| self.file.flush())
            .with_context(|| format!("failed to append to {}", self.path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ArtifactIdentity;
    use crate::runner::{LegResult, LegStatus, Seat};
    use std::path::PathBuf;

    fn identity(label: &str) -> ArtifactIdentity {
        ArtifactIdentity {
            label: label.to_owned(),
            executable: PathBuf::from("bin"),
            build_commit: "c".to_owned(),
            build_tree: "t".to_owned(),
            build_dirty: false,
            binary_hash_fnv1a64: "h".to_owned(),
            config_hash_fx64: "cfg".to_owned(),
        }
    }

    const HASH: &str = "manifest-hash";

    fn open(path: &std::path::Path, panel: &[u64]) -> Result<Checkpoint> {
        Checkpoint::open(path, HASH, &identity("a"), &identity("b"), panel)
    }

    fn leg() -> LegResult {
        LegResult {
            candidate_seat: Seat::South,
            status: LegStatus::Valid,
            south_points: Some(1),
            north_points: Some(0),
            candidate_margin: 1.0,
            candidate_win_score: 1.0,
            actions: 3,
            redeals: 0,
            trace: Vec::new(),
        }
    }

    fn seed(seed: u64) -> SeedResult {
        SeedResult {
            seed,
            candidate_deals: leg(),
            baseline_deals: leg(),
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koi-bench-checkpoint-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("legs.jsonl")
    }

    #[test]
    fn round_trip_and_resume() {
        let path = temp_path("round-trip");
        let first_secret;
        {
            let mut checkpoint = open(&path, &[11, 22, 33]).unwrap();
            first_secret = checkpoint.run_secret();
            checkpoint.append(&seed(11)).unwrap();
            checkpoint.append(&seed(22)).unwrap();
        }
        let checkpoint = open(&path, &[11, 22, 33]).unwrap();
        assert_eq!(checkpoint.completed().len(), 2);
        assert!(checkpoint.completed_seeds().contains(&22));
        // The stored secret is adopted on resume so decks replay identically.
        assert_eq!(checkpoint.run_secret(), first_secret);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn foreign_manifest_fails_closed() {
        let path = temp_path("foreign");
        {
            let mut checkpoint = open(&path, &[1]).unwrap();
            checkpoint.append(&seed(1)).unwrap();
        }
        assert!(Checkpoint::open(&path, "other", &identity("a"), &identity("b"), &[1]).is_err());
        assert!(Checkpoint::open(&path, HASH, &identity("z"), &identity("b"), &[1]).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn off_panel_duplicate_and_reordered_seeds_fail_closed() {
        // Duplicate of a declared seed.
        let path = temp_path("dup");
        {
            let mut checkpoint = open(&path, &[1, 2]).unwrap();
            checkpoint.append(&seed(1)).unwrap();
            checkpoint.append(&seed(1)).unwrap();
        }
        assert!(open(&path, &[1, 2]).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        // Foreign seed.
        let path = temp_path("off-panel");
        {
            let mut checkpoint = open(&path, &[1]).unwrap();
            checkpoint.append(&seed(9)).unwrap();
        }
        assert!(open(&path, &[1]).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        // Declared seeds in the wrong order.
        let path = temp_path("reordered");
        {
            let mut checkpoint = open(&path, &[2, 1]).unwrap();
            checkpoint.append(&seed(1)).unwrap();
        }
        assert!(open(&path, &[2, 1]).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn torn_tail_is_dropped_but_corrupt_middle_fails() {
        let path = temp_path("torn");
        {
            let mut checkpoint = open(&path, &[1, 2, 3]).unwrap();
            checkpoint.append(&seed(1)).unwrap();
            checkpoint.append(&seed(2)).unwrap();
        }
        // Tear the tail mid-line.
        let content = std::fs::read(&path).unwrap();
        let cut = content.len() - 10;
        std::fs::write(&path, &content[..cut]).unwrap();
        let checkpoint = open(&path, &[1, 2, 3]).unwrap();
        assert_eq!(checkpoint.completed().len(), 1);

        // A parseable-but-unterminated tail is still torn: a crash between
        // the payload and its newline must not count as evidence.
        let path = temp_path("parseable-torn");
        {
            let mut checkpoint = open(&path, &[1, 2]).unwrap();
            checkpoint.append(&seed(1)).unwrap();
            checkpoint.append(&seed(2)).unwrap();
        }
        let content = std::fs::read(&path).unwrap();
        let last_newline = content.iter().rposition(|byte| *byte == b'\n').unwrap();
        std::fs::write(&path, &content[..last_newline]).unwrap(); // drop the '\n' itself
        let checkpoint = open(&path, &[1, 2]).unwrap();
        assert_eq!(
            checkpoint.completed().len(),
            1,
            "unterminated tail must be truncated even when parseable"
        );

        // Corrupt a middle line.
        {
            let mut checkpoint = open(&path, &[1, 2, 3]).unwrap();
            checkpoint.append(&seed(2)).unwrap();
        }
        let content = std::fs::read(&path).unwrap();
        let text = String::from_utf8(content).unwrap();
        let mut lines: Vec<&str> = text.lines().collect();
        lines[1] = "{corrupt";
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        assert!(open(&path, &[1, 2, 3]).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
