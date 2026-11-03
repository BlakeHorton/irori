//! Threads (card #116): the person's conversation with the room, not work.
//!
//! A thread is an ask with `thread` set. It doesn't go to one idle agent
//! at once: the agents on the cards it names (#12) and the agents it
//! @mentions hear it first, woken, and answer if it's about their work.
//! If no agent has answered after [`THREAD_QUIET`], the ask's hand-out
//! gives it to one idle agent. Once an agent answers, the thread is
//! answered: nobody holds it (an agent in a thread can still take cards),
//! and the person's follow-ups go to the agents that answered. It stays
//! open until the person resolves it.
//!
//! On a shared board (card #134) a thread is the same, among that board's
//! agents and people: its rows live in the board's log, the agents on the
//! board's cards it names hear it first, and one idle agent of any member
//! gets it after the quiet spell. Another member's comment is a person's,
//! not an answer; the person who started it resolves it. Mentions stay off
//! board logs, as for every board ask.

use std::collections::BTreeSet;
use std::time::Duration;

use gocompat::time::GoTime;
use store::Event;

use super::asks::TAKE_POSTED;
use super::fold::{Fold, MADE_KIND, Touch, card_refs, line_of};
use super::rows::*;
use super::types::*;
use super::util::{SWEEPER, cut, deeper};
use crate::gotime;

/// How long the agents a thread names have to answer before one idle
/// agent gets it.
pub const THREAD_QUIET: Duration = Duration::from_secs(45);

/// A thread an agent answered: nothing more to hand out.
pub const TAKE_ANSWERED: &str = "answered";

const MAX_THREAD_NOTICE: usize = 1200;

/// What every agent in a thread hears about closing it.
pub const NOT_YOURS_TO_CLOSE: &str = "Don't ask_close it: the person resolves it when they're done. If your answer says you'll do work, answer in the thread first (board_comment: what you'll do), then make the card with board_create from=<this thread's #ref> (or board_edit the card it changes): Irori refuses a card from a thread you haven't answered, and the thread then shows your answer and the card. If you do it with no card, say so plainly in the thread.";

impl Fold {
    /// Whether `key` is a person, not an agent: this device's person, or on
    /// a board any member's person actor.
    pub(crate) fn is_person(&self, key: &str) -> bool {
        self.is_owner(key) || (self.on_board && key.starts_with(store::kinds::PERSON_ACTOR_PREFIX))
    }

    /// Who hears a person's follow-up in thread `t` now: the agents that
    /// answered, or before any answer, the ones it went to (and its taker).
    pub(crate) fn thread_to(&self, t: &Task) -> Vec<String> {
        let a = t.ask.clone().unwrap_or_default();
        let to = self.thread_repliers(&t.id);
        if !to.is_empty() || a.reason == TAKE_ANSWERED {
            return to;
        }
        let mut to = a.heard.clone();
        if !a.taker.is_empty() && !to.contains(&a.taker) {
            to.push(a.taker.clone());
        }
        to
    }

    /// The agents a new thread goes to first: those holding the open cards
    /// its text names, then the agents it @mentions (live ones).
    pub(crate) fn thread_hearers(&self, text: &str, mentions: &[Mention]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let push = |k: &str, out: &mut Vec<String>| {
            if !k.is_empty() && k != SWEEPER && !self.is_person(k) && !out.iter().any(|o| o == k) {
                out.push(k.to_string());
            }
        };
        let mut seen = BTreeSet::new();
        for (tag, n) in card_refs(text) {
            let mut hits = self.tasks.values().filter(|t| {
                t.short == n && tag.as_ref().is_none_or(|g| *g == t.tag) && t.ask.is_none()
            });
            let (Some(t), None) = (hits.next(), hits.next()) else {
                continue;
            };
            if is_terminal(&t.state) || !seen.insert(t.id.clone()) {
                continue;
            }
            for h in t.holders.iter().chain(&t.co_holders) {
                push(h, &mut out);
            }
        }
        for m in mentions {
            if !m.gone {
                push(&m.key, &mut out);
            }
        }
        out
    }

