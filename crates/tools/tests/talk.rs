//! Talk's tools (card #389): talk_post, talk_read, talk_list, talk_join
//! and talk_leave over channels, DMs and threads, and the conversation
//! views the window shares.

mod common;

use common::*;
use coord::talk as ct;
use inbox::mode::WAKE;

const USER: &str = "user:dev1";

#[test]
fn channels_mentions_and_reading() {
    let h = new_harness();
    let (a, c) = (call_a(), call_c());
    has(
        &h.okj(&a, "talk_list", json!({})),
        &["no conversations yet"],
    );
    let ch =
        h.b.talk
            .create_channel(USER, "general", "anything goes")
            .unwrap();
    // Not in it yet: listed as one to join.
    has(
        &h.okj(&a, "talk_list", json!({})),
        &["Channels you aren't in", "#general"],
    );
    has(
        &h.okj(&a, "talk_join", json!({"conv": "#general"})),
        &["Joined #general"],
    );
    has(
        &h.okj(&a, "talk_join", json!({"conv": ch.id})),
        &["already in #general"],
    );

    // A mention wakes gamma through the mention notice and adds it.
    let out = h.okj(
        &a,
        "talk_post",
        json!({"conv": "#general", "text": "@gamma can you look at the ports?"}),
    );
    has(&out, &["Posted post_", "Mentioned: gamma (woke)."]);
    let ns = h.notices(KEY_C);
    assert_eq!(ns.len(), 1, "{ns:?}");
    assert_eq!(ns[0].mode, WAKE);
    assert_eq!(ns[0].message.reason, coord::NOTICE_MENTIONED);
    assert_eq!(ns[0].message.ref_, format!("talk:{}", ch.id));
    has(
        &ns[0].message.text,
        &[
            "alpha mentioned you in #general",
            "can you look at the ports",
            &format!("talk_post conv={}", ch.id),
        ],
    );
    assert!(h.b.talk.is_member(&ch.id, KEY_C));
    // Beta isn't mentioned: no notice, and it isn't in the channel.
    assert!(h.notices(KEY_B).is_empty());
    assert!(!h.b.talk.is_member(&ch.id, KEY_B));

    // Gamma reads it: one unread first, none after.
    has(
        &h.okj(&c, "talk_list", json!({})),
        &["#general (channel", "1 unread", "1 mention you"],
    );
    has(
        &h.okj(&c, "talk_read", json!({"conv": "#general"})),
        &["can you look at the ports", "alpha ("],
    );
    has_not(&h.okj(&c, "talk_list", json!({})), &["unread"]);
    has(
        &h.okj(&c, "talk_read", json!({"conv": "#general"})),
        &["Nothing new"],
    );

    // Pages: since a post.
    let first = h.b.talk.posts(&ch.id)[0].post_id.clone();
    h.okj(
        &c,
        "talk_post",
        json!({"conv": "#general", "text": "on it"}),
    );
    let out = h.okj(&a, "talk_read", json!({"conv": ch.id, "since": first}));
    has(&out, &["on it"]);
    has_not(&out, &["can you look"]);

    // Leave; joining a DM or a thread isn't a thing.
    has(
        &h.okj(&c, "talk_leave", json!({"conv": "#general"})),
        &["Left #general"],
    );
    h.failsj(
        &a,
        "talk_join",
        json!({"conv": "@beta"}),
        &["only channels"],
    );
    h.failsj(
        &a,
        "talk_post",
        json!({"conv": "#nope", "text": "x"}),
        &["no channel, thread or agent"],
    );
    h.failsj(
        &a,
        "talk_post",
        json!({"conv": "#general", "text": "  "}),
        &["empty post"],
    );
}

