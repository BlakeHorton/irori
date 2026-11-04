//! Card #427: a settled card's build folder doesn't stay behind. Each
//! agent worktree builds into its own `target/` (the isolation card #90
//! needed), 20-90 GB each, and some 45 of them filled the disk on
//! 2026-11-03. So once a card is settled (the person said Done, or it was
//! canceled or failed), this device clears the `target/` of the worktree
//! its holder worked in, when nothing still uses it:
//!
//! - the worktree is this device's (the card's `where` is a local session's)
//!   and a linked git worktree (`.git` is a file): never a main checkout;
//! - `target/` is cargo's (it holds `CACHEDIR.TAG`);
//! - no card that isn't settled says it works there (a completed card
//!   still waiting on the person's Done keeps its build: it may come back);
//! - nothing in `target/` changed for [`IDLE`], and no cargo holds its lock.
//!
//! `target/` is renamed aside first (one step, so a build never sees it
//! half gone), then deleted; a leftover from a stop midway is deleted on
//! the next pass.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use coord::{Task, is_terminal};

/// How often the sweep looks for a card that has just settled.
pub const LOOK: Duration = Duration::from_secs(30);
/// How often it looks again at worktrees it kept.
pub const EVERY: Duration = Duration::from_secs(10 * 60);
/// How long a `target/` must be untouched before it goes.
pub const IDLE: Duration = Duration::from_secs(60 * 60);
/// The name a `target/` is moved to while it is deleted.
pub const CLEARING: &str = "target.irori-clearing";

/// When the sweep last listed the cards, when it last cleared, and the
/// worktrees it cleared then.
#[derive(Debug, Default)]
pub struct Looked {
    pub listed: Option<std::time::Instant>,
    pub cleared: Option<std::time::Instant>,
    pub seen: Vec<String>,
}

/// Whether a card is settled: closed, and not waiting on the person's Done.
pub fn settled(t: &Task) -> bool {
    is_terminal(&t.state) && !t.blockers.iter().any(|b| b.open() && b.done_check)
}

/// The worktrees on `device` whose builds may go: those settled cards'
/// holders worked in, less any a card that isn't settled works in.
pub fn settled_worktrees<'a>(
    tasks: impl IntoIterator<Item = &'a Task>,
    device: &str,
) -> Vec<String> {
    let mine = format!("{device}/");
    let (mut done, mut busy) = (BTreeSet::new(), BTreeSet::new());
    for t in tasks {
        let Some(w) = t.where_.as_ref().filter(|w| !w.worktree.is_empty()) else {
            continue;
        };
        if settled(t) {
            if w.who.starts_with(&mine) {
                done.insert(w.worktree.clone());
            }
        } else {
            busy.insert(w.worktree.clone());
        }
    }
    done.difference(&busy).cloned().collect()
}

/// What [`clear`] did with one worktree.
#[derive(Debug, PartialEq, Eq)]
pub enum Cleared {
    /// Its `target/` is gone.
    Gone,
    /// Nothing to clear, or not ours to: why.
    Kept(&'static str),
}

/// Clears `worktree`'s `target/` if every rule in the module doc holds.
pub fn clear(worktree: &Path, idle: Duration, now: SystemTime) -> Result<Cleared, String> {
    let aside = worktree.join(CLEARING);
    if aside.exists() {
        fs::remove_dir_all(&aside).map_err(|e| format!("{}: {e}", aside.display()))?;
    }
    if !worktree.join(".git").is_file() {
        return Ok(Cleared::Kept("not a linked worktree"));
    }
    let target = worktree.join("target");
    if !target.join("CACHEDIR.TAG").is_file() {
        return Ok(Cleared::Kept("no cargo target"));
    }
    if newest(&target, 3).is_some_and(|m| now.duration_since(m).unwrap_or_default() < idle) {
        return Ok(Cleared::Kept("built recently"));
    }
    // Hold every cargo lock in it while it moves, so no build starts midway.
    let mut locks = Vec::new();
    for p in cargo_locks(&target, 3) {
        let Ok(f) = File::options().read(true).write(true).open(&p) else {
            continue;
        };
        if f.try_lock().is_err() {
            return Ok(Cleared::Kept("a build holds its lock"));
        }
        locks.push(f);
    }
    fs::rename(&target, &aside).map_err(|e| format!("{}: {e}", target.display()))?;
    drop(locks);
    fs::remove_dir_all(&aside).map_err(|e| format!("{}: {e}", aside.display()))?;
    Ok(Cleared::Gone)
}

/// The latest change under `dir` (itself and its directories, `depth`
/// levels down): a build adds and removes entries in these.
fn newest(dir: &Path, depth: u32) -> Option<SystemTime> {
    let mut m = fs::metadata(dir).and_then(|m| m.modified()).ok();
    if depth == 0 {
        return m;
    }
    for e in fs::read_dir(dir).ok()?.flatten() {
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            m = m.max(newest(&e.path(), depth - 1));
        }
    }
    m
}

/// Cargo's `.cargo-lock` files under `dir`, `depth` levels down
/// (`target/debug`, `target/<triple>/debug`, ...).
fn cargo_locks(dir: &Path, depth: u32) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if e.file_name() == ".cargo-lock" {
            out.push(p);
        } else if depth > 1 && e.file_type().is_ok_and(|t| t.is_dir()) {
            out.extend(cargo_locks(&p, depth - 1));
        }
    }
    out
}
