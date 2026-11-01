//! Offers: a card an agent posts unheld goes to one free agent at a time
//! (card #82, research 07 case G), the way an ask does (#91, decision 38).
//! Pull, not push: the Core knows who is free, so Irori hands the card out
//! instead of the poster picking someone off the roster.
//!
//! The card goes to the free agent idle longest, which takes it with
//! board_join. It moves on only on an event, never on time: that agent
//! gives it back (board_leave, or skipping the delivery), its turn ends
//! without taking it, or it goes away. With nobody free it waits, and goes
//! out when an agent goes idle. The poster's device runs the hand-out and
//! writes its `task.offered` rows. An agent busy on a working card (or
//! owning open steps on one) is told no when it tries to take a second.

use std::collections::HashMap;

use gocompat::quote::quote;
use gocompat::time::GoTime;
use irori_identity::owner;
use store::{Event, kinds};

use super::asks::Handed;
use super::error::{Error, ErrorKind, Result, bad_state};
use super::fold::{Fold, Touch, engaged};
use super::rows::*;
use super::types::*;
use super::util::{SWEEPER, caller, contains, cut, deeper};
use crate::gotime;
use crate::roster::AgentCard;

/// What an offer notice quotes of the card's objective.
const MAX_OFFER_NOTICE: usize = 600;

/// Irori handed it to `to`.
pub const OFFER_HANDED: &str = "handed";
/// The agent it went to gave it back or ended its turn without taking it.
pub const OFFER_PASSED: &str = "passed";
/// Nobody is free: it waits for the next agent to go idle.
pub const OFFER_WAITING: &str = "waiting";

/// Whether card t is one Irori hands out now: open, unheld, unassigned,
/// private, not an ask and not a parent.
pub(crate) fn offerable(t: &Task) -> bool {
    t.state == SUBMITTED
        && t.holders.is_empty()
        && t.assigned.is_empty()
        && !t.shared
        && !t.withdrawn
        && t.ask.is_none()
        && t.subtasks.is_empty()
}

impl Task {
    /// Irori's hand-out while it applies: the card is still unheld.
    pub fn live_offer(&self) -> Option<&Offer> {
        self.offer.as_ref().filter(|_| offerable(self))
    }
}

impl Fold {
    /// The working card `key` already has in hand, if any: one it holds
    /// and has taken up, or owns open steps on (card #86), open, not
    /// shared, and not a parent (a parent's state follows its subtasks,
    /// and its holder runs them).
    pub(crate) fn busy_with(&self, key: &str, except: &str) -> Option<&Task> {
        let mut busy: Vec<&Task> = self
            .tasks
            .values()
            .filter(|t| {
                t.id != except
                    && t.state == WORKING
                    && !t.shared
                    && t.subtasks.is_empty()
                    && t.open_ask().is_none()
                    && (engaged(t, key) || contains(&t.co_holders, key))
            })
            .collect();
        busy.sort_by(|a, b| a.id.cmp(&b.id));
        busy.into_iter().next()
    }

    /// Refuses `by` taking card t while it already works another one. A
    /// shared card, a card someone assigned to `by`, and a person are let
    /// through.
    pub(crate) fn check_free(&self, by: &str, t: &Task) -> Result<()> {
        if t.shared || contains(&t.assigned, by) || self.is_owner(by) {
            return Ok(());
        }
        match self.busy_with(by, &t.id) {
            None => Ok(()),
            Some(b) => Err(Error::raw(
                ErrorKind::NotAllowed,
                format!(
                    "coord: you already hold {} {} and it is still working: one card at a time. Finish it (board_status completed) or hand it back (board_leave) first; until then leave {} for a free agent.",
                    b.short_ref(),
                    quote(&b.title),
                    t.short_ref()
                ),
            )),
        }
    }

