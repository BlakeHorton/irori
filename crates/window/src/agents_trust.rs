//! Trust and start (card #449, docs/design/spawned-agents.md §7 "The
//! pre-trust flow", §14 part 5).
//!
//! When a card's agent can't start because its repo isn't trusted for
//! agents on this device, the spawner fails the request with
//! `not trusted: <folder> on <device>` (card #446). The window shows that
//! card in Needs you with what trusting does; the person's Trust and start
//! records trust for the repo (its main root, so a worktree is covered,
//! card #445) and starts the agent again.
//!
//! This is the only path that writes trust, and it's a window command: no
//! tool, row or agent reaches it (`tests/trust_writers.rs`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Window;
use crate::claude_trust;

/// The reason prefix of a not-trusted spawn failure
/// (`SpawnRefusal::NotTrusted`).
pub const NOT_TRUSTED: &str = "not trusted: ";

/// What the person sees before they confirm.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrustNeed {
    pub task_id: String,
    /// The folder trust is recorded for: the repo's main root.
    pub folder: String,
    /// The device the agent runs on.
    pub device: String,
    /// Whether that device is this one: otherwise it's trusted on that
    /// device's own Irori.
    pub here: bool,
    /// What trusting does, in the words the confirm shows.
    pub what: String,
}

/// After Trust and start.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrustStarted {
    pub folder: String,
    /// It was already trusted (another window got there first).
    pub already: bool,
}

/// A board's agent permissions as Settings › Agents edits them (card #449,
/// decision 8): the choice, extra rules, and how each repo reaches main.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentPermissions {
    /// `""` for the private board.
    pub board_id: String,
    /// `limited`, `edits` or `bypass`; `""` is bypass, still refused on a
    /// shared board and when the brief has another person's text.
    pub permission: String,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
    /// Repo key → `pr` or `main`.
    pub merge: std::collections::BTreeMap<String, String>,
}

/// The words the confirm shows (§7 point 2).
pub fn what_trust_does(folder: &str, device: &str) -> String {
    format!(
        "Claude Code will run agents in {folder} on {device} without asking you first. \
         It's the trust Claude Code asks for the first time you open a folder."
    )
}

/// The folder to trust for a spawn folder: the main repo's root when it's
/// in git (a worktree follows its repo), else the folder itself.
pub fn trust_folder(folder: &str) -> String {
    let p = Path::new(folder);
    crate::spawn::trust_root(p)
        .map(|r| r.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder.to_string())
}

/// The folder a not-trusted failure names: the row's folder when it has
/// one, else the one in `not trusted: <folder> on <device>`
/// (`SpawnRefusal::NotTrusted`). `None` for any other failure.
pub fn refused_folder(reason: &str, device: &str, folder: &str) -> Option<String> {
    let rest = reason.strip_prefix(NOT_TRUSTED)?;
    if !folder.is_empty() {
        return Some(folder.to_string());
    }
    let named = rest.strip_suffix(&format!(" on {device}")).unwrap_or(rest);
    (!named.is_empty()).then(|| named.to_string())
}

impl Window {
    /// The card's trust need, when its agent couldn't start for lack of
    /// trust; `None` otherwise.
    pub fn agents_trust_need(&self, task_id: &str) -> Result<Option<TrustNeed>, String> {
        let co = self.core.coord();
        let id = co.task_ref(task_id).unwrap_or_else(|_| task_id.to_string());
        let t = co
            .get_task(&id)
            .ok_or_else(|| format!("no card {task_id}"))?;
        let Some(sp) = t.spawn_request() else {
            return Ok(None);
        };
        if sp.op != coord::SPAWN_FAILED {
            return Ok(None);
        }
        let Some(spawn_folder) = refused_folder(&sp.reason, &sp.device, &sp.folder) else {
            return Ok(None);
        };
        let folder = trust_folder(&spawn_folder);
        let here = sp.device == self.core.device_id();
        let device = if here {
            "this computer".to_string()
        } else {
            match self.core.device_label(&sp.device) {
                l if l.is_empty() => sp.device.chars().take(8).collect(),
                l => l,
            }
        };
        Ok(Some(TrustNeed {
            task_id: t.id.clone(),
            what: what_trust_does(&folder, &device),
            folder,
            device,
            here,
        }))
    }

    /// A board's agent permissions.
    pub fn agents_permissions(&self, board_id: &str) -> AgentPermissions {
        let s = self.core.spawn_setting(board_id);
        AgentPermissions {
            board_id: board_id.to_string(),
            permission: s.permission,
            allow: s.allow,
            deny: s.deny,
            merge: s.merge,
        }
    }

    /// The person sets a board's agent permissions (the app only, never a
    /// tool); the board's switch and other spawn settings stay.
    pub fn agents_permissions_set(&self, p: AgentPermissions) -> Result<AgentPermissions, String> {
        self.core
            .set_spawn_permissions(&p.board_id, &p.permission, p.allow, p.deny, p.merge)?;
        Ok(self.agents_permissions(&p.board_id))
    }

    /// The person's Trust and start: records trust for the card's repo on
    /// this device, then starts its agent again.
    pub fn agents_trust_and_start(&self, task_id: &str) -> Result<TrustStarted, String> {
        let file = claude_trust::config_path()
            .ok_or("can't find Claude Code's settings (.claude.json): no home folder")?;
        self.trust_and_start_in(&file, task_id)
    }

    /// [`Window::agents_trust_and_start`] with `.claude.json` at `file`
    /// (tests point it at a scratch file).
    #[doc(hidden)]
    pub fn trust_and_start_in(&self, file: &Path, task_id: &str) -> Result<TrustStarted, String> {
        let need = self
            .agents_trust_need(task_id)?
            .ok_or("this card's agent isn't waiting on trust")?;
        if !need.here {
            return Err(format!(
                "its agent runs on {}: trust it in Irori on that computer",
                need.device
            ));
        }
        let r = claude_trust::record_trust(file, &need.folder).map_err(|e| e.to_string())?;
        if !claude_trust::is_trusted(file, &need.folder) {
            return Err(format!("{} still reads as not trusted", need.folder));
        }
        self.core.start_agent(&need.task_id, "")?;
        Ok(TrustStarted {
            folder: need.folder,
            already: r == claude_trust::Recorded::Already,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refused_folder_comes_from_the_row_or_its_reason() {
        assert_eq!(
            refused_folder("not trusted: /r/a on b/x on dev1", "dev1", ""),
            Some("/r/a on b/x".into()),
            "a folder with ' on ' in it"
        );
        assert_eq!(
            refused_folder("not trusted: /r on dev1", "dev1", "/kept"),
            Some("/kept".into())
        );
        assert_eq!(
            refused_folder("no checkout of r on dev1", "dev1", "/r"),
            None
        );
        assert_eq!(refused_folder("not trusted:  on dev1", "dev1", ""), None);
    }
}
