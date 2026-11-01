//! Threads (card #116): the header box starts a conversation with the
//! room, shown in the Threads panel beside the Board, not in the grid.
//! The agents on the cards it names (#12) and the agents it @mentions hear
//! it first; after a quiet spell one idle agent gets it (coord's threads).
//! The person replies with the card comment box (BoardService.Comment) and
//! resolves it here. A shared board's page starts its threads on that
//! board (card #134): they live in its log, only its members' agents hear
//! them, and its page lists them in its own Threads panel.

use coord::{NewAsk, Task};
use gocompat::time::GoTime;
use serde::Serialize;
use specta::Type;

use crate::Window;
use crate::board::{TaskResult, Told};
use crate::cards::AgentRef;
use crate::ts;

/// Where a thread stands, for the panel.
pub const THREAD_WAITING: &str = "waiting";
pub const THREAD_HANDED: &str = "handed";
pub const THREAD_ANSWERED: &str = "answered";
pub const THREAD_RESOLVED: &str = "resolved";

/// How many resolved threads the panel lists.
const RESOLVED_SHOWN: usize = 20;

/// Whether `t` is the person's thread.
pub fn is_thread(t: &Task) -> bool {
    t.ask.as_ref().is_some_and(|a| a.thread)
}

/// One thread in the panel.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
    pub id: String,
    pub short: String,
    pub title: String,
    /// What the person asked, in full.
    pub text: String,
    /// waiting (for an answer), handed (to one idle agent), answered or
    /// resolved.
    pub state: String,
    /// Who heard it first, and who it was handed to.
    pub told: Vec<AgentRef>,
    /// The agents that answered.
    pub answered_by: Vec<AgentRef>,
    pub replies: i64,
    /// The newest reply, by anyone.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub last_by: Option<AgentRef>,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[specta(optional)]
    pub last_text: String,
    #[specta(type = ts::Time)]
    pub at: GoTime,
    #[specta(type = ts::Time)]
    pub updated_at: GoTime,
}

impl Window {
    /// BoardService.StartThread: the header box's message, as a thread;
    /// with `board_id`, on that shared board (no mentions there).
    pub fn start_thread(
        &self,
        text: &str,
        mentions: &[ts::Mention],
        board_id: &str,
    ) -> Result<TaskResult, String> {
        let board_id = crate::trim(board_id);
        let (t, n) = if board_id.is_empty() {
            let plan = self.plan_mentions("", mentions);
            self.core
                .coord()
                .post_ask(
                    &self.user_key(),
                    NewAsk {
                        text: text.into(),
                        mentions: plan.log.clone(),
                        thread: true,
                        ..Default::default()
                    },
                    &self.core.ask_agents(),
                )
                .map_err(|e| e.to_string())?
        } else {
            if !mentions.is_empty() {
                return Err(
                    "mentions work in threads on your own board; on a shared board, name a card (#12)"
                        .into(),
                );
            }
            self.core
                .board_ask(
                    board_id,
                    NewAsk {
                        text: text.into(),
                        thread: true,
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?
        };
        let mut told = Told::default();
        if !n.to.is_empty() {
            told.to = n.to.iter().map(|k| self.core.private_key(k)).collect();
            told.mode = inbox::mode::WAKE.into();
            let from = if n.person {
                self.core.user_sender()
            } else {
                inbox::system_sender()
            };
            if let Err(e) = self.core.route_notices(&from, std::slice::from_ref(&n)) {
                told.error = e;
            }
        }
        Ok(TaskResult {
            task: t,
            told,
            ..TaskResult::default()
        })
    }

    /// BoardService.Threads: open threads, newest first, then the newest
    /// resolved ones: your own board's, or with `board_id` that shared
    /// board's.
    pub fn threads(&self, board_id: &str) -> Vec<ThreadSummary> {
        let board_id = crate::trim(board_id);
        let co = if board_id.is_empty() {
            self.core.coord().clone()
        } else {
            match self.core.board_coord(board_id) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            }
        };
        let p = self.people();
        let mut open = Vec::new();
        let mut done = Vec::new();
        for t in co.list_tasks(&coord::TaskFilter::default()) {
            if !is_thread(&t) || t.withdrawn {
                continue;
            }
            let raw = co.comments(&t.id).unwrap_or_default();
            // People, not agents: on a board, any member (card #134).
            let mut answered: Vec<String> = Vec::new();
            for c in &raw {
                let k = self.core.private_key(&c.by);
                if c.msg_id.is_empty() && !co.is_any_person(&c.by) && !answered.contains(&k) {
                    answered.push(k);
                }
            }
            let cs = self.priv_comments(raw);
            let t = self.priv_task(t);
            let a = t.ask.clone().unwrap_or_default();
            let resolved = a.outcome.is_some() || coord::is_terminal(&t.state);
            let state = if resolved {
                THREAD_RESOLVED
            } else if !answered.is_empty() || a.reason == coord::TAKE_ANSWERED {
                THREAD_ANSWERED
            } else if !a.taker.is_empty() {
                THREAD_HANDED
            } else {
                THREAD_WAITING
            };
            let mut told: Vec<String> = a.heard.iter().map(|k| self.core.private_key(k)).collect();
            let taker = self.core.private_key(&a.taker);
            if !taker.is_empty() && !told.contains(&taker) {
                told.push(taker);
            }
            let last = cs.iter().filter(|c| c.msg_id.is_empty()).max_by(|x, y| {
                x.at.unix_nano()
                    .cmp(&y.at.unix_nano())
                    .then_with(|| x.id.cmp(&y.id))
            });
            let s = ThreadSummary {
                id: t.id.clone(),
                short: t.short.clone(),
                title: t.title.clone(),
                text: t.objective.clone(),
                state: state.into(),
                told: p.refs(&told),
                answered_by: p.refs(&answered),
                replies: cs.iter().filter(|c| c.msg_id.is_empty()).count() as i64,
                last_by: last.map(|c| p.ref_(&c.by)),
                last_text: last.map(|c| c.text.clone()).unwrap_or_default(),
                at: t.created_at,
                updated_at: t.updated_at,
            };
            if resolved {
                done.push(s);
            } else {
                open.push(s);
            }
        }
        let newest = |a: &ThreadSummary, b: &ThreadSummary| {
            b.updated_at.unix_nano().cmp(&a.updated_at.unix_nano())
        };
        open.sort_by(newest);
        done.sort_by(newest);
        done.truncate(RESOLVED_SHOWN);
        open.extend(done);
        open
    }

    /// BoardService.ResolveThread: the person is done with it.
    pub fn resolve_thread(&self, id: &str) -> Result<(), String> {
        let id = crate::trim(id);
        let (co, keys) = self.on(id, &[])?;
        co.resolve_thread(&keys[0], id)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