    /// Keeps the inbox's view of card `id`'s hand-out current, so a
    /// hand-out notice to an agent it no longer goes to closes before it
    /// wakes that agent (the Stale hook never takes coord's lock).
    pub(crate) fn sync_offer_stale(&self, id: &str) {
        let Some(t) = self.tasks.get(id).filter(|t| t.offer.is_some()) else {
            return;
        };
        let v = match t.live_offer() {
            Some(o) => (true, o.to.clone()),
            None if is_terminal(&t.state) => (false, "the card closed".to_string()),
            None if !t.holders.is_empty() => (false, format!("taken by {}", t.holders.join(", "))),
            None if !t.assigned.is_empty() => {
                (false, format!("assigned to {}", t.assigned.join(", ")))
            }
            None => (false, "it is no longer handed out".to_string()),
        };
        let mut s = self.stale.write().unwrap_or_else(|e| e.into_inner());
        s.offers.insert(id.to_string(), v);
    }

    /// The agents an unheld card other than `id` is handed to now: one
    /// thing at a time.
    pub(crate) fn offered_elsewhere(&self, id: &str) -> Vec<String> {
        self.tasks
            .values()
            .filter(|t| t.id != id)
            .filter_map(|t| t.live_offer())
            .filter(|o| !o.to.is_empty())
            .map(|o| o.to.clone())
            .collect()
    }

    /// The agents among `roster` free to take card t now, the one idle
    /// longest first: live, idle, in t's repo when both have one, holding
    /// no working card, handed no other card or ask, and not in `skip`.
    pub(crate) fn free_for(&self, t: &Task, roster: &[AgentCard], skip: &[String]) -> Vec<String> {
        let elsewhere = self.handed_elsewhere(&t.id);
        let mut out: Vec<String> = Vec::new();
        let mut since: HashMap<String, GoTime> = HashMap::new();
        for a in roster {
            if a.key.is_empty()
                || a.key == SWEEPER
                || self.is_owner(&a.key)
                || a.status != "idle"
                || a.device_state == crate::roster::OFFLINE
            {
                continue;
            }
            let dev = if a.device_id.is_empty() {
                owner::device_of(&a.key)
            } else {
                a.device_id.as_str()
            };
            if let Some(r) = &self.revoked
                && !dev.is_empty()
                && r(dev)
            {
                continue;
            }
            if !t.repo_key.is_empty() && !a.repo_key.is_empty() && a.repo_key != t.repo_key {
                continue;
            }
            if skip.contains(&a.key)
                || out.contains(&a.key)
                || elsewhere.contains(&a.key)
                || self.busy_with(&a.key, "").is_some()
            {
                continue;
            }
            out.push(a.key.clone());
            since.insert(a.key.clone(), a.updated_at);
        }
        out.sort_by(|x, y| gotime::cmp(&since[x], &since[y]).then_with(|| x.cmp(y)));
        out
    }

    /// `by` puts its new card `id` up for a free agent: Irori hands it to
    /// the one idle longest at once, or it waits for the next to go idle.
    /// The notice is for that agent (its `to` empty when nobody is free).
    pub(crate) fn offer_card(
        &mut self,
        by: &str,
        id: &str,
        roster: &[AgentCard],
    ) -> Result<Notice> {
        let by = caller(by)?;
        let t = self.open_task(id)?.clone();
        if !offerable(&t) || self.on_board {
            return Err(bad_state(format!(
                "{} isn't an unheld card Irori can hand out",
                t.short_ref()
            )));
        }
        let now = self.stamp();
        let skip = vec![by.clone()];
        let free = self.free_for(&t, roster, &skip);
        let row = match free.first() {
            Some(pick) => TaskOfferedRow {
                to: pick.clone(),
                reason: OFFER_HANDED.into(),
                heard: vec![pick.clone()],
                ..TaskOfferedRow::default()
            },
            None => TaskOfferedRow {
                reason: OFFER_WAITING.into(),
                ..TaskOfferedRow::default()
            },
        };
        self.write_offer(id, &by, now, row)?;
        let t = self.tasks[id].clone();
        Ok(match free.first() {
            Some(pick) => offer_notice(&t, pick, &by, ""),
            None => Notice::default(),
        })
    }

