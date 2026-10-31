//! New card (card #116): the person makes a card straight from the app,
//! without an ask. It goes up unheld and Irori hands it to one free agent
//! (pull, not push: card #82), or to the agent the person picked.

use coord::NewTask;
use serde::Deserialize;
use specta::Type;

use crate::Window;
use crate::board::{TaskResult, Told};

/// What the New card form sends.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct NewCardInput {
    pub title: String,
    pub objective: String,
    pub done_when: String,
    /// The repo it belongs to ("": none).
    pub repo_key: String,
    /// An agent's key: hand it to that agent instead of the free one.
    pub assign_to: String,
}

impl Window {
    /// BoardService.Create: a card on the person's own board.
    pub fn create_card(&self, input: NewCardInput) -> Result<TaskResult, String> {
        let title = crate::trim(&input.title);
        if title.is_empty() {
            return Err(format!(
                "{}: give the card a title",
                coord::ErrorKind::Invalid.text()
            ));
        }
        let co = self.core.coord().clone();
        let me = self.core.user_sender().key;
        let t = co
            .create_task(
                &me,
                NewTask {
                    title: title.into(),
                    objective: crate::trim(&input.objective).into(),
                    done_criteria: crate::trim(&input.done_when).into(),
                    repo_key: crate::trim(&input.repo_key).into(),
                    ..NewTask::default()
                },
            )
            .map_err(|e| e.to_string())?;
        let to = crate::trim(&input.assign_to);
        if !to.is_empty() {
            let (t, mut n) = co.assign(&me, &t.id, to).map_err(|e| e.to_string())?;
            // The person picked this agent: it hears now, idle or not.
            n.wake = true;
            let told = self.tell_from(&self.core.user_sender(), &n);
            return Ok(TaskResult {
                task: t,
                told,
                ..TaskResult::default()
            });
        }
        let n = co
            .offer_card(&me, &t.id, &self.core.ask_agents())
            .map_err(|e| e.to_string())?;
        // From Irori, not the person: it hands the card out.
        let told = self.tell_from(&inbox::system_sender(), &n);
        let t = co.get_task(&t.id).unwrap_or(t);
        Ok(TaskResult {
            task: t,
            told,
            ..TaskResult::default()
        })
    }

    /// Routes `n` from `from`; nobody to tell is not an error.
    fn tell_from(&self, from: &inbox::Sender, n: &coord::Notice) -> Told {
        if n.to.is_empty() {
            return Told::default();
        }
        let mut told = Told {
            to: n.to.iter().map(|k| self.core.private_key(k)).collect(),
            mode: if n.wake {
                inbox::mode::WAKE.into()
            } else {
                inbox::mode::NEXT_TURN.into()
            },
            error: String::new(),
        };
        if let Err(e) = self.core.route_notices(from, std::slice::from_ref(n)) {
            told.error = e;
        }
        told
    }
}