    /// The agents that have answered in thread `id`, first answer first.
    pub(crate) fn thread_repliers(&self, id: &str) -> Vec<String> {
        let mut cs: Vec<&Comment> = self
            .comments
            .values()
            .filter(|c| {
                c.task_id == id && c.msg_id.is_empty() && !self.is_person(&c.by) && c.by != SWEEPER
            })
            .collect();
        cs.sort_by(|a, b| gotime::cmp(&a.at, &b.at).then_with(|| a.id.cmp(&b.id)));
        let mut out: Vec<String> = Vec::new();
        for c in cs {
            if !out.contains(&c.by) {
                out.push(c.by.clone());
            }
        }
        out
    }

    /// The thread's next step in the owner's sweep, before the ask
    /// hand-out runs; true when the hand-out should skip it now.
    pub(crate) fn thread_sweep(&mut self, id: &str, now: &GoTime) -> super::error::Result<bool> {
        let t = &self.tasks[id];
        let Some(a) = t.ask.clone() else {
            return Ok(false);
        };
        if !a.thread {
            return Ok(false);
        }
        if a.reason == TAKE_ANSWERED && a.taker.is_empty() {
            return Ok(true);
        }
        // An agent's answer since it last went out.
        let answered = self.comments.values().any(|c| {
            c.task_id == id
                && !self.is_person(&c.by)
                && c.by != SWEEPER
                && c.msg_id.is_empty()
                && !gotime::before(&c.at, &a.since)
        });
        if answered {
            self.write_thread_answered(id, &a)?;
            return Ok(true);
        }
        if a.taker.is_empty()
            && a.reason == TAKE_POSTED
            && gotime::before(now, &gotime::add(&a.since, THREAD_QUIET))
        {
            return Ok(true);
        }
        Ok(false)
    }

    /// Marks thread `id` answered: nobody holds it from now on.
    pub(crate) fn write_thread_answered(&mut self, id: &str, a: &Ask) -> super::error::Result<()> {
        self.write_take(
            id,
            TakeStateRow {
                reason: TAKE_ANSWERED.into(),
                told: a.told,
                heard: a.heard.clone(),
                ..TakeStateRow::default()
            },
        )
    }

    /// The person's follow-up in a thread: it goes to the agents that
    /// answered (live ones), woken; before any answer, to the agents it went
    /// to. Answered, but none of them live: the thread goes out again (an
    /// idle agent gets it at the next sweep).
    pub(crate) fn thread_follow_up(
        &mut self,
        t: &Task,
        by: &str,
        text: &str,
        cid: &str,
        around: Option<&dyn Fn(&str) -> bool>,
    ) -> super::error::Result<Vec<Notice>> {
        let a = t.ask.clone().unwrap_or_default();
        // On a board another member's agents run on their devices: this one
        // can't tell if they're live, and they hear it through the bridge.
        let around = if self.on_board { None } else { around };
        let live = |k: &String| k != by && around.is_none_or(|f| f(k));
        let mut to: Vec<String> = self
            .thread_repliers(&t.id)
            .into_iter()
            .filter(live)
            .collect();
        if to.is_empty() && a.reason != TAKE_ANSWERED {
            // Not answered yet: the agents it went to hear the addition.
            to = a.heard.iter().filter(|k| live(k)).cloned().collect();
            if !a.taker.is_empty() && !to.contains(&a.taker) && live(&a.taker) {
                to.push(a.taker.clone());
            }
            if to.is_empty() {
                return Ok(Vec::new());
            }
        }
        if to.is_empty() {
            // Only the device that owns the ask hands it out again.
            if a.owner != self.dev {
                return Ok(Vec::new());
            }
            self.write_take(
                &t.id,
                TakeStateRow {
                    reason: super::asks::TAKE_REOPENED.into(),
                    told: a.told,
                    heard: Vec::new(),
                    since: self.stamp(),
                    ..TakeStateRow::default()
                },
            )?;
            return Ok(Vec::new());
        }
        let mut b = format!(
            "The person replied in their thread {} ({}):\n{}\n",
            gocompat::quote::quote(&t.title),
            t.refs(),
            cut(text, MAX_THREAD_NOTICE)
        );
        b.push_str("Answer in the thread with board_comment if it's for you. ");
        if to.len() > 1 {
            b.push_str("Others answered there too: read it first, one answer is enough. ");
        }
        b.push_str(NOT_YOURS_TO_CLOSE);
        b.push_str(&deeper(&t.id));
        Ok(vec![Notice {
            kind: NOTICE_COMMENT.into(),
            task_id: t.id.clone(),
            reference: format!("comment:{cid}"),
            title: t.title.clone(),
            by: by.into(),
            to,
            wake: true,
            person: true,
            text: b,
            ..Notice::default()
        }])
    }
}

