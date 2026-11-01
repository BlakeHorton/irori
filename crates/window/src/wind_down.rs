//! Wind down all (card #220, docs/design/wind-down.md): the person asks
//! every agent to save its place, hand back its cards and stop; the window
//! shows who has, then turns Pause all on.

use std::sync::Mutex;

use serde::Serialize;
use specta::Type;

use crate::Window;

/// windDown:changed: a run's status moved.
pub const EVENT_WIND_DOWN: &str = "windDown:changed";
/// This device's agents only.
pub const SCOPE_THIS: &str = "this";
/// Every one of the person's devices.
pub const SCOPE_ALL: &str = "all";

/// What a wind-down says (the wake's preamble carries the steps).
pub const WIND_DOWN_TEXT: &str = "Wind down: save your place, hand back your cards, release your leases and reply \"wound down\". Then stop until I resume you.";
/// The Stop a straggler's row sends.
pub const WIND_DOWN_STOP_TEXT: &str = "Wind down now: commit and push your work, post a hand-off and leave your cards, release your leases and reply \"wound down\".";

/// Row states.
pub mod row {
    /// Not reached yet (queued).
    pub const WAITING: &str = "waiting";
    /// Delivered or engaged: it is winding down.
    pub const WINDING: &str = "winding";
    /// It answered.
    pub const DONE: &str = "done";
    /// Closed without an answer (skipped, expired, gone).
    pub const NO_ANSWER: &str = "no_answer";
}

/// One agent in a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
pub struct WindDownRow {
    pub key: String,
    pub name: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub device: String,
    #[serde(rename = "deliveryId")]
    pub delivery_id: String,
    /// [`row`]'s states.
    pub state: String,
    /// Its answer (the cards it left), once it answered.
    pub reply: String,
    /// Why it couldn't be sent, when it couldn't.
    pub error: String,
}

/// A run's status (empty `id`: none yet).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
pub struct WindDownStatus {
    pub id: String,
    pub scope: String,
    #[serde(rename = "pauseAfter")]
    pub pause_after: bool,
    /// Pause all was turned on by this run.
    pub paused: bool,
    /// No agent is still waiting or winding down.
    pub finished: bool,
    pub rows: Vec<WindDownRow>,
}

#[derive(Debug, Default)]
pub(crate) struct Run {
    st: WindDownStatus,
    /// The wind-down message per row (its reply names it).
    msgs: Vec<String>,
}

pub(crate) type RunSlot = Mutex<Option<Run>>;

/// A delivery state as a row state.
pub fn row_state(delivery_state: &str) -> &'static str {
    use inbox::st;
    match delivery_state {
        st::QUEUED | st::HELD => row::WAITING,
        st::DELIVERED | st::ENGAGED | st::DEFERRED => row::WINDING,
        st::ANSWERED => row::DONE,
        _ => row::NO_ANSWER,
    }
}

/// Every row has ended (done or no answer) and at least one was sent.
pub fn finished(rows: &[WindDownRow]) -> bool {
    rows.iter().any(|r| !r.delivery_id.is_empty())
        && rows
            .iter()
            .all(|r| r.state == row::DONE || r.state == row::NO_ANSWER)
}

