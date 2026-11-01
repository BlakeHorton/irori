//! Branch tips read straight from a repo's ref files (card #87's branch
//! waits): no git process, so checking a few waits every sweep is a file
//! read each. A worktree's refs live in the repo's common dir (the
//! `commondir` file in its git dir); a ref is a loose file under it or a
//! line in `packed-refs`.

use std::path::{Path, PathBuf};

use super::reflog::git_dir;

/// The worktree root holding `dir`: the nearest ancestor (or itself) with
/// a `.git` entry.
pub fn worktree_root(dir: &str) -> Option<String> {
    let mut p = Path::new(dir);
    loop {
        if p.join(".git").exists() {
            return Some(p.to_string_lossy().into_owned());
        }
        p = p.parent()?;
    }
}

/// The repo's common git dir for worktree `root` (where refs live).
pub fn common_dir(root: &str) -> Option<PathBuf> {
    let gd = PathBuf::from(git_dir(root).ok()?);
    match std::fs::read_to_string(gd.join("commondir")) {
        Ok(s) => {
            let c = PathBuf::from(s.trim());
            Some(if c.is_absolute() { c } else { gd.join(c) })
        }
        Err(_) => Some(gd),
    }
}

fn is_sha(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The sha `name` (e.g. "refs/heads/main") points at in `common`: the
/// loose ref first, then packed-refs. `None` when it doesn't exist.
pub fn read_ref(common: &Path, name: &str) -> Option<String> {
    if name.contains("..") {
        return None;
    }
    if let Ok(s) = std::fs::read_to_string(common.join(name)) {
        let s = s.trim();
        if is_sha(s) {
            return Some(s.to_lowercase());
        }
    }
    let packed = std::fs::read_to_string(common.join("packed-refs")).ok()?;
    for line in packed.lines() {
        if line.starts_with('#') || line.starts_with('^') {
            continue;
        }
        if let Some((sha, r)) = line.split_once(' ')
            && r.trim() == name
            && is_sha(sha)
        {
            return Some(sha.to_lowercase());
        }
    }
    None
}

/// The refs a branch wait watches: the local branch and origin's copy.
pub fn branch_ref_names(branch: &str) -> [String; 2] {
    [
        format!("refs/heads/{branch}"),
        format!("refs/remotes/origin/{branch}"),
    ]
}

/// Each watched ref of `branch` in worktree `root` as "<ref>=<sha>" (an
/// empty sha when the ref doesn't exist). `None` when root isn't a repo.
pub fn branch_tips(root: &str, branch: &str) -> Option<Vec<String>> {
    let common = common_dir(root)?;
    Some(
        branch_ref_names(branch)
            .iter()
            .map(|n| format!("{n}={}", read_ref(&common, n).unwrap_or_default()))
            .collect(),
    )
}

/// Where a session's work is in git (card #137), read from the ref files
/// like the rest of this module: no git process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitPlace {
    /// The worktree root.
    pub worktree: String,
    /// The checked-out branch; empty when HEAD is detached.
    pub branch: String,
    /// HEAD's sha; empty on a branch with no commit yet.
    pub head: String,
    /// origin's copy of the branch is at HEAD.
    pub pushed: bool,
}

/// The git place of directory `dir`; `None` outside a repo.
pub fn place(dir: &str) -> Option<GitPlace> {
    let root = worktree_root(dir)?;
    let gd = PathBuf::from(git_dir(&root).ok()?);
    let common = common_dir(&root)?;
    let head = std::fs::read_to_string(gd.join("HEAD")).ok()?;
    let head = head.trim();
    let mut p = GitPlace {
        worktree: root,
        ..GitPlace::default()
    };
    if let Some(r) = head.strip_prefix("ref: ") {
        let r = r.trim();
        p.branch = r.strip_prefix("refs/heads/").unwrap_or(r).to_string();
        // A worktree's own refs (none for heads) sit in its git dir; heads
        // are in the common dir.
        p.head = read_ref(&common, r)
            .or_else(|| read_ref(&gd, r))
            .unwrap_or_default();
    } else if is_sha(head) {
        p.head = head.to_lowercase();
    }
    if !p.branch.is_empty() && !p.head.is_empty() {
        p.pushed = read_ref(&common, &format!("refs/remotes/origin/{}", p.branch))
            .is_some_and(|o| o == p.head);
    }
    Some(p)
}

