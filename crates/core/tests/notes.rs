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
    // A board note, by its tag.
    let shared = c
        .propose_note(&me, "mlg", nn("Ship models to the repo.", &["everywhere"]))
        .unwrap();
    assert_eq!(shared.board_id, b);

    let board_rows = note_rows(&c.board_store(&b).unwrap());
    let private_rows = note_rows(c.store());
    assert_eq!(board_rows, vec![shared.note.id.clone()]);
    assert!(
        !board_rows.contains(&mine.note.id),
        "a Mine note reached the board log"
    );
    assert!(private_rows.contains(&mine.note.id));
    assert!(!private_rows.contains(&shared.note.id));

    // The board knows the person by their board key, and both show in the list.
    assert!(
        shared
            .note
            .by
            .starts_with(store::kinds::PERSON_ACTOR_PREFIX)
    );
    let all = c.notes("", false).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(c.notes("mine", false).unwrap().len(), 1);
    assert_eq!(c.search_notes("repo", true).len(), 1);
    assert!(
        c.propose_note(&me, "nope", nn("x", &["everywhere"]))
            .is_err()
    );
}
