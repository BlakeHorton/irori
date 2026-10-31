//! Mentions on cards and asks (card #77; board.md "Mentions").

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use coord::roster::Agents as _;
use coord::rows::TaskCommentRow;
use coord::*;
use store::{OpenOptions, Store, kinds};

fn m(key: &str, name: &str) -> Mention {
    Mention {
        key: key.into(),
        name: name.into(),
        gone: false,
    }
}

fn nt(title: &str) -> NewTask {
    NewTask {
        title: title.into(),
        ..NewTask::default()
    }
}

fn held_by_alice(e: &Env) -> Task {
    let t = e.c.create_task(USER, nt("fix the build")).unwrap();
    e.c.claim_task(ALICE, &t.id).unwrap();
    e.c.get_task(&t.id).unwrap()
}

fn kinds_of(ns: &[Notice]) -> Vec<&str> {
    ns.iter().map(|n| n.kind.as_str()).collect()
}

#[test]
fn person_mention_wakes_and_watches_never_holds() {
    let e = Env::new();
    let t = held_by_alice(&e);
    let (cm, ns) =
        e.c.person_comment(
            USER,
            &t.id,
            "@carol can you look?",
            false,
            &[m(CAROL, "carol")],
            None,
        )
        .unwrap();
    // The holder hears as before (next turn); the mention is its own notice.
    assert_eq!(kinds_of(&ns), [NOTICE_COMMENT, NOTICE_MENTIONED]);
    assert_eq!(ns[0].to, [ALICE]);
    assert!(!ns[0].wake);
    let mn = &ns[1];
    assert_eq!(mn.to, [CAROL]);
    assert!(mn.wake, "a mention always wakes");
    assert_eq!(mn.reference, format!("comment:{}", cm.id));
    assert!(
        mn.text.starts_with("The person mentioned you on"),
        "{}",
        mn.text
    );
    assert!(
        mn.text.contains(&format!(
            "{ALICE} holds it; answer on the card with board_comment"
        )),
        "{}",
        mn.text
    );
    let got = e.c.get_task(&t.id).unwrap();
    assert_eq!(got.watchers, [CAROL]);
    assert_eq!(got.holders, [ALICE]);
    assert_eq!(cm.mentions, [m(CAROL, "carol")]);
    assert!(!cm.unheard);
    // The row carries the mention, and a replay folds it back.
    let c2 = e.reopen();
    assert_eq!(
        c2.get_comment(&cm.id).unwrap().mentions,
        [m(CAROL, "carol")]
    );
}

#[test]
fn a_mentioned_holder_hears_once() {
    let e = Env::new();
    let t = held_by_alice(&e);
    e.c.watch(BOB, &t.id).unwrap();
    let (_, ns) =
        e.c.person_comment(
            USER,
            &t.id,
            "@alice @bob",
            true,
            &[m(ALICE, "alice"), m(BOB, "bob")],
            None,
        )
        .unwrap();
    assert_eq!(kinds_of(&ns), [NOTICE_MENTIONED]);
    assert_eq!(ns[0].to, [ALICE, BOB]);
}

#[test]
fn an_unheld_card_says_to_join_it() {
    let e = Env::new();
    let t = e.c.create_task(USER, nt("nobody's yet")).unwrap();
    let (_, ns) =
        e.c.comment_on(
            ALICE,
            &t.id,
            CommentInput {
                text: "@bob this is yours".into(),
                mentions: vec![m(BOB, "bob")],
                ..CommentInput::default()
            },
        )
        .unwrap();
    let mn = ns.iter().find(|n| n.kind == NOTICE_MENTIONED).unwrap();
    assert_eq!(mn.to, [BOB]);
    assert!(mn.wake);
    assert!(mn.text.starts_with(&format!("{ALICE} mentioned you on")));
    assert!(
        mn.text.contains(
            "Nobody holds it. If the work is yours, board_join it; else answer on the card with board_comment."
        ),
        "{}",
        mn.text
    );
    let got = e.c.get_task(&t.id).unwrap();
    assert_eq!(got.watchers, [BOB]);
    assert!(got.holders.is_empty());
    // The task.watched row names the writer.
    let w = e.rows("task.watched");
    let p: coord::rows::TaskWatchedRow = payload(w.last().unwrap());
    assert_eq!((p.who.as_str(), p.by.as_str()), (BOB, ALICE));
}