/// The first ref whose sha differs between two "<ref>=<sha>" lists, as
/// (ref, old, new).
pub fn moved_ref(before: &[String], now: &[String]) -> Option<(String, String, String)> {
    for b in before {
        let (name, old) = b.split_once('=').unwrap_or((b, ""));
        let new = now
            .iter()
            .find_map(|n| {
                n.split_once('=')
                    .filter(|(k, _)| *k == name)
                    .map(|(_, v)| v)
            })
            .unwrap_or("");
        if old != new && !new.is_empty() {
            return Some((name.into(), old.into(), new.into()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "1111111111111111111111111111111111111111";
    const B: &str = "2222222222222222222222222222222222222222";

    #[test]
    fn loose_packed_and_worktree() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("app");
        let git = root.join(".git");
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        std::fs::write(git.join("refs/heads/main"), format!("{A}\n")).unwrap();
        std::fs::write(
            git.join("packed-refs"),
            format!("# pack-refs with: peeled\n{B} refs/remotes/origin/main\n^{A}\n"),
        )
        .unwrap();
        let r = root.to_string_lossy();
        assert_eq!(
            branch_tips(&r, "main").unwrap(),
            vec![
                format!("refs/heads/main={A}"),
                format!("refs/remotes/origin/main={B}")
            ]
        );
        assert_eq!(
            branch_tips(&r, "nope").unwrap(),
            vec!["refs/heads/nope=", "refs/remotes/origin/nope="]
        );

        // A linked worktree: .git is a file naming its git dir, whose
        // commondir points back at the main repo's.
        let wt = d.path().join("wt");
        let wgit = git.join("worktrees/wt");
        std::fs::create_dir_all(&wgit).unwrap();
        std::fs::create_dir_all(wt.join("sub")).unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", wgit.display())).unwrap();
        std::fs::write(wgit.join("commondir"), "../..\n").unwrap();
        let w = worktree_root(&wt.join("sub").to_string_lossy()).unwrap();
        assert_eq!(
            branch_tips(&w, "main").unwrap()[0],
            format!("refs/heads/main={A}")
        );

        let before = branch_tips(&r, "main").unwrap();
        std::fs::write(git.join("refs/heads/main"), format!("{B}\n")).unwrap();
        let now = branch_tips(&r, "main").unwrap();
        assert_eq!(
            moved_ref(&before, &now),
            Some(("refs/heads/main".into(), A.into(), B.into()))
        );
        assert_eq!(moved_ref(&now, &now), None);
    }

    #[test]
    fn place_reads_branch_head_and_pushed() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("app");
        let git = root.join(".git");
        std::fs::create_dir_all(git.join("refs/heads/feat")).unwrap();
        std::fs::create_dir_all(git.join("refs/remotes/origin/feat")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/feat/x\n").unwrap();
        std::fs::write(git.join("refs/heads/feat/x"), format!("{A}\n")).unwrap();
        let sub = root.join("src").to_string_lossy().into_owned();
        let p = place(&sub).unwrap();
        assert_eq!(
            (p.branch.as_str(), p.head.as_str(), p.pushed),
            ("feat/x", A, false)
        );
        assert_eq!(p.worktree, root.to_string_lossy());
        // origin has an older commit: still not pushed.
        std::fs::write(git.join("refs/remotes/origin/feat/x"), format!("{B}\n")).unwrap();
        assert!(!place(&sub).unwrap().pushed);
        // Pushed: origin's copy is at HEAD.
        std::fs::write(git.join("refs/remotes/origin/feat/x"), format!("{A}\n")).unwrap();
        assert!(place(&sub).unwrap().pushed);
        // Detached HEAD: a sha, no branch.
        std::fs::write(git.join("HEAD"), format!("{B}\n")).unwrap();
        let p = place(&sub).unwrap();
        assert_eq!(
            (p.branch.as_str(), p.head.as_str(), p.pushed),
            ("", B, false)
        );
        // Outside any repo.
        let other = tempfile::tempdir().unwrap();
        assert!(place(&other.path().to_string_lossy()).is_none());
    }
}
