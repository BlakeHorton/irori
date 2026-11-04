//! Where a note lives (decision 45, card #401, docs/design/notes.md §3):
//! a Mine note in the person's private log, a board note in that board's
//! log, never the other way round. Tools and the window reach the log
//! through these, as they reach a card's through `task_coord`.

use std::sync::Arc;

use coord::{Coord, ErrorKind as CK};

use super::Services;
use super::boards::{Error, trim};

/// The scope name of the person's own notes.
pub const SCOPE_MINE: &str = "mine";

/// One log notes can live in: Mine, or a shared board.
#[derive(Clone)]
pub struct NoteLog {
    /// `""` for Mine, else the board's id.
    pub board_id: String,
    /// `"mine"`, or the board's tag.
    pub scope: String,
    pub coord: Arc<Coord>,
}

impl Services {
    /// Whether this device may write note rows into board `board_id`'s log
    /// (#411). An older Irori refuses an unknown kind, and every later row
    /// of that writer's stream then fails its gap check there, so one
    /// note.* row would stall this device's whole stream on any member
    /// device that predates notes (nutmeg, #392). The note kinds landed
    /// before the board hello's `spaces` field (0f6544ab, then 9d5a72dc),
    /// so a device heard to read space rows reads note rows too: the
    /// space gate ([`Services::space_not_ready`]) is the notes gate. Mine
    /// notes never wait.
    pub(crate) fn board_notes_ready(
        &self,
        board_id: &str,
    ) -> std::result::Result<(), coord::Error> {
        let waiting = self.space_not_ready(board_id).map_err(Error::into_coord)?;
        if waiting.is_empty() {
            return Ok(());
        }
        Err(Error::coord(
            CK::NotAllowed,
            format_args!(
                "every device on the board must run an Irori with notes first; waiting on {}. Keep it in Mine for now",
                waiting.join(", ")
            ),
        )
        .into_coord())
    }

    /// The log for `scope` (`""` or `mine`: the private log; else a board's
    /// tag or id), with each of `keys` as that log knows the caller.
    pub(crate) fn notes_log(
        self: &Arc<Self>,
        scope: &str,
        keys: &[String],
    ) -> std::result::Result<(NoteLog, Vec<String>), coord::Error> {
        let scope = trim(scope);
        if scope.is_empty() || scope.eq_ignore_ascii_case(SCOPE_MINE) {
            return Ok((
                NoteLog {
                    board_id: String::new(),
                    scope: SCOPE_MINE.into(),
                    coord: self.coord.clone(),
                },
                keys.to_vec(),
            ));
        }
        let boards = self.shown_boards();
        let hits: Vec<_> = boards
            .iter()
            .filter(|b| b.id == scope || b.verifier.membership().tag.eq_ignore_ascii_case(scope))
            .cloned()
            .collect();
        let b = match hits.as_slice() {
            [b] => b.clone(),
            [] => {
                return Err(Error::coord(
                    CK::NotFound,
                    format_args!("no board {scope:?}: give mine, or a board's tag or id"),
                )
                .into_coord());
            }
            _ => {
                return Err(Error::coord(
                    CK::Ambiguous,
                    format_args!("two boards are tagged {scope:?}: give its id"),
                )
                .into_coord());
            }
        };
        let out = keys
            .iter()
            .map(|k| self.board_key(&b.id, k))
            .collect::<super::boards::Result<Vec<_>>>()
            .map_err(Error::into_coord)?;
        Ok((
            NoteLog {
                board_id: b.id.clone(),
                scope: b.verifier.membership().tag.clone(),
                coord: b.coord().clone(),
            },
            out,
        ))
    }

    /// [`Services::board_notes_ready`] for a board log; Mine always passes.
    pub(crate) fn write_gate(&self, l: &NoteLog) -> std::result::Result<(), coord::Error> {
        if l.board_id.is_empty() {
            return Ok(());
        }
        self.board_notes_ready(&l.board_id)
    }

    /// Every log this device reads notes from: Mine first, then each shown
    /// board.
    pub(crate) fn note_logs(&self) -> Vec<NoteLog> {
        let mut out = vec![NoteLog {
            board_id: String::new(),
            scope: SCOPE_MINE.into(),
            coord: self.coord.clone(),
        }];
        for b in self.shown_boards() {
            out.push(NoteLog {
                board_id: b.id.clone(),
                scope: b.verifier.membership().tag.clone(),
                coord: b.coord().clone(),
            });
        }
        out
    }

