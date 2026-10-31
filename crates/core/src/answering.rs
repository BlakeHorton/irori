//! Agents that stopped answering (card #110): a live, idle Claude session
//! whose wakes go unread. presence still sees its process, so the roster
//! kept it idle and Irori kept handing it work. The Core looks at the inbox
//! on its coord sweep and keeps the agents it finds here; presence's status
//! hook reads only this map (it runs inside presence's scan, so it never
//! touches the inbox).

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use gocompat::time::GoTime;

use coord::roster::NOT_ANSWERING;

fn ns(t: &GoTime) -> i128 {
    i128::from(t.secs) * 1_000_000_000 + i128::from(t.nanos)
}

/// Whether presence status `s` is idle, counting one this map set.
fn idle(s: &str) -> bool {
    s == "idle" || s == NOT_ANSWERING
}

/// The agents not answering now, each with since when a wake has waited on
/// it.
#[derive(Default)]
pub struct Answering {
    m: RwLock<HashMap<String, GoTime>>,
}

impl Answering {
    /// presence's status hook: [`NOT_ANSWERING`] for an idle card in the
    /// map that did nothing since its wake started waiting, else "".
    pub fn status(&self, c: &presence::AgentCard) -> String {
        if !idle(&c.status) {
            return String::new();
        }
        let m = self.m.read().unwrap_or_else(|e| e.into_inner());
        match m.get(&c.key) {
            Some(since) if ns(&c.last_activity) <= ns(since) => NOT_ANSWERING.into(),
            _ => String::new(),
        }
    }

    /// Looks again at this device's cards: an idle one that a wake has
    /// waited on for longer than `after` (`unanswered(key, after)`, the
    /// inbox), and that did nothing since, is not answering. Returns the
    /// keys newly not answering, and whether the map changed (presence
    /// then rescans so the roster shows it).
    pub fn refresh(
        &self,
        cards: &[presence::AgentCard],
        after: Duration,
        unanswered: impl Fn(&str, Duration) -> Option<GoTime>,
    ) -> (Vec<String>, bool) {
        let mut next: HashMap<String, GoTime> = HashMap::new();
        for c in cards {
            if c.key.is_empty() || !idle(&c.status) {
                continue;
            }
            let Some(since) = unanswered(&c.key, after) else {
                continue;
            };
            if ns(&c.last_activity) <= ns(&since) {
                next.insert(c.key.clone(), since);
            }
        }
        let mut m = self.m.write().unwrap_or_else(|e| e.into_inner());
        let mut newly: Vec<String> = next
            .keys()
            .filter(|k| !m.contains_key(*k))
            .cloned()
            .collect();
        newly.sort();
        let changed = next.len() != m.len() || !newly.is_empty();
        *m = next;
        (newly, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(secs: i64) -> GoTime {
        GoTime {
            secs,
            ..GoTime::default()
        }
    }

    fn card(key: &str, status: &str, last: i64) -> presence::AgentCard {
        presence::AgentCard {
            key: key.into(),
            status: status.into(),
            last_activity: t(last),
            ..presence::AgentCard::default()
        }
    }

    #[test]
    fn idle_agent_with_an_unread_wake_is_not_answering() {
        let a = Answering::default();
        let cards = vec![
            card("d/garnet", "idle", 100),
            card("d/busy", "busy", 100),
            card("d/woke", "idle", 300),
            card("d/fine", "idle", 100),
        ];
        let waits = |k: &str, _: Duration| match k {
            "d/garnet" | "d/busy" | "d/woke" => Some(t(200)),
            _ => None,
        };
        let (newly, changed) = a.refresh(&cards, Duration::from_secs(300), waits);
        assert_eq!(
            newly,
            vec!["d/garnet".to_string()],
            "busy, active-since and unwaited agents answer"
        );
        assert!(changed);
        assert_eq!(a.status(&cards[0]), NOT_ANSWERING);
        assert_eq!(a.status(&cards[1]), "", "a busy session is never marked");
        assert_eq!(a.status(&cards[3]), "");

        // Its own overlay counts as idle; a second look changes nothing.
        let marked = vec![card("d/garnet", NOT_ANSWERING, 100)];
        let (newly, changed) = a.refresh(&marked, Duration::from_secs(300), waits);
        assert!(newly.is_empty() && !changed);

        // It takes its wake up: activity after the wait clears it at once,
        // before the next look.
        let back = card("d/garnet", "idle", 250);
        assert_eq!(a.status(&back), "");
        let (_, changed) = a.refresh(&[back], Duration::from_secs(300), |_, _| None);
        assert!(changed);
    }
}
