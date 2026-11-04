//! Several agents on one card (docs/design/spawned-agents.md §3, part 7)
//! on the private board: a helper's part goes to the lead, and a card
//! split in three asks the person once.

mod common;

use common::*;
use coord::*;

const LEAD: &str = "dev-a/lead-session";
const HELPERS: [&str; 3] = ["dev-a/helper-1", "dev-a/helper-2", "dev-a/helper-3"];

fn env() -> Env {
    let mut e = Env::new();
    e.c = e.reopen_with(Options {
        done_check: true,
        ..e.opts()
    });
    e
}

fn sub(title: &str, spawn: bool) -> NewSubtask {
    NewSubtask {
        title: title.into(),
        spawn,
        ..NewSubtask::default()
    }
}

fn done_checks(c: &Coord, id: &str) -> usize {
    c.get_task(id)
        .unwrap()
        .blockers
        .iter()
        .filter(|b| b.done_check)
        .count()
}

/// A card the lead holds, split into `subs`, each claimed by a helper.
fn led(e: &Env, subs: Vec<NewSubtask>) -> (Task, Vec<Task>) {
    let p =
        e.c.create_task(
            USER,
            NewTask {
                title: "big card".into(),
                ..NewTask::default()
            },
        )
        .unwrap();
    e.c.claim_task(LEAD, &p.id).unwrap();
    let (p, made) = e.c.split(LEAD, &p.id, subs, vec![]).unwrap();
    for (k, h) in made.iter().zip(HELPERS) {
        e.c.claim_task(h, &k.id).unwrap();
    }
    (p, made)
}

#[test]
fn a_card_split_in_three_raises_one_done_check() {
    let e = env();
    let (p, made) = led(
        &e,
        vec![sub("tests", true), sub("ui", true), sub("docs", true)],
    );
    assert!(made.iter().all(|k| k.spawn), "each part is marked");
    for (k, h) in made.iter().zip(HELPERS) {
        // The helper says done: a question for the lead, not the person.
        let got =
            e.c.set_task_state(h, &k.id, COMPLETED, "merged-ready @abc")
                .unwrap();
        assert_eq!(got.state, BLOCKED);
        let b = got.blockers.iter().find(|b| b.open()).unwrap();
        assert_eq!(b.question, PART_QUESTION);
        assert!(!b.done_check);
        assert_eq!(b.context, "merged-ready @abc");
        let on = b.on.as_ref().unwrap();
        assert_eq!(
            (on.kind.as_str(), on.reference.as_str()),
            (BLOCKED_ON_AGENT, LEAD)
        );
        // Saying it again replaces the question.
        let again = e.c.set_task_state(h, &k.id, COMPLETED, "").unwrap();
        assert_eq!(again.blockers.iter().filter(|b| b.open()).count(), 1);
        // The lead accepts: completed, with no done check.
        let ok =
            e.c.set_task_state(LEAD, &k.id, COMPLETED, "merged")
                .unwrap();
        assert_eq!(ok.state, COMPLETED);
        assert_eq!(done_checks(&e.c, &k.id), 0);
    }
    // Only the parent asks the person.
    let got =
        e.c.set_task_state(LEAD, &p.id, COMPLETED, "all three merged")
            .unwrap();
    assert_eq!(got.state, BLOCKED);
    let all: usize = std::iter::once(&p.id)
        .chain(made.iter().map(|k| &k.id))
        .map(|id| done_checks(&e.c, id))
        .sum();
    assert_eq!(all, 1);
    // A replay agrees.
    let c2 = e.reopen();
    assert_eq!(done_checks(&c2, &p.id), 1);
    assert!(made.iter().all(|k| c2.get_task(&k.id).unwrap().spawn));
}

#[test]
fn an_unmarked_subtask_still_asks_the_person() {
    let e = env();
    let (_, made) = led(&e, vec![sub("plain", false)]);
    assert!(!made[0].spawn);
    let got =
        e.c.set_task_state(HELPERS[0], &made[0].id, COMPLETED, "done")
            .unwrap();
    assert_eq!(done_checks(&e.c, &got.id), 1);
}

#[test]
fn a_part_with_no_lead_asks_the_person() {
    let e = env();
    let (p, made) = led(&e, vec![sub("tests", true)]);
    e.c.leave(LEAD, &p.id, LEAD).unwrap();
    e.c.set_task_state(HELPERS[0], &made[0].id, COMPLETED, "done")
        .unwrap();
    assert_eq!(done_checks(&e.c, &made[0].id), 1);
}

#[test]
fn a_marked_step_moves_its_mark_to_the_subtask_and_marks_stay() {
    let e = env();
    let t =
        e.c.create_task(
            USER,
            NewTask {
                title: "card".into(),
                ..NewTask::default()
            },
        )
        .unwrap();
    e.c.claim_task(LEAD, &t.id).unwrap();
    let st = |text: &str, spawn: bool| Step {
        text: text.into(),
        spawn,
        ..Step::default()
    };
    let got =
        e.c.set_steps(LEAD, &t.id, vec![st("plan", false), st("tests", true)])
            .unwrap();
    assert!(!got.steps[0].spawn && got.steps[1].spawn);
    // Redone without the mark: it stays.
    let got =
        e.c.set_steps(LEAD, &t.id, vec![st("plan", false), st("tests", false)])
            .unwrap();
    assert!(got.steps[1].spawn);
    let (_, made) =
        e.c.split(
            LEAD,
            &t.id,
            vec![
                NewSubtask {
                    title: "plan".into(),
                    steps: vec!["s1".into()],
                    ..NewSubtask::default()
                },
                NewSubtask {
                    title: "tests".into(),
                    steps: vec!["s2".into()],
                    ..NewSubtask::default()
                },
            ],
            vec![],
        )
        .unwrap();
    assert!(!made[0].spawn);
    assert!(made[1].spawn);
}

#[test]
fn an_unmarked_subtask_of_a_led_card_is_not_spawned() {
    use std::sync::Arc;
    let mut e = env();
    e.c = e.reopen_with(Options {
        done_check: true,
        spawning: Some(Arc::new(|| true)),
        ..e.opts()
    });
    let p =
        e.c.create_task(
            USER,
            NewTask {
                title: "big card".into(),
                ..NewTask::default()
            },
        )
        .unwrap();
    e.c.claim_task(LEAD, &p.id).unwrap();
    let (_, made) =
        e.c.split(
            LEAD,
            &p.id,
            vec![sub("plain", false), sub("helper", true)],
            vec![],
        )
        .unwrap();
    let listed = ids(&e.c.cards_to_spawn());
    assert!(
        !listed.contains(&made[0].id),
        "unmarked: the lead's to give"
    );
    assert!(listed.contains(&made[1].id), "marked: a helper is spawned");
    // With no lead, an unmarked subtask needs its own agent again.
    e.c.leave(LEAD, &p.id, LEAD).unwrap();
    assert!(ids(&e.c.cards_to_spawn()).contains(&made[0].id));
}