impl Window {
    /// WindDownService.All: sends a wind-down to every live agent of the
    /// person's (`scope` this or all devices) and starts a run, replacing
    /// any earlier one. With `pause_after`, Pause all turns on once every
    /// agent has answered.
    pub fn wind_down_all(&self, scope: &str, pause_after: bool) -> Result<WindDownStatus, String> {
        let scope = match crate::trim(scope) {
            "" | SCOPE_THIS => SCOPE_THIS,
            SCOPE_ALL => SCOPE_ALL,
            other => return Err(format!("wind down where? {other:?} (this or all)")),
        };
        let here = self.core.device_id().to_string();
        let me = self.core.user_sender();
        let ib = self.core.inbox();
        let mut agents: Vec<_> = self
            .core
            .mesh_agents()
            .into_iter()
            .filter(|a| scope == SCOPE_ALL || a.device_id == here)
            .collect();
        agents.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.key.cmp(&b.key)));
        agents.dedup_by(|a, b| a.key == b.key);
        let mut run = Run {
            st: WindDownStatus {
                id: format!(
                    "wd_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or_default()
                ),
                scope: scope.into(),
                pause_after,
                ..WindDownStatus::default()
            },
            msgs: Vec::new(),
        };
        for a in agents {
            let to = self.core.private_key(&a.key);
            let mut r = WindDownRow {
                key: to.clone(),
                name: a.name.clone(),
                device: self.core.device_label(&a.device_id),
                device_id: a.device_id.clone(),
                state: row::WAITING.into(),
                ..WindDownRow::default()
            };
            let o = ::inbox::SendOptions {
                mode: ::inbox::mode::WAKE.into(),
                reason: ::inbox::REASON_WIND_DOWN.into(),
                ..::inbox::SendOptions::default()
            };
            match ib.send_with(
                &me,
                &::inbox::Address::new(::inbox::addr::AGENT, &to),
                WIND_DOWN_TEXT,
                o,
            ) {
                Ok((m, d)) => {
                    r.delivery_id = d.id;
                    r.state = row_state(&d.state).into();
                    run.msgs.push(m.id);
                }
                Err(e) => {
                    r.state = row::NO_ANSWER.into();
                    r.error = e.to_string();
                    run.msgs.push(String::new());
                }
            }
            run.st.rows.push(r);
        }
        if run.st.rows.is_empty() {
            return Err("no agent is running to wind down".into());
        }
        *self.wind_down.lock().unwrap_or_else(|e| e.into_inner()) = Some(run);
        Ok(self.wind_down_status())
    }

    /// WindDownService.Status: the current run, read fresh from its
    /// deliveries. Turns Pause all on once it finished, if asked (once).
    pub fn wind_down_status(&self) -> WindDownStatus {
        let ib = self.core.inbox();
        let me = self.core.user_sender().key;
        let mut slot = self.wind_down.lock().unwrap_or_else(|e| e.into_inner());
        let Some(run) = slot.as_mut() else {
            return WindDownStatus::default();
        };
        let mut replies: Option<Vec<::inbox::Delivery>> = None;
        for (r, mid) in run.st.rows.iter_mut().zip(run.msgs.iter()) {
            if r.delivery_id.is_empty() {
                continue;
            }
            let Some(d) = ib.get(&r.delivery_id) else {
                continue;
            };
            r.state = row_state(&d.state).into();
            if r.state == row::DONE && r.reply.is_empty() {
                let rs =
                    replies.get_or_insert_with(|| ib.for_agent(&me, true, crate::inbox::MAX_LIST));
                if let Some(x) = rs.iter().find(|x| x.message.reply_to == *mid) {
                    r.reply = x.message.text.clone();
                }
            }
        }
        run.st.finished = finished(&run.st.rows);
        if run.st.finished && run.st.pause_after && !run.st.paused {
            match self.inbox_set_paused(true) {
                Ok(()) => run.st.paused = true,
                Err(e) => irori_core::log(&format!("irori: wind down: pause all: {e}")),
            }
        }
        run.st.clone()
    }

    /// Re-reads the run after an inbox change and tells the window.
    pub(crate) fn wind_down_tick(&self) {
        let has = self
            .wind_down
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|r| !r.st.paused || !r.st.finished);
        if has {
            let st = self.wind_down_status();
            self.emit(EVENT_WIND_DOWN, &st);
        }
    }

    /// Stop on a straggler's row: cuts its running turn with the
    /// wind-down's steps.
    pub fn wind_down_stop(&self, key: &str) -> Result<(), String> {
        self.agent_stop(key, WIND_DOWN_STOP_TEXT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(state: &str) -> WindDownRow {
        WindDownRow {
            delivery_id: "d".into(),
            state: state.into(),
            ..WindDownRow::default()
        }
    }

    #[test]
    fn states() {
        assert_eq!(row_state("queued"), row::WAITING);
        assert_eq!(row_state("held"), row::WAITING);
        assert_eq!(row_state("delivered"), row::WINDING);
        assert_eq!(row_state("engaged"), row::WINDING);
        assert_eq!(row_state("deferred"), row::WINDING);
        assert_eq!(row_state("answered"), row::DONE);
        assert_eq!(row_state("skipped"), row::NO_ANSWER);
    }

    #[test]
    fn finished_only_when_every_row_ended() {
        assert!(!finished(&[]));
        assert!(!finished(&[r(row::DONE), r(row::WINDING)]));
        assert!(!finished(&[r(row::WAITING)]));
        assert!(finished(&[r(row::DONE), r(row::NO_ANSWER)]));
        let unsent = WindDownRow {
            state: row::NO_ANSWER.into(),
            ..WindDownRow::default()
        };
        assert!(!finished(std::slice::from_ref(&unsent)), "nothing was sent");
        assert!(finished(&[unsent, r(row::DONE)]));
    }
}
