//! Settings › Agents (card #448, docs/design/spawned-agents.md §10, sa-6):
//! a board's spawned-agent switch and the model and effort its agents
//! start with. Set in the app only, never by a tool (decision 23). Each
//! write changes only its own fields of the board's `spawn.set` row, so
//! the permission fields part 5 keeps there are left as they are.

use irori_core::spawn::{self, SpawnSetting};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::Window;

/// One agent Irori started, or is starting, for a card (card #448): a row
/// of Settings › Agents' Running agents list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunningAgent {
    /// The agent's key; `""` while it is still starting.
    pub key: String,
    pub name: String,
    /// starting, busy, idle, unresponsive.
    pub status: String,
    pub card_id: String,
    /// `#N`.
    pub card_short: String,
    pub card_title: String,
    /// "This PC" or the device's name.
    pub device: String,
    /// It runs here, so End works from this window.
    pub here: bool,
    pub model: String,
    pub effort: String,
    /// When Irori asked for it, in ms.
    #[specta(type = f64)]
    pub started_at: i64,
}

/// What Settings › Agents shows for one board.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentsSettings {
    /// `""`: the private board.
    pub board_id: String,
    /// Irori starts an agent for each card instead of waiting for a free one.
    pub on: bool,
    /// He has set the switch himself; until then it is the default (on
    /// for the private board in the app), and Home shows the first-run
    /// notice.
    pub chosen: bool,
    /// What a spawn passes: the board's pick, or the default.
    pub model: String,
    pub effort: String,
    /// The choices Settings › Agents offers.
    pub models: Vec<String>,
    pub efforts: Vec<String>,
    pub default_model: String,
    pub default_effort: String,
}

/// The person's change: the fields left `None` keep what is stored.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentsChange {
    #[serde(default)]
    pub board_id: String,
    #[serde(default)]
    pub on: Option<bool>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
}

pub(crate) fn view(s: &SpawnSetting, chosen: bool) -> AgentsSettings {
    AgentsSettings {
        board_id: s.board_id.clone(),
        on: s.on,
        chosen,
        model: s.model().into(),
        effort: s.effort().into(),
        models: spawn::MODELS.iter().map(|m| (*m).into()).collect(),
        efforts: spawn::EFFORTS.iter().map(|e| (*e).into()).collect(),
        default_model: spawn::DEFAULT_MODEL.into(),
        default_effort: spawn::DEFAULT_EFFORT.into(),
    }
}

/// The stored row with the change applied.
pub(crate) fn apply(mut s: SpawnSetting, c: AgentsChange) -> SpawnSetting {
    if let Some(on) = c.on {
        s.on = on;
    }
    if let Some(m) = c.model {
        s.model = m;
    }
    if let Some(e) = c.effort {
        s.effort = e;
    }
    s
}

impl Window {
    /// Settings › Agents for a board (`""`: the private board).
    pub fn agents_settings(&self, board_id: &str) -> AgentsSettings {
        view(
            &self.core.spawn_setting(board_id),
            self.core.spawn_switch_set(board_id),
        )
    }

    /// The person's End on an agent Irori started (card #448, sa-2): its
    /// hand-off, then stop and rm. Only on the device it runs on until
    /// part 11 sends end requests across devices.
    pub fn agents_end(&self, key: &str) -> Result<(), String> {
        let dev = key.split_once('/').map_or("", |(d, _)| d);
        if dev != self.core.device_id() {
            return Err("End it from the device it runs on".into());
        }
        self.core.end_spawned(key)
    }

    /// Every agent Irori started that is still running, and every card an
    /// agent is starting for, newest first (card #448).
    pub fn agents_running(&self) -> Vec<RunningAgent> {
        let p = self.people();
        let co = self.core.coord();
        let me = self.core.device_id().to_string();
        let mut out = Vec::new();
        let mut cards: std::collections::HashSet<String> = Default::default();
        for a in self.core.mesh_agents() {
            if a.status == "gone" || !co.is_spawned(&a.key) {
                continue;
            }
            let Some(sp) = co.spawned(&a.key) else {
                continue;
            };
            let t = co.get_task(&sp.task_id);
            let req = t.as_ref().and_then(|t| {
                t.spawns
                    .iter()
                    .find(|s| s.key == a.key)
                    .or_else(|| t.spawn_request())
                    .cloned()
            });
            let dev = if a.device_id.is_empty() {
                sp.device.clone()
            } else {
                a.device_id.clone()
            };
            cards.insert(sp.task_id.clone());
            out.push(RunningAgent {
                key: a.key.clone(),
                name: if a.name.is_empty() {
                    a.key.clone()
                } else {
                    a.name.clone()
                },
                status: a.status.clone(),
                card_id: sp.task_id.clone(),
                card_short: t.as_ref().map(|t| t.short_ref()).unwrap_or_default(),
                card_title: t.as_ref().map(|t| t.title.clone()).unwrap_or_default(),
                device: p.where_(&dev),
                here: dev == me,
                model: req.as_ref().map(|r| r.model.clone()).unwrap_or_default(),
                effort: req.as_ref().map(|r| r.effort.clone()).unwrap_or_default(),
                started_at: sp.at.unix_milli_value(),
            });
        }
        for t in co.list_tasks(&coord::TaskFilter {
            open: true,
            ..coord::TaskFilter::default()
        }) {
            let Some(r) = t.spawn_request().filter(|r| r.pending()) else {
                continue;
            };
            if cards.contains(&t.id) {
                continue;
            }
            out.push(RunningAgent {
                status: "starting".into(),
                card_id: t.id.clone(),
                card_short: t.short_ref(),
                card_title: t.title.clone(),
                device: p.where_(&r.device),
                here: r.device == me,
                model: r.model.clone(),
                effort: r.effort.clone(),
                started_at: r.at.unix_milli_value(),
                ..RunningAgent::default()
            });
        }
        out.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        out
    }

    /// The person changes a board's switch, model or effort.
    pub fn agents_set(&self, c: AgentsChange) -> Result<AgentsSettings, String> {
        let board_id = c.board_id.clone();
        let s = apply(self.core.spawn_setting(&board_id), c);
        self.core.set_spawn_setting(s)?;
        Ok(self.agents_settings(&board_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_keeps_the_fields_it_doesnt_name() {
        let stored = SpawnSetting {
            on: true,
            model: "opus".into(),
            ..SpawnSetting::default()
        };
        let s = apply(
            stored,
            AgentsChange {
                effort: Some("high".into()),
                ..AgentsChange::default()
            },
        );
        assert_eq!(
            (s.on, s.model.as_str(), s.effort.as_str()),
            (true, "opus", "high")
        );
    }

    #[test]
    fn nothing_picked_shows_sonnet_at_medium() {
        let v = view(&SpawnSetting::default(), false);
        assert_eq!(
            (v.model.as_str(), v.effort.as_str(), v.on),
            ("sonnet", "medium", false)
        );
        assert_eq!(v.models[0], "sonnet");
    }
}
