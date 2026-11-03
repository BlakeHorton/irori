//! Where notes live (card #401, docs/design/notes.md §3, §8): a Mine note
//! stays in the private log even when it names a shared board; a board
//! note goes in that board's log only.

mod common;

use common::Rig;
use store::Filter;

fn note_rows(st: &store::Store) -> Vec<String> {
    st.list(&Filter {
        kinds: vec!["note.".into()],
        limit: 1000,
        ..Default::default()
    })
    .unwrap()
    .iter()
    .map(|ev| {
        let v: serde_json::Value = serde_json::from_slice(&ev.payload).unwrap();
        v["noteId"].as_str().unwrap_or_default().to_string()
    })
    .collect()
}

fn nn(text: &str, tags: &[&str]) -> coord::NewNote {
    coord::NewNote {
        kind: "rule".into(),
        text: text.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        ..Default::default()
    }
}

#[test]
fn a_mine_note_never_enters_a_board_log() {
    let r = Rig::new();
    let c = &r.core;
    let b = c.create_board("MLG", "mlg").unwrap();
    let me = c.user_sender().key;

    // A Mine note about the board, written by the person and accepted at once.
    let mine = c
        .propose_note(&me, "mine", nn("Home IPs stay out of MLG.", &["board:mlg"]))
        .unwrap();
    assert_eq!((mine.scope.as_str(), mine.board_id.as_str()), ("mine", ""));
    assert_eq!(mine.note.state, coord::NOTE_ACCEPTED);
    // A board note waits until every member's Irori reads note rows
    // (#411, #392): refused with a reason, and nothing is written.
    let e = c
        .propose_note(&me, "mlg", nn("Ship models to the repo.", &["everywhere"]))
        .unwrap_err();
    assert!(e.is(coord::ErrorKind::NotAllowed), "{e}");
    assert!(e.to_string().contains("Mine"), "{e}");
    assert!(note_rows(&c.board_store(&b).unwrap()).is_empty());
    assert_eq!(note_rows(c.store()), vec![mine.note.id.clone()]);

    // Written straight to the board's coord (as the gate will allow), a
    // board note goes in the board log only.
    let person = format!("{}{}", store::kinds::PERSON_ACTOR_PREFIX, c.person().id);
    let shared = c
        .board_coord(&b)
        .unwrap()
        .propose_note(&person, nn("Ship models to the repo.", &["everywhere"]))
        .unwrap();
    assert_eq!(
        note_rows(&c.board_store(&b).unwrap()),
        vec![shared.id.clone()]
    );
    assert!(!note_rows(c.store()).contains(&shared.id));

    let all = c.notes("", false).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(c.notes("mine", false).unwrap().len(), 1);
    assert_eq!(c.notes("mlg", false).unwrap()[0].board_id, b);
    assert_eq!(c.search_notes("repo", true).len(), 1);
    // Removing it from the board waits too; an unknown scope is refused.
    assert!(c.remove_note(&me, &shared.id, "").is_err());
    assert!(
        c.propose_note(&me, "nope", nn("x", &["everywhere"]))
            .is_err()
    );
}
