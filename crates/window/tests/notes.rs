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

    // Alone on a board nothing waits (#392's gate): a board note shows
    // under its board, made by the person, so it's live at once. (The
    // wait for another device is tested in core's boards.rs.)
    w.boards_create("MLG", "mlg").unwrap();
    let on_board = w.note_new(input("mlg", "Ship to the repo.")).unwrap();
    assert_eq!(on_board.scope, "mlg");
    assert_eq!(on_board.state, "accepted");
    assert_eq!(w.notes_list("mlg", false).unwrap().len(), 1);
}

#[test]
fn a_cards_wake_notes_reach_the_window() {
    let r = Rig::new();
    let w = &r.w;
    let me = w.core.user_sender().key;
    w.core
        .propose_note(
            &me,
            "mine",
            coord::NewNote {
                kind: "rule".into(),
                text: "Every wake hears this.".into(),
                tags: vec!["everywhere".into()],
                ..Default::default()
            },
        )
        .unwrap();
    let a = tools::Caller {
        sender: inbox::Sender::new(&r.key_b, "beta", "claude"),
        path: inbox::path::MOD.into(),
    };
    let out = w
        .core
        .tools()
        .call(
            &a,
            "board_create",
            &serde_json::to_vec(
                &serde_json::json!({"title": "Wake notes card", "assign_to": "alpha"}),
            )
            .unwrap(),
        )
        .unwrap();
    assert!(out.starts_with("Created #"), "{out}");
    let task = w
        .core
        .coord()
        .list_tasks(&coord::TaskFilter::default())
        .into_iter()
        .find(|t| t.title == "Wake notes card")
        .unwrap();
    assert!(w.card_wake_notes(&format!("task:{}", task.id)).is_empty());
    w.core
        .inbox()
        .next_wake(&r.key_a, inbox::path::MOD)
        .unwrap()
        .expect("alpha has the assignment");
    let got = w.card_wake_notes(&format!("task:{}", task.id));
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(
        (
            got[0].text.as_str(),
            got[0].tag.as_str(),
            got[0].agent.as_str()
        ),
        ("Every wake hears this.", "everywhere", r.key_a.as_str())
    );
    let v = serde_json::to_value(&got[0]).unwrap();
    assert!(v.get("noteId").is_some() && v.get("at").is_some(), "{v}");
}
