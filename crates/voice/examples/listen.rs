//! Headless check of the engine:
//! `cargo run -p irori-voice --example listen -- <model.bin> [seconds] [speech.wav]`.
//! Prints the status, then every event of one session (default 8 s). With
//! a WAV (16-bit PCM, any rate, mono or stereo) the engine hears that file
//! in real time instead of the microphone.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    let model = PathBuf::from(args.next().unwrap_or_else(|| voice::MODEL.file.to_string()));
    let secs: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(8);
    let wav = args.next();
    println!("status: {:?}", voice::status());
    let t0 = std::time::Instant::now();
    let sink: voice::Sink =
        Arc::new(move |e| println!("{:>6} ms  {e:?}", t0.elapsed().as_millis()));
    let started = match &wav {
        Some(p) => voice::start_with_samples(&model, read_wav(p), sink),
        None => voice::start(&model, sink),
    };
    match started {
        Ok(session) => {
            println!("{:>6} ms  listening for {secs} s", t0.elapsed().as_millis());
            std::thread::sleep(Duration::from_secs(secs));
            session.stop();
            println!("{:>6} ms  stopped", t0.elapsed().as_millis());
        }
        Err(e) => println!("start failed: {:?}: {}", e.code, e.message),
    }
}

/// A 16-bit PCM WAV as 16 kHz mono (nearest-sample resampling; enough here).
fn read_wav(path: &str) -> Vec<f32> {
    let d = std::fs::read(path).expect("read wav");
    let u16at = |i: usize| u16::from_le_bytes([d[i], d[i + 1]]);
    let u32at = |i: usize| u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
    let (mut ch, mut rate, mut i) = (1usize, 16_000usize, 12usize);
    while i + 8 <= d.len() {
        let id = &d[i..i + 4];
        let len = u32at(i + 4) as usize;
        if id == b"fmt " {
            ch = u16at(i + 10) as usize;
            rate = u32at(i + 12) as usize;
        } else if id == b"data" {
            let pcm: Vec<f32> = d[i + 8..(i + 8 + len).min(d.len())]
                .chunks(2 * ch)
                .map(|f| {
                    f.chunks(2)
                        .map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0)
                        .sum::<f32>()
                        / ch as f32
                })
                .collect();
            let n = pcm.len() * 16_000 / rate;
            return (0..n).map(|k| pcm[k * rate / 16_000]).collect();
        }
        i += 8 + len + (len & 1);
    }
    panic!("no data chunk in {path}");
}
