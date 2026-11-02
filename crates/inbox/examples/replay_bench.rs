//! Times the inbox replay on a copy of a data dir's irori.db (card #243):
//! `cargo run --release -p irori-inbox --example replay_bench -- <irori.db> <device-id>`.
//! Prints the time to read the replay rows alone, then Inbox::new in full.
use std::sync::Arc;
use std::time::Instant;

use inbox::{AgentCard, Inbox, Options, Roster};
use store::{Cursor, Filter, MAX_LIST_LIMIT, Store};

struct NoRoster;
impl Roster for NoRoster {
    fn agents(&self) -> Vec<AgentCard> {
        Vec::new()
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("irori.db path");
    let dev = args.next().unwrap_or_else(|| "bench".into());
    let st = Arc::new(Store::open(&path, &dev, Arc::new(hlc::Clock::new())).expect("open"));
    let kinds = [
        "message.sent",
        "delivery.state",
        "wake.issued",
        "agent.alias",
        "agent.name",
        "turn.engaged",
        "turn.ended",
        "agent.seen",
        "delivery.decided",
        "receive.set",
        "autonomy.set",
        "threads.set",
    ];
    let t = Instant::now();
    let (mut n, mut bytes, mut after) = (0usize, 0usize, Cursor::default());
    loop {
        let evs = st
            .list(&Filter {
                kinds: kinds.iter().map(|k| k.to_string()).collect(),
                after: after.clone(),
                limit: MAX_LIST_LIMIT,
                ..Filter::default()
            })
            .expect("list");
        for ev in &evs {
            n += 1;
            bytes += ev.payload.len();
            after = ev.cursor();
        }
        if (evs.len() as i64) < MAX_LIST_LIMIT {
            break;
        }
    }
    println!(
        "read  {n} rows, {} KB: {} ms",
        bytes / 1024,
        t.elapsed().as_millis()
    );
    let t = Instant::now();
    let ib = Inbox::new(
        st.clone(),
        Arc::new(NoRoster),
        Options {
            device_id: dev.clone(),
            ..Default::default()
        },
    )
    .expect("inbox");
    println!("Inbox::new: {} ms", t.elapsed().as_millis());
    drop(ib);
}