#[test]
fn a_gone_mention_is_stored_not_delivered() {
    let e = Env::new();
    let t = held_by_alice(&e);
    let gone = Mention {
        key: CAROL.into(),
        name: "carol".into(),
        gone: true,
    };
    let (cm, ns) =
        e.c.person_comment(
            USER,
            &t.id,
            "@carol",
            false,
            std::slice::from_ref(&gone),
            None,
        )
        .unwrap();
    assert_eq!(kinds_of(&ns), [NOTICE_COMMENT]);
    assert_eq!(cm.mentions, [gone]);
    assert!(e.c.get_task(&t.id).unwrap().watchers.is_empty());
}

#[test]
fn self_people_and_irori_are_dropped_and_ten_is_the_cap() {
    let e = Env::new();
    let t = held_by_alice(&e);
    let (cm, ns) =
        e.c.comment_on(
            ALICE,
            &t.id,
            CommentInput {
                text: "hi".into(),
                mentions: vec![m(ALICE, "me"), m(USER, "you"), m("irori", "irori")],
                ..CommentInput::default()
            },
        )
        .unwrap();
    assert!(cm.mentions.is_empty());
    assert_eq!(ns.len(), 1);
    let many: Vec<Mention> = (0..11).map(|i| m(&format!("dev-a/s{i}"), "x")).collect();
    want_err(
        e.c.comment_on(
            ALICE,
            &t.id,
            CommentInput {
                text: "everyone".into(),
                mentions: many,
                ..CommentInput::default()
            },
        ),
        ErrorKind::Invalid,
    );
}

#[test]
fn an_answer_mention_is_on_the_thread_and_wakes() {
    let e = Env::new();
    let t = held_by_alice(&e);
    e.c.raise(
        ALICE,
        &t.id,
        NewBlocker {
            question: "which port?".into(),
            ..NewBlocker::default()
        },
    )
    .unwrap();
    let (_, ns) =
        e.c.answer(
            USER,
            &t.id,
            "b1",
            BlockerAnswer {
                text: "ask @bob".into(),
                mentions: vec![m(BOB, "bob")],
                ..BlockerAnswer::default()
            },
        )
        .unwrap();
    assert_eq!(kinds_of(&ns), [NOTICE_ANSWERED, NOTICE_MENTIONED]);
    assert_eq!(ns[0].to, [ALICE]);
    assert_eq!(ns[1].to, [BOB]);
    let cs = e.c.comments(&t.id).unwrap();
    assert_eq!(cs.last().unwrap().mentions, [m(BOB, "bob")]);
    assert_eq!(e.c.get_task(&t.id).unwrap().watchers, [BOB]);
}

#[test]
fn a_mention_with_nothing_stored() {
    let e = Env::new();
    let t = held_by_alice(&e);
    let n =
        e.c.mention(ALICE, &t.id, &[m(BOB, "bob")], "which port, @bob?")
            .unwrap()
            .unwrap();
    assert_eq!(
        (n.kind.as_str(), n.to.clone(), n.wake),
        (NOTICE_MENTIONED, vec![BOB.to_string()], true)
    );
    assert_eq!(n.reference, format!("task:{}", t.id));
    assert!(e.c.mention(ALICE, &t.id, &[], "x").unwrap().is_none());
}

fn agents(e: &Env) -> Vec<coord::roster::AgentCard> {
    e.ros.agents()
}

#[test]
fn an_ask_with_mentions_goes_only_to_them() {
    let e = Env::new();
    e.ros.set(BOB, "busy");
    let (t, n) =
        e.c.post_ask(
            USER,
            NewAsk {
                text: "@bob why is the Mac build slow?".into(),
                mentions: vec![m(BOB, "bob")],
                ..NewAsk::default()
            },
            &agents(&e),
        )
        .unwrap();
    // Busy or not, only the mentioned agent hears, woken.
    assert_eq!(n.to, [BOB]);
    assert!(n.wake);
    assert!(
        n.text.starts_with("The person asks you by name"),
        "{}",
        n.text
    );
    let a = t.ask.clone().unwrap();
    assert_eq!((a.told, a.heard.clone()), (1, vec![BOB.to_string()]));
    assert_eq!(a.mentions, [m(BOB, "bob")]);
    // No late arrivals: a newly idle agent isn't told.
    e.ros.set(CAROL, "idle");
    e.clk.add(secs(5));
    let ns = e.c.sweep_asks(&agents(&e)).unwrap();
    assert!(ns.is_empty(), "{ns:?}");
    // Wake them re-tells only the mentioned.
    e.c.ask_again(USER, &t.id).unwrap();
    let ns = e.c.sweep_asks(&agents(&e)).unwrap();
    assert_eq!(ns.len(), 1, "{ns:?}");
    assert_eq!(ns[0].to, [BOB]);
    assert!(ns[0].wake);
    // The person's comment while nobody took it reaches the same agent.
    let (_, ns) =
        e.c.person_comment(USER, &t.id, "any luck?", false, &[], None)
            .unwrap();
    assert_eq!(ns[0].kind, NOTICE_ASK_COMMENT);
    assert_eq!(ns[0].to, [BOB]);
}

