//! A holder that ends its turn on a working card with no blocker (card
//! #305). Home reads such a card as calm Waiting, so a card really waiting
//! on something (a PC check, another agent) looks the same as one nobody
//! is working: the holder is nudged once to name what it waits on, or to
//! keep going.

use gocompat::quote::quote;

use super::fold::Fold;
use super::types::*;
use super::util::{SWEEPER, deeper};
use crate::gotime;

impl Fold {
    /// Nudges each of this device's agents that holds a working card with
    /// no open blocker, went idle, and ran a turn since the card last
    /// changed. Once per card change: a nudged agent that changes nothing
    /// isn't told again, so an agent with nothing to say isn't woken over
    /// and over.
    pub(crate) fn tell_idle_holders(&mut self, out: &mut Vec<Notice>) {
        let Some(r) = &self.roster else {
            return;
        };
        let agents = r.agents();
        let idle: std::collections::HashMap<&str, &crate::roster::AgentCard> = agents
            .iter()
            .filter(|a| a.status == "idle" && a.device_state != "offline" && self.local_key(&a.key))
            .map(|a| (a.key.as_str(), a))
            .collect();
        // Forget pairs whose card closed or changed hands.
        self.idle_told.retain(|(h, id), _| {
            self.tasks
                .get(id)
                .is_some_and(|t| !is_terminal(&t.state) && t.holders.contains(h))
        });
        if idle.is_empty() {
            return;
        }
        let mut todo: Vec<(String, String)> = Vec::new();
        for t in self.tasks.values() {
            if !idle_waits_unnamed(t) {
                continue;
            }
            for h in &t.holders {
                let Some(a) = idle.get(h.as_str()) else {
                    continue;
                };
                // A turn ran and ended after the card's last change.
                if !gotime::after(&a.last_activity, &t.updated_at)
                    || !gotime::after(&a.updated_at, &t.updated_at)
                {
                    continue;
                }
                let pair = (h.clone(), t.id.clone());
                if self.idle_told.get(&pair) == Some(&t.updated_at) {
                    continue;
                }
                todo.push(pair);
            }
        }
        todo.sort();
        for (h, id) in todo {
            let t = &self.tasks[&id];
            self.idle_told.insert((h.clone(), id.clone()), t.updated_at);
            out.push(idle_notice(t, &h));
        }
    }
}

/// The card would read "Waiting" with no reason: held and working, with no
/// open blocker, ask, review, pause or rollup saying what it waits on.
pub(crate) fn idle_waits_unnamed(t: &Task) -> bool {
    t.state == WORKING
        && !t.holders.is_empty()
        && t.rollup.is_none()
        && !t.paused
        && t.open_ask().is_none()
        && t.open_blockers().is_empty()
}

fn idle_notice(t: &Task, to: &str) -> Notice {
    let r = t.short_ref();
    let text = format!(
        "You hold {} ({}) and your turn ended with no open blocker and no change to the card, so the person sees it as Waiting with no reason given. \
If it waits on something, name it with board_block on: \"agent <name>\" (e.g. another agent's check), \"task <id>\", \"lease <resource>\", \"external <what>\" (CI, a review), or \"person\" with a question; one blocker per thing. \
If there's more to do, keep going and tick its steps. If it's done, set it completed with proof.{}",
        quote(&t.title),
        r,
        deeper(&t.id)
    );
    Notice {
        kind: NOTICE_IDLE.into(),
        task_id: t.id.clone(),
        reference: format!("task:{}", t.id),
        title: t.title.clone(),
        by: SWEEPER.into(),
        to: vec![to.to_string()],
        // The agent is idle: a notice that doesn't wake it waits for the
        // person to type in its window, and the card sits unnamed.
        wake: true,
        text,
        ..Notice::default()
    }
}