impl Fold {
    /// The person reopens their resolved thread `id` (card #169): it is a
    /// live thread again. Their next reply goes to the agents that
    /// answered it; with none, it goes out to an idle agent as a new
    /// thread would. Agents don't reopen threads.
    pub(crate) fn reopen_thread(&mut self, by: &str, id: &str) -> super::error::Result<Task> {
        let t = self.any_task(id)?.clone();
        let Some(a) = t.ask.clone().filter(|a| a.thread) else {
            return Err(super::error::invalid(format!(
                "{} isn't a thread",
                t.short_ref()
            )));
        };
        // On a board, the member who started it (card #134).
        if !self.is_owner(by) || (self.on_board && t.created_by != by) {
            return Err(super::error::not_allowed(
                "only the person who started a thread reopens it",
            ));
        }
        if a.is_open() {
            return Err(super::error::bad_state(format!(
                "{} is open already",
                t.short_ref()
            )));
        }
        let at = self.stamp();
        self.write(
            by,
            store::kinds::TASK_STATUS,
            &TaskStateRow {
                task_id: id.into(),
                state: SUBMITTED.into(),
                note: "reopened by the person".into(),
                reopen_ask: true,
                by: by.into(),
                at,
                ..TaskStateRow::default()
            },
        )?;
        // Only the device that owns the ask writes its take rows; on a
        // board another device of the same person leaves them to it.
        if a.owner != self.dev {
            return Ok(self.tasks[id].clone());
        }
        let answered = !self.thread_repliers(id).is_empty();
        let since = self.stamp();
        self.write_take(
            id,
            TakeStateRow {
                reason: if answered {
                    TAKE_ANSWERED.into()
                } else {
                    super::asks::TAKE_REOPENED.into()
                },
                told: a.told,
                since,
                ..TakeStateRow::default()
            },
        )?;
        Ok(self.tasks[id].clone())
    }

    /// Whether `id` is a thread the person hasn't resolved.
    pub(crate) fn open_thread(&self, id: &str) -> bool {
        self.tasks.get(id).is_some_and(|t| {
            t.ask
                .as_ref()
                .is_some_and(|a| a.thread && a.outcome.is_none())
                && !is_terminal(&t.state)
        })
    }

    /// The thread a card `by` makes or changes comes from (card #155):
    /// `from` (a thread's ref) when given, else the one open thread `by`
    /// answered or was handed. None, or more than one: no thread.
    ///
    /// An agent must have answered on an open thread before it makes or
    /// changes a card from it (card #322): the person sees its answer, not
    /// only the card, and can resolve the thread.
    pub(crate) fn thread_from(&self, by: &str, from: &str) -> super::error::Result<String> {
        if !from.trim().is_empty() {
            let id = self.task_ref_locked(from)?;
            let t = self.any_task(&id)?;
            if !t.ask.as_ref().is_some_and(|a| a.thread) {
                return Err(super::error::invalid(format!(
                    "from: {} isn't a thread; give the #ref of the person's thread the work came from",
                    t.short_ref()
                )));
            }
            self.check_answered(by, &id)?;
            return Ok(id);
        }
        if by.is_empty() || by == SWEEPER || self.is_owner(by) {
            return Ok(String::new());
        }
        let mut hits = self.tasks.values().filter(|t| {
            self.open_thread(&t.id)
                && (t.ask.as_ref().is_some_and(|a| a.taker == by)
                    || self.thread_repliers(&t.id).iter().any(|k| k == by))
        });
        let id = match (hits.next(), hits.next()) {
            (Some(t), None) => t.id.clone(),
            _ => return Ok(String::new()),
        };
        self.check_answered(by, &id)?;
        Ok(id)
    }

    /// Refuses a card from open thread `id` by an agent that hasn't
    /// answered on it yet (card #322).
    fn check_answered(&self, by: &str, id: &str) -> super::error::Result<()> {
        if by.is_empty() || by == SWEEPER || self.is_person(by) || !self.open_thread(id) {
            return Ok(());
        }
        if self.thread_repliers(id).iter().any(|k| k == by) {
            return Ok(());
        }
        let r = self.tasks[id].short_ref();
        Err(super::error::not_allowed(format!(
            "{r} is the person's thread and you haven't answered on it yet. Answer there first \
(board_comment {r}: one or two lines on what you'll do), then make the card from it (from={r}): \
the thread shows your answer and the card, so the person can resolve it."
        )))
    }

