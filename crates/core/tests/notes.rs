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
    // Alone on the board there is no device to wait on (#411, #392): a
    // board note goes in the board log only.
    let shared = c
        .propose_note(&me, "mlg", nn("Ship models to the repo.", &["everywhere"]))
        .unwrap()
        .note;
    assert_eq!(
        note_rows(&c.board_store(&b).unwrap()),
        vec![shared.id.clone()]
    );
    assert_eq!(note_rows(c.store()), vec![mine.note.id.clone()]);

    let all = c.notes("", false).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(c.notes("mine", false).unwrap().len(), 1);
    assert_eq!(c.notes("mlg", false).unwrap()[0].board_id, b);
    assert_eq!(c.search_notes("repo", true).len(), 1);
    // The person removes it from the board; an unknown scope is refused.
    c.remove_note(&me, &shared.id, "").unwrap();
    assert!(c.notes("mlg", false).unwrap().is_empty());
    assert!(
        c.propose_note(&me, "nope", nn("x", &["everywhere"]))
            .is_err()
    );
}
