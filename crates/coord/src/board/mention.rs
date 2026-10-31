//! Mentions on a card (card #77; board.md "Mentions"): the mentioned
//! agents become watchers and get their own woken notice.

use store::kinds;

use super::error::{Result, invalid};
use super::fold::Fold;
use super::rows::TaskWatchedRow;
use super::types::*;
use super::util::{SWEEPER, check_key, contains, cut, deeper};
use crate::names::trim_space;

/// How much of the text a mention notice carries.
const MAX_MENTION_TEXT: usize = 1200;

/// Takes the keys of `ms` out of `n.to`: a mentioned agent hears only its
/// mention notice.
pub fn without_mentioned(n: &mut Notice, ms: &[Mention]) {
    n.to.retain(|k| !ms.iter().any(|m| m.key == *k));
}

impl Fold {
    /// Checks and cleans the mentions `by` gives: at most MAX_MENTIONS,
    /// deduped by key, without the writer, people and Irori.
    pub(crate) fn check_mentions(&self, by: &str, ms: &[Mention]) -> Result<Vec<Mention>> {
        let mut out: Vec<Mention> = Vec::new();
        for m in ms {
            let key = trim_space(&m.key).to_string();
            if key.is_empty()
                || key == by
                || key == SWEEPER
                || key.starts_with("user:")
                || self.is_owner(&key)
                || out.iter().any(|o| o.key == key)
            {
                continue;
            }
            check_key(&key)?;
            out.push(Mention {
                key,
                name: trim_space(&m.name).to_string(),
                gone: m.gone,
            });
        }
        if out.len() > MAX_MENTIONS {
            return Err(invalid(format!("mention at most {MAX_MENTIONS} agents")));
        }
        Ok(out)
    }

    /// Makes `who` watchers of card `id` as `by` (task.watched with `by` the
    /// writer); holders and watchers already on it are skipped.
    pub(crate) fn watch_for(&mut self, by: &str, id: &str, who: &[String]) -> Result<Task> {
        let by = super::util::caller(by)?;
        let t = self.any_task(id)?.clone();
        let actor = self.actor_for(&by);
        let mut done: Vec<String> = Vec::new();
        for w in who {
            let w = trim_space(w);
            if w.is_empty()
                || w == by
                || w == SWEEPER
                || self.is_owner(w)
                || contains(&t.holders, w)
                || contains(&t.watchers, w)
                || contains(&done, w)
            {
                continue;
            }
            check_key(w)?;
            let at = self.stamp();
            self.write_as(
                &actor,
                kinds::TASK_WATCHED,
                &TaskWatchedRow {
                    task_id: id.into(),
                    who: w.into(),
                    watching: true,
                    by: by.clone(),
                    at,
                },
            )?;
            done.push(w.to_string());
        }
        Ok(self.tasks[id].clone())
    }

    /// The live mentions in `ms` made watchers of card `id`, and their
    /// woken notice (`None`: nobody to tell). `text` is what was written;
    /// `reference` the notice's ref.
    pub(crate) fn mention_notice(
        &mut self,
        by: &str,
        id: &str,
        ms: &[Mention],
        text: &str,
        reference: &str,
    ) -> Result<Option<Notice>> {
        let to: Vec<String> = ms
            .iter()
            .filter(|m| !m.gone)
            .map(|m| m.key.clone())
            .collect();
        if to.is_empty() {
            return Ok(None);
        }
        let t = self.watch_for(by, id, &to)?;
        let who = if self.is_owner(by) {
            "The person".to_string()
        } else {
            by.to_string()
        };
        let mut s = format!(
            "{who} mentioned you on {} ({}):\n{}\n",
            gocompat::quote::quote(&t.title),
            t.refs(),
            cut(trim_space(text), MAX_MENTION_TEXT)
        );
        s.push_str(&mention_ending(&t, |k| self.is_owner(k)));
        s.push_str(&deeper(id));
        Ok(Some(Notice {
            kind: NOTICE_MENTIONED.into(),
            task_id: id.into(),
            reference: reference.into(),
            title: t.title.clone(),
            to,
            by: by.into(),
            text: s,
            wake: true,
            person: self.is_owner(by),
            ..Notice::default()
        }))
    }
}

/// The last line of a mention notice, by the card's state.
pub(crate) fn mention_ending(t: &Task, is_owner: impl Fn(&str) -> bool) -> String {
    if t.open_ask().is_some_and(|a| a.taker.is_empty()) && !is_terminal(&t.state) {
        return format!(
            "It is an ask nobody has taken: if you can take it on now, ask_claim {}.",
            t.short_ref()
        );
    }
    if is_terminal(&t.state) {
        return format!(
            "It is {}; answer on the card with board_comment if it asks you.",
            t.state
        );
    }
    let holders: Vec<&str> = t
        .holders
        .iter()
        .filter(|h| !is_owner(h) && *h != SWEEPER)
        .map(String::as_str)
        .collect();
    if holders.is_empty() {
        return "Nobody holds it. If the work is yours, board_join it; else answer on the card with board_comment.".into();
    }
    let verb = if holders.len() == 1 { "holds" } else { "hold" };
    format!(
        "{} {verb} it; answer on the card with board_comment if it asks you.",
        holders.join(", ")
    )
}