    /// Card `card` was made or changed from thread `thread` (card #155):
    /// the thread lists it and gets a line, "made #155 from this thread".
    /// Derived from the card's row, like a mention: no row of its own.
    pub(crate) fn fold_made(
        &mut self,
        ev: &Event,
        thread: &str,
        card: &str,
        at: GoTime,
        by: &str,
        verb: &str,
    ) {
        let Some(cref) = self.tasks.get(card).map(|t| t.short_ref()) else {
            return;
        };
        let Some(th) = self.tasks.get_mut(thread) else {
            return;
        };
        if !th.ask.as_ref().is_some_and(|a| a.thread) {
            return;
        }
        if !th.made.iter().any(|c| c == card) {
            th.made.push(card.into());
        }
        let ti = Touch::of(th);
        let mut line = line_of(ev, at, by, &format!("{verb} {cref} from this thread"));
        line.kind = MADE_KIND.into();
        self.touch_line(&ti, line, true, Vec::new());
        self.note(Change {
            kind: TASK_CHANGED.into(),
            task_id: thread.into(),
            ..Change::default()
        });
    }

    /// The person resolves thread `id`: closed as answered. Agents don't.
    pub(crate) fn resolve_thread(&mut self, by: &str, id: &str) -> super::error::Result<Task> {
        let t = self.any_task(id)?;
        if !t.ask.as_ref().is_some_and(|a| a.thread) {
            return Err(super::error::invalid(format!(
                "{} isn't a thread",
                t.short_ref()
            )));
        }
        // On a board, the member who started it.
        if !self.is_owner(by) || (self.on_board && t.created_by != by) {
            return Err(super::error::not_allowed(
                "only the person who started a thread resolves it",
            ));
        }
        self.ask_close(
            by,
            id,
            AskOutcome {
                kind: super::asks::OUTCOME_ANSWERED.into(),
                note: "resolved".into(),
                ..AskOutcome::default()
            },
        )
    }
}

/// What the agents a new thread names hear.
pub(crate) fn thread_notice(t: &Task, to: &[String]) -> Notice {
    let mut b = format!(
        "The person started a thread ({}). It's a question or a conversation, not a card:\n{}\n",
        t.refs(),
        cut(&t.objective, MAX_THREAD_NOTICE)
    );
    b.push_str(&format!(
        "You hear it first because it names your card or you. If you can answer, answer in the thread with board_comment {}; if it isn't about your work, leave it. If no agent answers within {}s, Irori hands it to one idle agent. {NOT_YOURS_TO_CLOSE}",
        t.short_ref(),
        THREAD_QUIET.as_secs()
    ));
    b.push_str(&deeper(&t.id));
    Notice {
        kind: NOTICE_ASK.into(),
        task_id: t.id.clone(),
        reference: format!("task:{}", t.id),
        title: t.title.clone(),
        by: t.created_by.clone(),
        to: to.to_vec(),
        wake: true,
        text: b,
        person: true,
        ..Notice::default()
    }
}

/// What the idle agent a thread is handed to hears.
pub(crate) fn thread_hand_notice(t: &Task, to: &str, told: &[String]) -> Notice {
    let mut b = format!(
        "The person started a thread ({}), and Irori handed it to you. It's a question or a conversation, not a card:\n{}\n",
        t.refs(),
        cut(&t.objective, MAX_THREAD_NOTICE)
    );
    let others: Vec<&String> = told.iter().filter(|k| *k != to).collect();
    b.push_str(&format!(
        "Answer it in the thread with board_comment {}: look things up (board, show, search) as needed.",
        t.short_ref()
    ));
    if !others.is_empty() {
        b.push_str(&format!(
            " {} heard it first: read the thread before you answer, in case one already did.",
            others
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    b.push_str(&format!(
        " {NOT_YOURS_TO_CLOSE} If you can't answer it now, board_leave {} and it goes to the next idle agent; ending your turn without answering does the same.",
        t.short_ref()
    ));
    b.push_str(&deeper(&t.id));
    Notice {
        kind: NOTICE_ASK.into(),
        task_id: t.id.clone(),
        reference: format!("task:{}", t.id),
        title: t.title.clone(),
        by: t.created_by.clone(),
        to: vec![to.into()],
        wake: true,
        text: b,
        person: true,
        ..Notice::default()
    }
}