#[test]
fn dms_wake_and_show_once() {
    let h = new_harness();
    let (a, b) = (call_a(), call_b());
    let out = h.okj(
        &a,
        "talk_post",
        json!({"conv": "@beta", "text": "got a minute?"}),
    );
    has(&out, &["Posted msg_"]);
    let dm = ct::dm_id(KEY_A, KEY_B);
    // Beta is woken with it, as a message about the DM.
    let ds = h.b.ib.for_agent(KEY_B, false, 10);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].mode, WAKE);
    assert_eq!(ds[0].message.ref_, format!("talk:{dm}"));
    assert_eq!(h.b.talk.members(&dm).len(), 2);

    // Beta answers with reply: the DM shows the post once and the reply.
    h.clk.advance(std::time::Duration::from_secs(5));
    h.okj(
        &b,
        "reply",
        json!({"delivery_id": ds[0].id, "text": "sure, what's up"}),
    );
    let ps = tools::talk::posts(&*h.b, KEY_A, &dm).unwrap();
    let texts: Vec<&str> = ps.iter().map(|p| p.text.as_str()).collect();
    assert_eq!(texts, ["got a minute?", "sure, what's up"]);
    has(
        &h.okj(&a, "talk_read", json!({"conv": "@beta"})),
        &["got a minute?", "sure, what's up"],
    );
    // Gamma may not read it; the person may.
    h.failsj(
        &call_c(),
        "talk_read",
        json!({"conv": dm}),
        &["isn't yours"],
    );
    assert_eq!(tools::talk::posts(&*h.b, USER, &dm).unwrap().len(), 2);

    // To the person: a post, no message.
    let before = h.b.ib.list(100).len();
    h.okj(
        &a,
        "talk_post",
        json!({"conv": "person", "text": "done with #3"}),
    );
    assert_eq!(h.b.ib.list(100).len(), before);
    let mine = ct::dm_id(KEY_A, USER);
    let cs = tools::talk::conversations(&*h.b, USER).unwrap();
    let p = cs
        .iter()
        .find(|x| x.conv_id == mine)
        .expect("the person's DM");
    assert_eq!((p.kind.as_str(), p.unread, p.read_only), ("dm", 1, false));
    assert_eq!(p.name, "alpha");
    // Alpha and beta's DM is the person's to read only.
    let ab = cs.iter().find(|x| x.conv_id == dm).expect("agents' DM");
    assert!(ab.read_only);
    assert_eq!(ab.unread, 0);
}

#[test]
fn agents_messages_mirror_as_dms() {
    let h = new_harness();
    let (a, c) = (call_a(), call_c());
    h.send_to(&a, KEY_C, "the ports clash on 8080");
    let dm = ct::dm_id(KEY_A, KEY_C);
    // No talk.post rows: it's read from the message.
    assert!(h.b.talk.posts(&dm).is_empty());
    let cs = tools::talk::conversations(&*h.b, USER).unwrap();
    let x = cs.iter().find(|x| x.conv_id == dm).expect("mirrored");
    assert!(x.read_only);
    assert_eq!(x.members.len(), 2);
    has(
        &h.okj(&c, "talk_read", json!({"conv": "@alpha"})),
        &["the ports clash on 8080"],
    );
    // Gamma's own list counts it.
    let cs = tools::talk::conversations(&*h.b, KEY_C).unwrap();
    assert!(cs.iter().any(|x| x.conv_id == dm && !x.read_only));
}

#[test]
fn threads_read_and_post_as_comments() {
    let h = new_harness();
    let a = call_a();
    let (t, _) =
        h.b.co
            .post_ask(
                USER,
                coord::NewAsk {
                    text: "why is the build slow?".into(),
                    thread: true,
                    ..Default::default()
                },
                &[],
            )
            .unwrap();
    let r = h.short(&t.id);
    let out = h.okj(
        &a,
        "talk_post",
        json!({"conv": r, "text": "the cache is cold"}),
    );
    has(&out, &["Commented comment:"]);
    assert!(
        h.b.talk.posts(&t.id).is_empty(),
        "no talk.post rows for a thread"
    );
    let out = h.okj(&a, "talk_read", json!({"conv": format!("task:{}", t.id)}));
    has(&out, &["why is the build slow?", "the cache is cold"]);
    let cs = tools::talk::conversations(&*h.b, USER).unwrap();
    let x = cs.iter().find(|x| x.conv_id == t.id).expect("the thread");
    assert_eq!(x.kind, "thread");
    assert_eq!(x.unread, 1, "alpha's answer");
    // Alpha answered, so it's on the thread's list too.
    assert!(
        tools::talk::conversations(&*h.b, KEY_A)
            .unwrap()
            .iter()
            .any(|x| x.conv_id == t.id)
    );
    // Joining a thread isn't Talk's.
    h.failsj(&a, "talk_join", json!({"conv": r}), &["only channels"]);
}