#[test]
fn an_ask_whose_mentions_are_all_gone_is_unclaimed_at_once() {
    let e = Env::new();
    let (t, n) =
        e.c.post_ask(
            USER,
            NewAsk {
                text: "@fern?".into(),
                mentions: vec![Mention {
                    key: "dev-a/fern".into(),
                    name: "fern".into(),
                    gone: true,
                }],
                ..NewAsk::default()
            },
            &agents(&e),
        )
        .unwrap();
    assert!(n.to.is_empty());
    let a = t.ask.unwrap();
    assert_eq!(a.told, 0);
    assert!(a.unclaimed);
}

// ---- shared boards ----

const BOARD_PERSON: &str = "person:p1";
const BOARD_CARD: &str = "agent:c1";

struct Board {
    _dir: tempfile::TempDir,
    st: Arc<Store>,
    c: Coord,
}

fn board() -> Board {
    let clk = Clk(Arc::new(Mutex::new(fixed_now())));
    let c2 = clk.clone();
    let dir = tempfile::tempdir().unwrap();
    let hc = hlc::Clock::with_now(Box::new(move || c2.now().to_system()));
    let st = Arc::new(
        Store::open_with(
            dir.path().join("boards").join("b.db"),
            "dev-a",
            Arc::new(hc),
            OpenOptions {
                board: true,
                ..OpenOptions::default()
            },
        )
        .unwrap(),
    );
    let raw = |s: &str| gocompat::json::RawJson(s.as_bytes().to_vec());
    st.append(
        BOARD_PERSON,
        kinds::BOARD_CREATED,
        &raw(r#"{"boardId":"b","creator":{"personId":"p1"},"name":"b","tag":"b"}"#),
    )
    .unwrap();
    st.append(
        BOARD_CARD,
        kinds::AGENT_CARD,
        &raw(r#"{"cardId":"c1","personId":"p1","status":"active"}"#),
    )
    .unwrap();
    let c = Coord::new(
        st.clone(),
        None,
        Options {
            now: Some(clk.func()),
            board: true,
            person: BOARD_PERSON.into(),
            is_owner: Some(Arc::new(|k: &str| k == BOARD_PERSON)),
            ..Options::default()
        },
    )
    .unwrap();
    Board { _dir: dir, st, c }
}

#[test]
fn a_shared_card_keeps_mentions_out_of_the_board_log() {
    let b = board();
    let t = b.c.create_task(BOARD_PERSON, nt("shared work")).unwrap();
    let (cm, ns) =
        b.c.person_comment(
            BOARD_PERSON,
            &t.id,
            "@cedar look",
            false,
            &[m(BOARD_CARD, "cedar")],
            None,
        )
        .unwrap();
    // The agent is still woken and watches.
    let mn = ns.iter().find(|n| n.kind == NOTICE_MENTIONED).unwrap();
    assert_eq!(mn.to, [BOARD_CARD]);
    assert!(mn.wake);
    assert_eq!(b.c.get_task(&t.id).unwrap().watchers, [BOARD_CARD]);
    // The row and the folded comment carry plain text only.
    assert!(cm.mentions.is_empty());
    let rows =
        b.st.list(&store::Filter {
            kinds: vec![kinds::TASK_COMMENT.into()],
            limit: store::MAX_LIST_LIMIT,
            ..store::Filter::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    let p: TaskCommentRow = payload(&rows[0]);
    assert!(p.mentions.is_empty());
    assert!(!String::from_utf8_lossy(&rows[0].payload).contains("mentions"));
    assert_eq!(p.text, "@cedar look");
}

#[test]
fn a_shared_board_ask_refuses_mentions() {
    let b = board();
    want_err(
        b.c.post_ask(
            BOARD_PERSON,
            NewAsk {
                text: "@cedar?".into(),
                mentions: vec![m(BOARD_CARD, "cedar")],
                ..NewAsk::default()
            },
            &[],
        ),
        ErrorKind::Invalid,
    );
}
