//! Card positions on the Board's canvas (card #93): where a card sits when
//! the Board is shown as a canvas. One `task.placed` row per drop; the
//! newest wins (rows fold in HLC order), and it replicates like any card
//! row, so every device of the person (and a shared board's members) sees
//! the same layout. It is not activity: no watch digest, no updated time.

use std::collections::HashMap;

use store::{Event, kinds};

use super::error::{Result, invalid};
use super::fold::Fold;
use super::rows::*;
use super::util::{caller, check_key};

/// How far from the origin a card may be placed, either way.
pub const MAX_PLACE: i64 = 1_000_000;

impl Fold {
    /// Records where `by` dropped card `id` on the canvas.
    pub(crate) fn place(&mut self, by: &str, id: &str, x: i64, y: i64) -> Result<()> {
        let by = caller(by)?;
        if !self.is_owner(&by) {
            check_key(&by)?;
        }
        self.any_task(id)?;
        if x.abs() > MAX_PLACE || y.abs() > MAX_PLACE {
            return Err(invalid(format!("position {x},{y} is off the canvas")));
        }
        if self.placed.get(id) == Some(&(x, y)) {
            return Ok(());
        }
        let at = self.stamp();
        self.write(
            &by,
            kinds::TASK_PLACED,
            &TaskPlacedRow {
                task_id: id.into(),
                x,
                y,
                by: by.clone(),
                at,
            },
        )
    }

    /// Every placed card's position, by card id.
    pub(crate) fn positions(&self) -> HashMap<String, (i64, i64)> {
        self.placed.clone()
    }

    pub(crate) fn fold_placed(&mut self, ev: &Event) {
        let Some(p) = row::<TaskPlacedRow>(&ev.payload) else {
            return;
        };
        if p.task_id.is_empty() || self.deleted.contains(&p.task_id) {
            return;
        }
        self.placed.insert(p.task_id.clone(), (p.x, p.y));
        self.note(super::types::Change {
            kind: super::types::TASK_CHANGED.into(),
            task_id: p.task_id,
            ..super::types::Change::default()
        });
    }
}
