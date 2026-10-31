//! Card #112: the person has the final say on done. With
//! `Options::done_check` on, an agent's completed raises a done check on
//! the person instead of closing the card.

mod common;

use common::*;
use coord::*;
use store::kinds;

fn env() -> Env {
    let mut e = Env::new();
    e.c = e.reopen_with(Options {
        done_check: true,
        ..e.opts()
    });
    e
}

fn claimed(e: &Env) -> Task {
    let t =
        e.c.create_task(
            ALICE,
            NewTask {
                title: "ship it".into(),
                ..NewTask::default()
            },
        )
        .unwrap();
    e.c.claim_task(BOB, &t.id).unwrap()
}

fn done_checks(t: &Task) -> Vec<&Blocker> {
    t.blockers
        .iter()
        .filter(|b| b.open() && b.done_check)
        .collect()
}

#[test]
fn an_agents_completed_waits_on_the_person() {
    let e = env();
    let t = claimed(&e);
    let got =
        e.c.set_task_state(BOB, &t.id, COMPLETED, "all green")
            .unwrap();
    assert_eq!(got.state, BLOCKED);
    let checks = done_checks(&got);
    assert_eq!(checks.len(), 1);
    let b = checks[0];
    assert_eq!(b.question, DONE_QUESTION);
    assert_eq!(b.context, "all green");
    assert_eq!(b.raised_by, BOB);
    assert_eq!(b.options.len(), 1);
    assert_eq!(b.options[0].answer, DONE_ANSWER);
    assert!(b.options[0].recommended);
    // Nothing wrote completed.
    let rows = e.rows(kinds::TASK_STATUS);
    assert!(
        rows.iter()
            .all(|r| !String::from_utf8_lossy(&r.payload).contains("\"completed\"")),
        "no completed row"
    );
    // A replay folds the same check.
    let again = e.reopen().get_task(&t.id).unwrap();
    assert_eq!(done_checks(&again).len(), 1);
}

#[test]
fn no_note_gives_the_default_context() {
    let e = env();
    let t = claimed(&e);
    let got = e.c.set_task_state(BOB, &t.id, COMPLETED, "").unwrap();
    assert_eq!(done_checks(&got)[0].context, DONE_CONTEXT);
}

#[test]
fn the_persons_completed_closes_directly() {
    let e = env();
    let t = claimed(&e);
    let got = e.c.set_task_state(USER, &t.id, COMPLETED, "").unwrap();
    assert_eq!(got.state, COMPLETED);
    assert!(got.blockers.is_empty());
}

#[test]
fn failed_and_canceled_stay_direct() {
    let e = env();
    let t = claimed(&e);
    let got = e.c.set_task_state(BOB, &t.id, FAILED, "broke").unwrap();
    assert_eq!(got.state, FAILED);
}

#[test]
fn done_then_completed_closes_it() {
    let e = env();
    let t = claimed(&e);
    let got = e.c.set_task_state(BOB, &t.id, COMPLETED, "").unwrap();
    let bid = done_checks(&got)[0].id.clone();
    let (after, _) =
        e.c.answer(
            USER,
            &t.id,
            &bid,
            BlockerAnswer {
                pick: 1,
                resolve: true,
                ..BlockerAnswer::default()
            },
        )
        .unwrap();
    assert!(done_checks(&after).is_empty());
    assert_eq!(after.state, WORKING);
    let closed = e.c.set_task_state(USER, &t.id, COMPLETED, "").unwrap();
    assert_eq!(closed.state, COMPLETED);
}

#[test]
fn not_done_goes_back_and_a_new_check_replaces_it() {
    let e = env();
    let t = claimed(&e);
    let got = e.c.set_task_state(BOB, &t.id, COMPLETED, "").unwrap();
    let first = done_checks(&got)[0].id.clone();
    e.c.sweep_notices().unwrap();
    // The person says why it isn't done: the agent hears it.
    let (_, ns) =
        e.c.answer(
            USER,
            &t.id,
            &first,
            BlockerAnswer {
                text: "the login page still 500s".into(),
                ..BlockerAnswer::default()
            },
        )
        .unwrap();
    assert!(
        ns.iter().any(|n| n.kind == NOTICE_ANSWERED
            && n.to.iter().any(|k| k == BOB)
            && n.text.contains("the login page still 500s")),
        "{ns:?}"
    );
    // The agent fixes it and says done again: the answered check gives
    // way to a fresh one, so only one waits on the person.
    let again =
        e.c.set_task_state(BOB, &t.id, COMPLETED, "fixed the 500")
            .unwrap();
    let checks = done_checks(&again);
    assert_eq!(checks.len(), 1);
    assert_ne!(checks[0].id, first);
    assert_eq!(checks[0].context, "fixed the 500");
    assert_eq!(again.state, BLOCKED);
}

#[test]
fn asking_twice_keeps_one_check() {
    let e = env();
    let t = claimed(&e);
    e.c.set_task_state(BOB, &t.id, COMPLETED, "").unwrap();
    let got = e.c.set_task_state(BOB, &t.id, COMPLETED, "").unwrap();
    assert_eq!(done_checks(&got).len(), 1);
}

#[test]
fn an_old_windows_done_closes_on_the_next_completed() {
    // A window that predates done checks sends Done as a plain answer;
    // the agent resolves it and says completed again: the person's Done
    // stands, so the card closes instead of asking again.
    let e = env();
    let t = claimed(&e);
    let got = e.c.set_task_state(BOB, &t.id, COMPLETED, "").unwrap();
    let bid = done_checks(&got)[0].id.clone();
    e.c.answer(
        USER,
        &t.id,
        &bid,
        BlockerAnswer {
            pick: 1,
            ..BlockerAnswer::default()
        },
    )
    .unwrap();
    // The agent resolves the answered check (board_unblock blocker_id).
    e.c.answer(
        BOB,
        &t.id,
        &bid,
        BlockerAnswer {
            text: "closing it".into(),
            resolve: true,
            ..BlockerAnswer::default()
        },
    )
    .unwrap();
    let closed = e.c.set_task_state(BOB, &t.id, COMPLETED, "").unwrap();
    assert_eq!(closed.state, COMPLETED);
    assert!(done_checks(&closed).is_empty());
}
