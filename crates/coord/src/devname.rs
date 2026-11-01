//! The person's own names for their devices (card #184): a device is
//! called by its computer's host name (its certificate's label) until the
//! person renames it in Settings → Devices, which writes a replicated
//! `device.name` row. The rename follows to the person's other devices;
//! it never goes on a shared board (board logs carry no device labels,
//! v0.5 §4.5).

use std::collections::HashMap;
use std::sync::Mutex;

use crate::gojson::decode;

/// The replicated row a rename writes: `{deviceId, name}`.
pub const KIND: &str = store::kinds::DEVICE_NAME;

/// The longest name, in characters.
pub const MAX_LEN: usize = 40;

/// A name as it is kept: one line, trimmed, at most [`MAX_LEN`]
/// characters. `""` goes back to the computer's name.
pub fn clean(name: &str) -> String {
    let one: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let words: Vec<&str> = one.split_whitespace().collect();
    words.join(" ").chars().take(MAX_LEN).collect()
}

#[derive(Debug, Clone)]
struct Pick {
    name: String,
    hlc: u64,
    device: String,
    seq: i64,
}

fn newer(a: &Pick, b: &Pick) -> bool {
    if a.hlc != b.hlc {
        return a.hlc > b.hlc;
    }
    if a.device != b.device {
        return a.device > b.device;
    }
    a.seq > b.seq
}

crate::go_struct! {
    #[derive(Debug, Clone, Default)]
    struct Row {
        device_id: String = "deviceId",
        name: String = "name",
    }
}

/// Folds `device.name` rows: the person's latest name per device. Safe
/// for concurrent use.
#[derive(Debug, Default)]
pub struct Names {
    m: Mutex<HashMap<String, Pick>>,
}

impl Names {
    /// An empty fold.
    pub fn new() -> Names {
        Names::default()
    }

    /// Folds one row; reports whether a name changed. Only the person
    /// renames (actor `user`).
    pub fn apply(&self, ev: &store::Event) -> bool {
        if ev.kind != KIND || ev.actor != "user" {
            return false;
        }
        let Some(r) = decode::<Row>(&ev.payload) else {
            return false;
        };
        if r.device_id.is_empty() {
            return false;
        }
        let n = Pick {
            name: clean(&r.name),
            hlc: ev.hlc.0,
            device: ev.device_id.clone(),
            seq: ev.seq,
        };
        let mut m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cur) = m.get(&r.device_id)
            && !newer(&n, cur)
        {
            return false;
        }
        m.insert(r.device_id, n);
        true
    }

    /// The person's name per device (`""` renames, back to the computer's
    /// name, are left out).
    pub fn chosen(&self) -> HashMap<String, String> {
        let m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        m.iter()
            .filter(|(_, p)| !p.name.is_empty())
            .map(|(id, p)| (id.clone(), p.name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_trims_to_one_short_line() {
        assert_eq!(clean("  Mac\n Book\t"), "Mac Book");
        assert_eq!(clean(""), "");
        assert_eq!(clean(&"x".repeat(60)).chars().count(), MAX_LEN);
    }
}
