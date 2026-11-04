//! The person's avatar on shared spaces (card #393): a small picture they
//! pick, kept like their colour (#385). A private `person.avatar` row,
//! written only by the person, carries it; every space log learns it from
//! the `avatar` field on their member.profile.

use std::sync::Mutex;

use crate::gojson::decode;

/// The private row a pick writes: `{avatar, atDevice}`.
pub const KIND: &str = store::kinds::PERSON_AVATAR;

/// The longest avatar, in bytes of its data URL. The window shrinks a
/// picked picture to a small square well under this; the cap keeps a
/// member.profile row, copied into every space log, small.
pub const MAX_LEN: usize = 32 * 1024;

/// The data URL prefixes an avatar may have: PNG, JPEG or WebP, base64.
/// Nothing else (no SVG, no links), since it is drawn on other people's
/// screens.
const PREFIXES: &[&str] = &[
    "data:image/png;base64,",
    "data:image/jpeg;base64,",
    "data:image/webp;base64,",
];

/// Whether `a` can be an avatar: "" (none), or a PNG, JPEG or WebP data
/// URL of plain base64, at most [`MAX_LEN`] bytes.
pub fn valid(a: &str) -> bool {
    if a.is_empty() {
        return true;
    }
    if a.len() > MAX_LEN {
        return false;
    }
    let Some(body) = PREFIXES.iter().find_map(|p| a.strip_prefix(p)) else {
        return false;
    };
    !body.is_empty()
        && body
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

crate::go_struct! {
    #[derive(Debug, Clone, Default)]
    struct Row {
        avatar: String = "avatar",
    }
}

#[derive(Debug, Clone)]
struct Pick {
    avatar: String,
    hlc: u64,
    device: String,
    seq: i64,
}

fn newer(a: &Pick, b: &Pick) -> bool {
    (a.hlc, &a.device, a.seq) > (b.hlc, &b.device, b.seq)
}

/// Folds `person.avatar` rows: the person's newest pick. Safe for
/// concurrent use.
#[derive(Debug, Default)]
pub struct PersonAvatar {
    m: Mutex<Option<Pick>>,
}

impl PersonAvatar {
    /// An empty fold.
    pub fn new() -> PersonAvatar {
        PersonAvatar::default()
    }

    /// Folds one row; reports whether the avatar changed. Only the person
    /// picks (actor `user`).
    pub fn apply(&self, ev: &store::Event) -> bool {
        if ev.kind != KIND || ev.actor != "user" {
            return false;
        }
        let Some(r) = decode::<Row>(&ev.payload) else {
            return false;
        };
        if !valid(&r.avatar) {
            return false;
        }
        let n = Pick {
            avatar: r.avatar,
            hlc: ev.hlc.0,
            device: ev.device_id.clone(),
            seq: ev.seq,
        };
        let mut m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        if m.as_ref().is_some_and(|cur| !newer(&n, cur)) {
            return false;
        }
        *m = Some(n);
        true
    }

    /// The person's avatar, "" for none.
    pub fn chosen(&self) -> String {
        let m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        m.as_ref().map(|p| p.avatar.clone()).unwrap_or_default()
    }
}
