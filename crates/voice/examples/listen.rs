//! Headless check of the engine: `cargo run -p irori-voice --example listen -- [seconds]`.
//! Prints the status, then every event of one session (default 8 s).

use std::sync::Arc;
use std::time::Duration;

fn main() {
    let secs: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    println!("status: {:?}", voice::status());
    let t0 = std::time::Instant::now();
    let sink: voice::Sink =
        Arc::new(move |e| println!("{:>6} ms  {e:?}", t0.elapsed().as_millis()));
    match voice::start(sink) {
        Ok(session) => {
            println!("listening for {secs} s");
            std::thread::sleep(Duration::from_secs(secs));
            session.stop();
            println!("stopped");
        }
        Err(e) => println!("start failed: {:?}: {}", e.code, e.message),
    }
}
