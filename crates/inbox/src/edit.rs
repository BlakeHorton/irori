//! Holding the person's comment while they edit it (card #105).
//!
//! A comment on a card reaches its agents as notices whose ref is
//! `comment:<id>`. Until an agent's turn takes one, it sits queued, and the
//! person may take it back to edit: Modify holds every one of them
//! ([`hold::EDITING`]), Save drops the held ones (the caller sends the new
//! text), Cancel queues them again. Once any of them was handed to an
//! agent, the comment is read and can't be edited. An editing hold is the
//! person's own, like Pause all: it is never asked about.

use std::collections::HashMap;

use crate::messages::dismissed;
use crate::policy::placed;
use crate::state::{Drec, Inner};
use crate::{Delivery, Error, ErrorKind, Inbox, Result, hold, st};

/// Where the person's comment stands with its agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditState {
    /// Every notice of it still waits for its agent's turn (or it had none).
    Unread,
    /// The person holds it back while they edit.
    Editing,
    /// An agent has it (handed over, read or answered): it stays as it is.
    Read,
}

/// Held while the person edits it.
pub fn editing(d: &Delivery) -> bool {
    d.state == st::HELD && d.hold == hold::EDITING
}

fn rec_editing(d: &Drec) -> bool {
    d.d.state == st::HELD && d.d.hold == hold::EDITING
}

/// Left behind by an edit: dropped when the new text went out.
fn rec_replaced(d: &Drec) -> bool {
    d.d.state == st::DISMISSED && d.d.hold == hold::EDITING
}

/// Not handed to the agent yet: queued, no wake issued for it.
fn rec_waiting(d: &Drec) -> bool {
    d.d.state == st::QUEUED && d.d.wake_id.is_empty()
}

fn read_err(r: &str) -> Error {
    Error::wrap(
        ErrorKind::BadState,
        format!("an agent already has {r}: it can't be edited now"),
    )
}

impl Inner {
    /// Ids of the notices `from` sent with ref `r`, oldest first.
    fn edit_ids(&self, from: &str, r: &str) -> Vec<String> {
        self.order
            .iter()
            .filter(|id| {
                self.dels.get(*id).is_some_and(|d| {
                    self.msgs
                        .get(&d.d.message_id)
                        .is_some_and(|m| m.notice && m.ref_ == r && m.from.key == from)
                })
            })
            .cloned()
            .collect()
    }

    fn edit_state_of(&self, ids: &[String]) -> EditState {
        let mut editing = false;
        for id in ids {
            let d = &self.dels[id];
            if rec_replaced(d) {
                continue;
            }
            if rec_editing(d) {
                editing = true;
            } else if !rec_waiting(d) {
                return EditState::Read;
            }
        }
        if editing {
            EditState::Editing
        } else {
            EditState::Unread
        }
    }
}

impl Inbox {
    /// Where the notices `from` sent with ref `r` stand.
    pub fn edit_state(&self, from: &str, r: &str) -> EditState {
        self.read(|b| b.edit_state_of(&b.edit_ids(from, r)))
    }

    /// [`Inbox::edit_state`] for every `comment:` ref `from` sent notices
    /// with, by comment id. A comment with no notices isn't listed.
    pub fn comment_edit_states(&self, from: &str) -> HashMap<String, EditState> {
        self.read(|b| {
            let mut ids: HashMap<String, Vec<String>> = HashMap::new();
            for id in &b.order {
                let d = &b.dels[id];
                let Some(m) = b.msgs.get(&d.d.message_id) else {
                    continue;
                };
                if !m.notice || m.from.key != from {
                    continue;
                }
                if let Some(cid) = m.ref_.strip_prefix("comment:") {
                    ids.entry(cid.to_string()).or_default().push(id.clone());
                }
            }
            ids.into_iter()
                .map(|(cid, ds)| (cid, b.edit_state_of(&ds)))
                .collect()
        })
    }

    /// Modify: holds every waiting notice `from` sent with ref `r`, so no
    /// agent gets it while the person edits. Refused once an agent has one,
    /// or when one is for an agent on another device. Returns how many it
    /// holds now.
    pub fn hold_for_edit(&self, from: &str, r: &str, by: &str) -> Result<usize> {
        self.mutate(|b| {
            let ids = b.edit_ids(from, r);
            if b.edit_state_of(&ids) == EditState::Read {
                return Err(read_err(r));
            }
            let mut n = 0;
            for id in &ids {
                let d = &b.dels[id];
                if rec_replaced(d) {
                    continue;
                }
                if rec_editing(d) {
                    n += 1;
                    continue;
                }
                if !b.owns(d) {
                    return Err(Error::wrap(
                        ErrorKind::BadState,
                        format!("{r} waits for an agent on another device: edit it there"),
                    ));
                }
            }
            for id in &ids {
                let d = &b.dels[id];
                if rec_waiting(d) {
                    let row = placed(d.row(), hold::EDITING);
                    b.put_delivery(row, by)?;
                    n += 1;
                }
            }
            Ok(n)
        })
    }

    /// Cancel: queues the notices held for editing again, unchanged.
    pub fn release_edit(&self, from: &str, r: &str, by: &str) -> Result<usize> {
        self.mutate(|b| {
            let ids: Vec<String> = b
                .edit_ids(from, r)
                .into_iter()
                .filter(|id| rec_editing(&b.dels[id]))
                .collect();
            for id in &ids {
                b.release_locked(id, by)?;
            }
            Ok(ids.len())
        })
    }

    /// Save: drops the notices held for editing and returns them (with
    /// their messages), for the caller to send again with the new text.
    pub fn take_edit(&self, from: &str, r: &str, by: &str) -> Result<Vec<Delivery>> {
        self.mutate(|b| {
            let ids: Vec<String> = b
                .edit_ids(from, r)
                .into_iter()
                .filter(|id| rec_editing(&b.dels[id]))
                .collect();
            let mut out = Vec::with_capacity(ids.len());
            for id in &ids {
                out.push(b.copy_out(&b.dels[id]));
                let row = dismissed(&b.dels[id]);
                b.put_delivery(row, by)?;
            }
            Ok(out)
        })
    }
}