    /// `who` gives back card `id` Irori handed it: it goes to the next
    /// free agent. Ok(false) when it wasn't handed to `who`.
    pub(crate) fn pass_offer(&mut self, who: &str, id: &str) -> Result<bool> {
        let Some(o) = self.tasks.get(id).and_then(|t| t.live_offer()).cloned() else {
            return Ok(false);
        };
        if o.to != who {
            return Ok(false);
        }
        let now = self.stamp();
        self.write_offer(
            id,
            who,
            now,
            TaskOfferedRow {
                reason: OFFER_PASSED.into(),
                was: who.into(),
                heard: o.heard,
                ..TaskOfferedRow::default()
            },
        )?;
        Ok(true)
    }

    fn write_offer(
        &mut self,
        id: &str,
        by: &str,
        now: GoTime,
        mut r: TaskOfferedRow,
    ) -> Result<()> {
        r.task_id = id.into();
        r.by = by.into();
        r.since = now;
        r.at = now;
        let actor = if by == SWEEPER {
            SWEEPER.to_string()
        } else {
            self.actor_for(by)
        };
        self.write_as(&actor, kinds::TASK_OFFERED, &r)
    }

    pub(crate) fn fold_offered(&mut self, ev: &Event, at: &dyn Fn(GoTime) -> GoTime) {
        let Some(p) = row::<TaskOfferedRow>(&ev.payload) else {
            return;
        };
        let Some(t) = self.tasks.get_mut(&p.task_id) else {
            return;
        };
        let was = t.offer.as_ref().map(|o| o.to.clone()).unwrap_or_default();
        t.offer = Some(Offer {
            to: p.to.clone(),
            reason: p.reason.clone(),
            since: at(p.since),
            heard: p.heard.clone(),
        });
        let ti = Touch::of(t);
        let when = at(p.at);
        let gone = if p.was.is_empty() { was } else { p.was.clone() };
        match p.reason.as_str() {
            OFFER_HANDED if !p.to.is_empty() => {
                if !p.was.is_empty() {
                    self.touch(ev, &ti, when, &p.was, "passed it on", false, Vec::new());
                }
                let what = format!("handed it to {}", self.named(&p.to));
                self.touch(ev, &ti, when, SWEEPER, &what, false, Vec::new());
            }
            OFFER_PASSED if !gone.is_empty() => {
                self.touch(ev, &ti, when, &gone, "passed it on", false, Vec::new());
            }
            OFFER_WAITING if !gone.is_empty() => {
                let what = "passed it on; no agent is free, so it goes to the next one to go idle";
                self.touch(ev, &ti, when, &gone, what, false, Vec::new());
            }
            OFFER_WAITING => {
                let what = "no agent is free: it goes to the next one to go idle";
                self.touch(ev, &ti, when, SWEEPER, what, false, Vec::new());
            }
            _ => {}
        }
        self.note(Change {
            kind: TASK_CHANGED.into(),
            task_id: p.task_id,
            ..Change::default()
        });
    }

