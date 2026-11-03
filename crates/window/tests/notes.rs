//! The Notes page's commands (card #411, docs/design/notes.md §11).

mod common;

use common::*;
use irori_window::notes::{NoteEditInput, NoteInput};

fn input(scope: &str, text: &str) -> NoteInput {
    NoteInput {
        scope: scope.into(),
        kind: "rule".into(),
        text: text.into(),
        tags: vec!["everywhere".into()],
        ..NoteInput::default()
    }
}

#[test]
fn notes_in_the_window() {
    let r = Rig::new();
    let w = &r.w;

    // An agent proposes a Mine note: it waits, from an agent.
    let p = w
        .core
        .propose_note(
            &r.key_a,
            "mine",
            coord::NewNote {
                kind: "gotcha".into(),
                text: "Load 20 makes presence tests time out.".into(),
                tags: vec!["topic:tests".into()],
                ..Default::default()
            },
        )
        .unwrap();
    let list = w.notes_list("", false).unwrap();
    assert_eq!(list.len(), 1);
    let v = &list[0];
    assert_eq!((v.state.as_str(), v.scope.as_str()), ("proposed", "mine"));
    assert!(!v.by_person, "{v:?}");
    assert_eq!(v.stale, "");

    // Keep: live, accepted by the person.
    let k = w.note_accept(&format!("note:{}", p.note.id)).unwrap();
    assert_eq!(k.state, "accepted");
    assert_eq!(k.accepted_by, w.core.user_sender().key);

    // The agent proposes an edit: shown as pending; the person's edit
    // applies at once and drops it.
    w.core
        .edit_note(
            &r.key_a,
            &p.note.id,
            coord::NoteEdit {
                text: Some("Load 20 times presence tests out.".into()),
                ..Default::default()
            },
            "shorter",
        )
        .unwrap();
    let v = &w.notes_list("mine", false).unwrap()[0];
    let pend = v.pending.as_ref().expect("pending edit");
    assert_eq!((pend.why.as_str(), pend.remove), ("shorter", false));
    let e = w
        .note_edit(
            &p.note.id,
            NoteEditInput {
                text: Some("Under load, rerun presence tests alone.".into()),
                ..NoteEditInput::default()
            },
        )
        .unwrap();
    assert!(e.pending.is_none());
    assert_eq!(e.text, "Under load, rerun presence tests alone.");

    // Keep as note from a card: live at once; once the card goes, it asks
    // Still true?
    let t = w
        .core
        .coord()
        .create_task(
            &w.core.user_sender().key,
            coord::NewTask {
                title: "the source".into(),
                ..Default::default()
            },
        )
        .unwrap();
    let mut kept = input("", "Answers on this card are decisions.");
    kept.task_id = t.id.clone();
    kept.source = format!("task:{}", t.id);
    let kept = w.note_new(kept).unwrap();
    assert_eq!(
        (kept.state.as_str(), kept.scope.as_str()),
        ("accepted", "mine")
    );
    assert!(kept.by_person);
    w.delete_card(&t.id).unwrap();
    let kept = w
        .notes_list("mine", false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == kept.id)
        .unwrap();
    assert_eq!(kept.stale, "its source card is gone");

    // Removed: gone from the list, kept in the history view.
    w.note_remove(&kept.id, "card gone").unwrap();
    assert_eq!(w.notes_list("mine", false).unwrap().len(), 1);
    assert_eq!(w.notes_list("mine", true).unwrap().len(), 2);

    // A board write waits for #392's gate, with a reason the page can show.
    w.boards_create("MLG", "mlg").unwrap();
    let err = w.note_new(input("mlg", "Ship to the repo.")).unwrap_err();
    assert!(err.contains("keep it in Mine"), "{err}");
    assert!(w.notes_list("mlg", false).unwrap().is_empty());
}
