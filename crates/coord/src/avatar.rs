//! Agents' faces (card #407): an open-bot-avatar definition, one SVG
//! outline in a 0 0 100 100 box plus where the eyes sit, kept by agent name
//! so a name keeps its face on every device. An agent sets its own with the
//! avatar tool; the row is `agent.avatar` `{name, avatar}`, newest wins.
//!
//! Only a definition that passes [`check`] is kept: plain JSON with the
//! contract's fields, a path made of SVG path commands and numbers, and a
//! size cap. The window checks it again with the package's own
//! validateAvatar before drawing it, and falls back to a default face.
//! Never raw SVG.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use crate::gojson::decode;

/// The replicated row a face writes.
pub const KIND: &str = store::kinds::AGENT_AVATAR;

/// The longest outline path kept.
pub const MAX_PATH: usize = 4096;
/// The longest definition kept, as stored JSON.
pub const MAX_JSON: usize = 5000;

/// The eyes: their centre, the distance between them and their size.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Eyes {
    pub x: f64,
    pub y: f64,
    pub spacing: f64,
    pub scale: f64,
}

/// One face, in the package's contract (version 1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Avatar {
    pub version: u8,
    pub id: String,
    pub name: String,
    pub body: Body,
    pub eyes: Eyes,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Body {
    pub path: String,
}

fn num(v: &serde_json::Value, field: &str) -> Result<f64, String> {
    match v.get(field).and_then(|x| x.as_f64()) {
        Some(n) if n.is_finite() => Ok(n),
        _ => Err(format!("eyes.{field} must be a number")),
    }
}

fn id_ok(id: &str) -> bool {
    let b = id.as_bytes();
    let ok = |c: &u8| c.is_ascii_lowercase() || c.is_ascii_digit();
    !b.is_empty()
        && b.len() <= 40
        && ok(&b[0])
        && ok(&b[b.len() - 1])
        && b.iter().all(|c| ok(c) || *c == b'-')
}

/// Checks a face definition and returns it in canonical form, or why not.
pub fn check(v: &serde_json::Value) -> Result<Avatar, String> {
    let o = v.as_object().ok_or("the face must be a JSON object")?;
    for k in o.keys() {
        if !matches!(k.as_str(), "version" | "id" | "name" | "body" | "eyes") {
            return Err(format!("unknown field {k:?}"));
        }
    }
    if v.get("version").and_then(|x| x.as_u64()) != Some(1) {
        return Err("version must be 1".into());
    }
    let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
    if !id_ok(id) {
        return Err("id must be a lowercase slug (a-z, 0-9, -), at most 40 characters".into());
    }
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").trim();
    if name.is_empty()
        || name.chars().count() > 40
        || name.chars().any(|c| c.is_control() || c == '<' || c == '>')
    {
        return Err("name must be 1 to 40 plain characters".into());
    }
    let body = v
        .get("body")
        .and_then(|x| x.as_object())
        .ok_or("body must be an object { \"path\": \"...\" }")?;
    if body.keys().any(|k| k != "path") {
        return Err("body takes only \"path\"".into());
    }
    let path = body
        .get("path")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    if path.is_empty() || path.len() > MAX_PATH {
        return Err(format!("body.path must be 1 to {MAX_PATH} characters"));
    }
    if !path
        .chars()
        .all(|c| "MmLlHhVvCcSsQqTtAaZz0123456789.,-+eE \t\n".contains(c))
    {
        return Err("body.path may hold only SVG path commands and numbers".into());
    }
    if !path.starts_with(['M', 'm']) {
        return Err("body.path must start with M".into());
    }
    let e = v
        .get("eyes")
        .and_then(|x| x.as_object())
        .ok_or("eyes must be an object")?;
    if e.keys()
        .any(|k| !matches!(k.as_str(), "x" | "y" | "spacing" | "scale"))
    {
        return Err("eyes takes only x, y, spacing and scale".into());
    }
    let eyes_v = v.get("eyes").unwrap_or(&serde_json::Value::Null);
    let eyes = Eyes {
        x: num(eyes_v, "x")?,
        y: num(eyes_v, "y")?,
        spacing: num(eyes_v, "spacing")?,
        scale: num(eyes_v, "scale")?,
    };
    if !(0.0..=100.0).contains(&eyes.x) || !(0.0..=100.0).contains(&eyes.y) {
        return Err("eyes.x and eyes.y must be inside the 0 to 100 box".into());
    }
    if !(0.0..=60.0).contains(&eyes.spacing) || !(0.2..=4.0).contains(&eyes.scale) {
        return Err("eyes.spacing must be 0 to 60 and eyes.scale 0.2 to 4".into());
    }
    let a = Avatar {
        version: 1,
        id: id.to_string(),
        name: name.to_string(),
        body: Body {
            path: path.split_whitespace().collect::<Vec<_>>().join(" "),
        },
        eyes,
    };
    if to_json(&a).len() > MAX_JSON {
        return Err("the face is too big".into());
    }
    Ok(a)
}