    /// What this device does for the unheld cards its agents posted: hands
    /// each on when the agent it went to gave it back, ended its turn
    /// without taking it, or went away, and hands out the ones waiting
    /// when an agent is free. Never by time waited.
    pub(crate) fn sweep_offers(&mut self, agents: &[AgentCard]) -> (Vec<Notice>, Result<()>) {
        let mut out = Vec::new();
        if self.on_board {
            return (out, Ok(()));
        }
        let in_roster: HashMap<&str, &AgentCard> =
            agents.iter().map(|a| (a.key.as_str(), a)).collect();
        // Cards posted here that never had a hand-out (posted before 0.7.1,
        // put back on the Board by Irori, or with their offer row lost) go
        // out too (card #110), as if just posted.
        let mut ids: Vec<(GoTime, String)> = self
            .tasks
            .values()
            .filter(|t| {
                (t.live_offer().is_some() || (t.offer.is_none() && offerable(t)))
                    && owner::device_of(&t.created_by) == self.dev
            })
            .map(|t| (t.created_at, t.id.clone()))
            .collect();
        // Oldest card first: it gets the agent idle longest.
        ids.sort_by(|a, b| gotime::cmp(&a.0, &b.0).then_with(|| a.1.cmp(&b.1)));
        let res = (|| -> Result<()> {
            for (_, id) in &ids {
                let t = &self.tasks[id];
                let o = match t.live_offer() {
                    Some(o) => o.clone(),
                    None if t.offer.is_none() && offerable(t) => Offer::default(),
                    None => continue,
                };
                let mut heard = o.heard.clone();
                let (was, why) = if !o.to.is_empty() {
                    match self.turn_state(&o.to, &o.since, &in_roster) {
                        Handed::Waits | Handed::Touched => continue,
                        Handed::Ended => (
                            o.to.clone(),
                            "the agent it went to ended its turn without taking it",
                        ),
                        Handed::Gone => (o.to.clone(), "the agent it went to went away"),
                    }
                } else if o.reason == OFFER_PASSED {
                    (String::new(), "the agent it went to gave it back")
                } else {
                    (String::new(), "")
                };
                if !was.is_empty() && !heard.contains(&was) {
                    heard.push(was.clone());
                }
                let t = self.tasks[id].clone();
                let mut skip = heard.clone();
                skip.push(t.created_by.clone());
                let free = self.free_for(&t, agents, &skip);
                let now = self.stamp();
                if let Some(pick) = free.first() {
                    heard.push(pick.clone());
                    self.write_offer(
                        id,
                        SWEEPER,
                        now,
                        TaskOfferedRow {
                            to: pick.clone(),
                            was,
                            reason: OFFER_HANDED.into(),
                            heard,
                            ..TaskOfferedRow::default()
                        },
                    )?;
                    let t = self.tasks[id].clone();
                    out.push(offer_notice(&t, pick, &t.created_by, why));
                    continue;
                }
                // Nobody free: it waits, and goes out the moment an agent
                // goes idle (the next sweep, kicked by the roster change).
                if o.to.is_empty() && o.reason == OFFER_WAITING {
                    continue;
                }
                self.write_offer(
                    id,
                    SWEEPER,
                    now,
                    TaskOfferedRow {
                        was,
                        reason: OFFER_WAITING.into(),
                        heard,
                        ..TaskOfferedRow::default()
                    },
                )?;
            }
            Ok(())
        })();
        (out, res)
    }
}

/// The notice handing unheld card t to agent `to`.
fn offer_notice(t: &Task, to: &str, poster: &str, why: &str) -> Notice {
    let mut b = format!(
        "{poster} put up a card, and Irori handed it to you{}: {} ({})",
        if why.is_empty() {
            String::new()
        } else {
            format!(" ({why})")
        },
        quote(&t.title),
        t.refs()
    );
    if !t.objective.is_empty() {
        b.push_str(&format!(":\n{}", cut(&t.objective, MAX_OFFER_NOTICE)));
    } else {
        b.push('.');
    }
    if !t.done_criteria.is_empty() {
        b.push_str(&format!(
            "\nDone when: {}",
            cut(&t.done_criteria, MAX_OFFER_NOTICE)
        ));
    }
    b.push_str(&t.handover_lines());
    b.push_str(&format!(
        "\nIt is yours to take: board_join {} and start. If you can't take it on now, board_leave {} and it goes to the next free agent; ending your turn without taking it does the same.",
        t.short_ref(),
        t.short_ref()
    ));
    b.push_str(&deeper(&t.id));
    Notice {
        kind: NOTICE_OFFER.into(),
        task_id: t.id.clone(),
        reference: format!("task:{}", t.id),
        title: t.title.clone(),
        by: poster.to_string(),
        to: vec![to.to_string()],
        // A free agent is idle, and an idle agent gets no next turn until
        // someone types in its window: an offer that doesn't wake reaches
        // nobody (as with asks, #69).
        wake: true,
        text: b,
        ..Notice::default()
    }
}
