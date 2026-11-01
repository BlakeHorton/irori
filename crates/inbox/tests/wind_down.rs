//! The person's Wind down all (card #220, docs/design/wind-down.md): a wake
//! with reason wind_down that only the person sends, that Pause all doesn't
//! hold, and that waits for the agent's safe point instead of cutting it.

mod common;

use common::*;
use inbox::{Policy, REASON_WIND_DOWN, SendOptions, hold, mode, st};

fn wind_down() -> SendOptions {
    SendOptions {
        mode: mode::WAKE.into(),
        reason: REASON_WIND_DOWN.into(),
        ..SendOptions::default()
    }
}

fn pause_all(h: &Harness) {
    h.ib.set_policy(
        Policy {
            mode: String::new(),
            block: vec![],
            allow: vec![],
            paused: true,
        },
        "user",
    )
    .unwrap();
}

#[test]
fn only_the_person_winds_down() {
    let h = Harness::new();
    h.abc();
    let err =
        h.ib.send_with(
            &claude(KEY_A, "alpha"),
            &to("beta"),
            "wind down",
            wind_down(),
        )
        .unwrap_err();
    assert!(err.to_string().contains("only the person"), "{err}");
}

#[test]
fn wind_down_wakes_with_the_instructions() {
    let h = Harness::new();
    h.abc();
    let (m, d) =
        h.ib.send_with(
            &person(),
            &to("beta"),
            "Wind down for the day.",
            wind_down(),
        )
        .unwrap();
    assert_eq!(m.reason, REASON_WIND_DOWN);
    assert_eq!(d.mode, mode::WAKE);
    let w = h.wake(KEY_B);
    assert_eq!(w.mode, mode::WAKE, "a wind-down never cuts a turn");
    assert_eq!(w.delivery_ids, vec![d.id]);
    for want in [
        "wind_down=\"true\"",
        "is the person winding you down",
        "commit and push",
        "left_off",
        "board_leave",
        "release your leases",
        "\"wound down\"",
        "Wind down for the day.",
    ] {
        assert!(w.prompt.contains(want), "missing {want:?} in\n{}", w.prompt);
    }
}

#[test]
fn an_ordinary_wake_has_no_wind_down_text() {
    let h = Harness::new();
    h.abc();
    h.send(&person(), &to("beta"), "hello");
    let w = h.wake(KEY_B);
    assert!(!w.prompt.contains("wind_down"), "{}", w.prompt);
}

#[test]
fn wind_down_gets_through_pause_all() {
    let h = Harness::new();
    h.abc();
    pause_all(&h);
    let (_, held) = h.send(&person(), &to("beta"), "an ordinary message");
    assert_eq!(h.state(&held.id).hold, hold::PAUSED);
    let (_, d) =
        h.ib.send_with(&person(), &to("beta"), "wind down", wind_down())
            .unwrap();
    h.want_state(&d.id, st::QUEUED);
    assert_eq!(h.state(&d.id).hold, "");
    let w = h.wake(KEY_B);
    assert_eq!(w.delivery_ids, vec![d.id], "only the wind-down goes");
    h.no_wake(KEY_B);
}

#[test]
fn wind_down_waits_for_the_running_turn() {
    let h = Harness::new();
    h.abc();
    // beta is in a turn: its wake is delivered, not ended.
    h.send(&claude(KEY_A, "alpha"), &to("beta"), "a long task");
    let w1 = h.wake(KEY_B);
    h.ib.mark_delivered(KEY_B, &w1.token).unwrap();
    let (_, d) =
        h.ib.send_with(&person(), &to("beta"), "wind down", wind_down())
            .unwrap();
    // Not cut: no wake jumps the gate.
    h.no_wake(KEY_B);
    // It rides into the running turn at its next tool round, as an append.
    let a = h.ib.next_append(KEY_B).unwrap().expect("an append");
    assert_eq!(a.delivery_ids, vec![d.id]);
    assert!(a.prompt.contains("wind_down=\"true\""), "{}", a.prompt);
}

#[test]
fn wind_down_appends_under_pause_all_and_nothing_else_does() {
    let h = Harness::new();
    h.abc();
    h.send(&claude(KEY_A, "alpha"), &to("beta"), "a long task");
    let w1 = h.wake(KEY_B);
    h.ib.mark_delivered(KEY_B, &w1.token).unwrap();
    pause_all(&h);
    h.send(&person(), &to("beta"), "an ordinary message");
    let (_, d) =
        h.ib.send_with(&person(), &to("beta"), "wind down", wind_down())
            .unwrap();
    let a = h.ib.next_append(KEY_B).unwrap().expect("an append");
    assert_eq!(a.delivery_ids, vec![d.id]);
    assert!(h.ib.next_append(KEY_B).unwrap().is_none());
}
