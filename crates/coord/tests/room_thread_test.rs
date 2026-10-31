//! Threads (card #116): the person's conversation with the room. The
//! agents on the cards it names hear it first; after a quiet spell one
//! idle agent gets it; once answered nobody holds it, follow-ups go to the
//! agents that answered, and only the person resolves it.

mod common;

use common::*;
use coord::roster::AgentCard;
use coord::*;

fn agents(e: &Env) -> Vec<AgentCard> {
    use coord::roster::Agents as _;
    e.ros.agents()
}

fn thread(e: &Env, text: &str) -> (Task, Notice) {
    e.c.post_ask(
        USER,
        NewAsk {
            text: text.into(),
            thread: true,
            ..NewAsk::default()
        },
        &agents(e),
    )
    .unwrap()
}

fn ask_of(c: &Coord, id: &str) -> Ask {
    c.get_task(id).and_then(|t| t.ask).unwrap()
}

/// A card alice holds and is busy on.
fn alices_card(e: &Env) -> Task {
    let t =
        e.c.create_task(
            ALICE,
            NewTask {
                title: "lobby join".into(),
                ..NewTask::default()
            },
        )
        .unwrap();
    e.ros.set(ALICE, "busy");
    e.c.claim_task(ALICE, &t.id).unwrap()
}

#[test]
fn a_thread_goes_to_the_cards_it_names_first() {
    let e = Env::new();
    let card = alices_card(&e);
    let (tk, n) = thread(&e, &format!("Is #{} still moving?", card.short));
    let a = ask_of(&e.c, &tk.id);
    assert!(
        a.thread && a.taker.is_empty() && a.heard == [ALICE],
        "{a:?}"
    );
    assert!(tk.holders.is_empty(), "nobody holds a thread");
    assert!(
        n.to == [ALICE] && n.wake && n.text.contains("started a thread") && n.text.contains("45s"),
        "{n:?}"
    );

    // Inside the quiet spell nobody else hears it.
    e.clk.add(secs(30));
    assert!(e.c.sweep_asks(&agents(&e)).unwrap().is_empty());

    // Alice answers: answered, still nobody holds it, and no hand-out.
    e.c.add_comment(
        ALICE,
        &tk.id,
        "Waiting on your call about the relay.",
        false,
    )
    .unwrap();
    assert!(e.c.sweep_asks(&agents(&e)).unwrap().is_empty());
    let a = ask_of(&e.c, &tk.id);
    assert!(a.reason == "answered" && a.taker.is_empty(), "{a:?}");
    e.clk.add(secs(60));
    assert!(e.c.sweep_asks(&agents(&e)).unwrap().is_empty());
    assert!(e.c.get_task(&tk.id).unwrap().holders.is_empty());

    // The person's follow-up goes to alice, woken.
    let (_, ns) =
        e.c.person_comment(USER, &tk.id, "Our relay.", false, &[], None)
            .unwrap();
    assert!(
        ns.len() == 1 && ns[0].to == [ALICE] && ns[0].wake && ns[0].text.contains("Our relay."),
        "{ns:?}"
    );

    // Only the person resolves it.
    want_err(
        e.c.ask_close(
            ALICE,
            &tk.id,
            AskOutcome {
                kind: OUTCOME_ANSWERED.into(),
                note: "done".into(),
                ..AskOutcome::default()
            },
        ),
        ErrorKind::NotAllowed,
    );
    let t = e.c.resolve_thread(USER, &tk.id).unwrap();
    assert!(is_terminal(&t.state), "{t:?}");
    assert!(e.c.get_task(&tk.id).unwrap().open_ask().is_none());
}

#[test]
fn nobody_answers_so_an_idle_agent_gets_it() {
    let e = Env::new();
    let card = alices_card(&e);
    let (tk, _) = thread(&e, &format!("Did #{} finish?", card.short));
    e.ros.set(CAROL, "busy");
    e.clk.add(secs(46));
    let ns = e.c.sweep_asks(&agents(&e)).unwrap();
    assert!(
        ns.len() == 1
            && ns[0].to == [BOB]
            && ns[0].text.contains("handed it to you")
            && ns[0].text.contains(ALICE),
        "{ns:?}"
    );
    let a = ask_of(&e.c, &tk.id);
    assert!(a.taker == BOB && a.handed, "{a:?}");
    // Bob answers: answered, and bob no longer holds it.
    e.c.add_comment(BOB, &tk.id, "It finished an hour ago.", false)
        .unwrap();
    e.c.sweep_asks(&agents(&e)).unwrap();
    let a = ask_of(&e.c, &tk.id);
    assert!(a.reason == "answered" && a.taker.is_empty(), "{a:?}");
    assert!(e.c.get_task(&tk.id).unwrap().holders.is_empty());
}

#[test]
fn a_thread_naming_nothing_goes_to_an_idle_agent_at_once() {
    let e = Env::new();
    e.ros.set(ALICE, "busy");
    e.ros.set(CAROL, "busy");
    let (tk, n) = thread(&e, "What did we ship today?");
    assert!(
        n.to == [BOB] && n.text.contains("handed it to you"),
        "{n:?}"
    );
    assert_eq!(ask_of(&e.c, &tk.id).taker, BOB);
}