/// The stored form of a face.
pub fn to_json(a: &Avatar) -> String {
    serde_json::to_string(a).unwrap_or_default()
}

/// The key a face is kept under: the agent's name, trimmed and lower case.
pub fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

#[derive(Debug, Clone)]
struct Kept {
    json: String,
    hlc: u64,
    device: String,
    seq: i64,
}

crate::go_struct! {
    #[derive(Debug, Clone, Default)]
    struct Row {
        name: String = "name",
        avatar: String = "avatar",
    }
}

/// The `agent.avatar` payload.
#[derive(Serialize)]
pub struct RowOut<'a> {
    pub avatar: &'a str,
    pub name: &'a str,
}

/// Folds `agent.avatar` rows: the newest face per agent name. A row whose
/// face fails [`check`] is ignored. Safe for concurrent use.
#[derive(Debug, Default)]
pub struct Faces {
    m: Mutex<HashMap<String, Kept>>,
}

impl Faces {
    pub fn new() -> Faces {
        Faces::default()
    }

    /// Folds one row; reports whether a face changed. `""` clears a face.
    pub fn apply(&self, ev: &store::Event) -> bool {
        if ev.kind != KIND {
            return false;
        }
        let Some(r) = decode::<Row>(&ev.payload) else {
            return false;
        };
        let key = name_key(&r.name);
        if key.is_empty() {
            return false;
        }
        let json = if r.avatar.is_empty() {
            String::new()
        } else {
            match serde_json::from_str::<serde_json::Value>(&r.avatar)
                .ok()
                .and_then(|v| check(&v).ok())
            {
                Some(a) => to_json(&a),
                None => return false,
            }
        };
        let n = Kept {
            json,
            hlc: ev.hlc.0,
            device: ev.device_id.clone(),
            seq: ev.seq,
        };
        let mut m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cur) = m.get(&key) {
            let newer = (n.hlc, &n.device, n.seq) > (cur.hlc, &cur.device, cur.seq);
            if !newer {
                return false;
            }
        }
        m.insert(key, n);
        true
    }

    /// The face kept for an agent name, "" for none.
    pub fn get(&self, name: &str) -> String {
        let m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        m.get(&name_key(name))
            .map(|k| k.json.clone())
            .unwrap_or_default()
    }

    /// Every face kept, by name key.
    pub fn all(&self) -> HashMap<String, String> {
        let m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        m.iter()
            .filter(|(_, k)| !k.json.is_empty())
            .map(|(n, k)| (n.clone(), k.json.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mug() -> serde_json::Value {
        json!({"version":1,"id":"my-mug","name":"Mug","body":{"path":"M20 20 H80 V80 H20 Z"},"eyes":{"x":50,"y":46,"spacing":15,"scale":1}})
    }

    #[test]
    fn a_good_face_passes_in_canonical_form() {
        let a = check(&mug()).unwrap();
        assert_eq!(a.body.path, "M20 20 H80 V80 H20 Z");
        assert!(to_json(&a).starts_with("{\"version\":1,\"id\":\"my-mug\""));
    }

    #[test]
    fn markup_and_extras_are_refused() {
        let mut v = mug();
        v["body"]["path"] = json!("M0 0<script>");
        assert!(check(&v).is_err());
        let mut v = mug();
        v["svg"] = json!("<svg/>");
        assert!(check(&v).is_err());
        let mut v = mug();
        v["name"] = json!("<b>x</b>");
        assert!(check(&v).is_err());
        let mut v = mug();
        v["eyes"]["x"] = json!(500);
        assert!(check(&v).is_err());
        let mut v = mug();
        v["body"]["path"] = json!(format!("M0 0 {}", "L1 1 ".repeat(1000)));
        assert!(check(&v).is_err());
        assert!(check(&json!("M0 0")).is_err());
    }

    fn row(name: &str, avatar: &str, hlc: u64) -> store::Event {
        store::Event {
            device_id: "pc".into(),
            seq: hlc as i64,
            hlc: hlc::Timestamp(hlc),
            actor: "irori".into(),
            kind: KIND.into(),
            payload: serde_json::to_vec(&RowOut { avatar, name }).unwrap(),
            sig: Vec::new(),
        }
    }

    #[test]
    fn the_newest_valid_face_per_name_wins() {
        let f = Faces::new();
        let good = mug().to_string();
        assert!(f.apply(&row("Libra", &good, 2)));
        assert!(!f.apply(&row("libra", &good, 1)), "older");
        assert!(!f.apply(&row("libra", "{\"svg\":1}", 3)), "invalid");
        assert!(f.get("LIBRA ").contains("my-mug"));
        assert!(f.apply(&row("libra", "", 4)));
        assert_eq!(f.get("libra"), "");
    }
}
