//! Settings › Agents (card #448, docs/design/spawned-agents.md §10, sa-6):
//! a board's spawned-agent switch and the model and effort its agents
//! start with. Set in the app only, never by a tool (decision 23). Each
//! write changes only its own fields of the board's `spawn.set` row, so
//! the permission fields part 5 keeps there are left as they are.

use irori_core::spawn::{self, SpawnSetting};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::Window;

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