    /// The log holding note `id`, with `keys` as it knows them.
    pub(crate) fn note_log(
        self: &Arc<Self>,
        id: &str,
        keys: &[String],
    ) -> std::result::Result<(NoteLog, Vec<String>), coord::Error> {
        let id = trim(id.strip_prefix("note:").unwrap_or(id)).to_string();
        let Some(l) = self
            .note_logs()
            .into_iter()
            .find(|l| l.coord.note_by_id(&id).is_some())
        else {
            return Err(Error::coord(CK::NotFound, format_args!("note {id}")).into_coord());
        };
        if l.board_id.is_empty() {
            return Ok((l, keys.to_vec()));
        }
        let out = keys
            .iter()
            .map(|k| self.board_key(&l.board_id, k))
            .collect::<super::boards::Result<Vec<_>>>()
            .map_err(Error::into_coord)?;
        Ok((l, out))
    }
}

// ---- the Core's notes API (card #401) ---------------------------------------

/// A note with the log it lives in.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopedNote {
    /// `"mine"`, or the board's tag.
    pub scope: String,
    /// `""` for Mine.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub board_id: String,
    #[serde(flatten)]
    pub note: coord::Note,
}

fn scoped(l: &NoteLog, n: coord::Note) -> ScopedNote {
    ScopedNote {
        scope: l.scope.clone(),
        board_id: l.board_id.clone(),
        note: n,
    }
}

impl super::Core {
    /// Writes a note in `scope` (`mine`, or a board's tag or id) as `by`,
    /// the person at this device or one of their agents. A Mine note stays
    /// in the private log whatever its tags name.
    pub fn propose_note(
        &self,
        by: &str,
        scope: &str,
        n: coord::NewNote,
    ) -> std::result::Result<ScopedNote, coord::Error> {
        let (l, keys) = self.s.notes_log(scope, &[by.to_string()])?;
        self.s.write_gate(&l)?;
        let note = l.coord.propose_note(&keys[0], n)?;
        Ok(scoped(&l, note))
    }

    /// A person accepts note `id` (or an agent's pending change to it).
    pub fn accept_note(&self, by: &str, id: &str) -> std::result::Result<ScopedNote, coord::Error> {
        let (l, keys) = self.s.note_log(id, &[by.to_string()])?;
        self.s.write_gate(&l)?;
        let id = id.strip_prefix("note:").unwrap_or(id);
        Ok(scoped(&l, l.coord.accept_note(&keys[0], id)?))
    }

    pub fn edit_note(
        &self,
        by: &str,
        id: &str,
        e: coord::NoteEdit,
        why: &str,
    ) -> std::result::Result<ScopedNote, coord::Error> {
        let (l, keys) = self.s.note_log(id, &[by.to_string()])?;
        self.s.write_gate(&l)?;
        let id = id.strip_prefix("note:").unwrap_or(id);
        Ok(scoped(&l, l.coord.edit_note(&keys[0], id, e, why)?))
    }

    pub fn remove_note(
        &self,
        by: &str,
        id: &str,
        why: &str,
    ) -> std::result::Result<ScopedNote, coord::Error> {
        let (l, keys) = self.s.note_log(id, &[by.to_string()])?;
        self.s.write_gate(&l)?;
        let id = id.strip_prefix("note:").unwrap_or(id);
        Ok(scoped(&l, l.coord.remove_note(&keys[0], id, why)?))
    }

    /// The notes in `scope` (`""`: every log), newest first.
    pub fn notes(
        &self,
        scope: &str,
        removed: bool,
    ) -> std::result::Result<Vec<ScopedNote>, coord::Error> {
        let logs = if trim(scope).is_empty() {
            self.s.note_logs()
        } else {
            vec![self.s.notes_log(scope, &[])?.0]
        };
        Ok(logs
            .iter()
            .flat_map(|l| l.coord.notes(removed).into_iter().map(|n| scoped(l, n)))
            .collect())
    }

    /// Notes holding every word of `q` across every log this device reads;
    /// `live_only`: accepted only, what agents may read.
    pub fn search_notes(&self, q: &str, live_only: bool) -> Vec<ScopedNote> {
        self.s
            .note_logs()
            .iter()
            .flat_map(|l| {
                l.coord
                    .search_notes(q, live_only)
                    .into_iter()
                    .map(|n| scoped(l, n))
            })
            .collect()
    }
}
