//! Who a text and a `mention` list name (card #77; board.md "Agents
//! mention too"), resolved against the roster of every paired device.
//! The tools use it for agents' writes; the window passes the keys its
//! picker recorded and never parses the person's text.

use crate::board::{MAX_MENTIONS, Mention};

const SWEEPER: &str = "irori";
use crate::roster::AgentCard;

/// A miss: why an `@name` or a `mention` item reached nobody.
pub const MISS_AMBIGUOUS: &str = "ambiguous";
pub const MISS_UNKNOWN: &str = "unknown";
/// Past the MAX_MENTIONS cap.
pub const MISS_CAP: &str = "over the cap";

/// One name or key that wasn't delivered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Miss {
    /// As written, without the @.
    pub name: String,
    /// [`MISS_AMBIGUOUS`], [`MISS_UNKNOWN`] or [`MISS_CAP`].
    pub reason: String,
    /// The keys it could be, for an ambiguous one.
    pub candidates: Vec<String>,
}

impl Miss {
    /// Whether the item looked like a key (no agent has it).
    pub fn is_key(&self) -> bool {
        key_shaped(&self.name)
    }
}

/// What [`resolve`] found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolved {
    /// Deduped by key, `given` first then the text's order, at most
    /// MAX_MENTIONS.
    pub mentions: Vec<Mention>,
    pub misses: Vec<Miss>,
}

/// Whether `s` looks like an agent key rather than a name: a board key
/// (`agent:…`) or a session key (`<device>/<session>`).
pub fn key_shaped(s: &str) -> bool {
    s.starts_with("agent:") || !irori_identity::owner::device_of(s).is_empty()
}

fn live(a: &AgentCard) -> bool {
    a.status != "gone"
}

fn dropped(key: &str, author: &str) -> bool {
    key.is_empty() || key == author || key == SWEEPER || key.starts_with("user:")
}

/// Ends a token: the end, whitespace, or closing punctuation.
fn ends_token(c: Option<char>) -> bool {
    match c {
        None => true,
        Some(c) => c.is_whitespace() || ".,;:!?)'\"".contains(c),
    }
}

/// Whether `rest` starts with `name`, ignoring case; the byte length of
/// the match in `rest`.
fn starts_with_name(rest: &str, name: &str) -> Option<usize> {
    let mut it = rest.char_indices();
    for nc in name.chars() {
        let (_, rc) = it.next()?;
        if !rc.to_lowercase().eq(nc.to_lowercase()) {
            return None;
        }
    }
    Some(it.next().map_or(rest.len(), |(i, _)| i))
}

/// The live agents named `name` (ignoring case), the author left out;
/// `None` when the author was the only one.
fn named<'a>(agents: &'a [AgentCard], author: &str, name: &str) -> Option<Vec<&'a AgentCard>> {
    let low = name.to_lowercase();
    let all: Vec<&AgentCard> = agents
        .iter()
        .filter(|a| live(a) && !a.name.is_empty() && a.name.to_lowercase() == low)
        .collect();
    let others: Vec<&AgentCard> = all.iter().copied().filter(|a| a.key != author).collect();
    if others.is_empty() && !all.is_empty() {
        return None;
    }
    Some(others)
}

/// Resolves `given` (keys or names) and the @name tokens in `text` against
/// `agents` (every device's roster), dropping `author`, `user:` keys and
/// irori.
pub fn resolve(agents: &[AgentCard], author: &str, text: &str, given: &[String]) -> Resolved {
    let mut r = Resolved::default();
    let add = |r: &mut Resolved, m: Mention| {
        if dropped(&m.key, author) || r.mentions.iter().any(|o| o.key == m.key) {
            return;
        }
        if r.mentions.len() < MAX_MENTIONS {
            r.mentions.push(m);
        } else if !r.misses.iter().any(|x| x.name == m.name) {
            r.misses.push(Miss {
                name: m.name,
                reason: MISS_CAP.into(),
                candidates: Vec::new(),
            });
        }
    };
    let miss = |r: &mut Resolved, name: &str, cands: &[&AgentCard]| {
        if r.misses.iter().any(|m| m.name.eq_ignore_ascii_case(name)) {
            return;
        }
        r.misses.push(Miss {
            name: name.to_string(),
            reason: if cands.is_empty() {
                MISS_UNKNOWN
            } else {
                MISS_AMBIGUOUS
            }
            .into(),
            candidates: cands.iter().map(|a| a.key.clone()).collect(),
        });
    };
    for g in given {
        let g = g.trim().trim_start_matches('@').trim();
        if g.is_empty() {
            continue;
        }
        if let Some(a) = agents.iter().find(|a| a.key == g) {
            add(
                &mut r,
                Mention {
                    key: a.key.clone(),
                    name: if a.name.is_empty() {
                        a.key.clone()
                    } else {
                        a.name.clone()
                    },
                    gone: !live(a),
                },
            );
            continue;
        }
        if dropped(g, author) {
            continue;
        }
        if key_shaped(g) {
            miss(&mut r, g, &[]);
            continue;
        }
        match named(agents, author, g) {
            None => {}
            Some(c) if c.len() == 1 => add(
                &mut r,
                Mention {
                    key: c[0].key.clone(),
                    name: g.to_string(),
                    gone: false,
                },
            ),
            Some(c) => miss(&mut r, g, &c),
        }
    }
    // Live names, longest first, so "@cedar two" beats "@cedar".
    let mut names: Vec<&str> = agents
        .iter()
        .filter(|a| live(a) && !a.name.is_empty())
        .map(|a| a.name.as_str())
        .collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.chars().count()));
    names.dedup();
    let mut prev: Option<char> = None;
    for (i, c) in text.char_indices() {
        let starts = c == '@' && prev.is_none_or(|p| p.is_whitespace() || p == '(');
        prev = Some(c);
        if !starts {
            continue;
        }
        let rest = &text[i + 1..];
        let mut hit: Option<(&str, &str)> = None;
        for n in &names {
            if let Some(len) = starts_with_name(rest, n)
                && ends_token(rest[len..].chars().next())
            {
                hit = Some((&rest[..len], n));
                break;
            }
        }
        match hit {
            Some((written, n)) => match named(agents, author, n) {
                None => {}
                Some(c) if c.len() == 1 => add(
                    &mut r,
                    Mention {
                        key: c[0].key.clone(),
                        name: written.to_string(),
                        gone: false,
                    },
                ),
                // Named by key in `given` already: not a miss.
                Some(c)
                    if r.mentions.iter().any(|m| {
                        m.name.to_lowercase() == written.to_lowercase()
                            || c.iter().any(|a| a.key == m.key)
                    }) => {}
                Some(c) => miss(&mut r, written, &c),
            },
            None => {
                let word: String = rest
                    .chars()
                    .take_while(|c| !c.is_whitespace())
                    .collect::<String>()
                    .trim_end_matches(|c: char| ".,;:!?)'\"".contains(c))
                    .to_string();
                if !word.is_empty() {
                    miss(&mut r, &word, &[]);
                }
            }
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(key: &str, name: &str, status: &str) -> AgentCard {
        AgentCard {
            key: key.into(),
            name: name.into(),
            status: status.into(),
            ..AgentCard::default()
        }
    }

    fn roster() -> Vec<AgentCard> {
        vec![
            card("pc/s1", "cedar", "idle"),
            card("mac/s2", "lotus", "working"),
            card("pc/s3", "maple", "idle"),
            card("mac/s4", "maple", "idle"),
            card("pc/s5", "fern", "gone"),
            card("pc/s6", "big oak", "idle"),
            card("pc/me", "me", "working"),
        ]
    }

    fn keys(r: &Resolved) -> Vec<&str> {
        r.mentions.iter().map(|m| m.key.as_str()).collect()
    }

    #[test]
    fn text_tokens() {
        let r = resolve(
            &roster(),
            "pc/me",
            "@Cedar and @lotus, look (@big oak).",
            &[],
        );
        assert_eq!(keys(&r), ["pc/s1", "mac/s2", "pc/s6"]);
        assert_eq!(r.mentions[0].name, "Cedar");
        assert_eq!(r.mentions[2].name, "big oak");
        assert!(r.misses.is_empty());
    }

    #[test]
    fn not_tokens() {
        let r = resolve(&roster(), "pc/me", "mail a@cedar.com or x@lotus", &[]);
        assert!(r.mentions.is_empty() && r.misses.is_empty());
        // A name must end at a boundary: @cedar2 names nobody.
        let r = resolve(&roster(), "pc/me", "@cedar2 hi", &[]);
        assert!(r.mentions.is_empty());
        assert_eq!(r.misses[0].name, "cedar2");
        assert_eq!(r.misses[0].reason, MISS_UNKNOWN);
    }

    #[test]
    fn ambiguous_lists_keys() {
        let r = resolve(&roster(), "pc/me", "@maple?", &[]);
        assert!(r.mentions.is_empty());
        assert_eq!(r.misses[0].reason, MISS_AMBIGUOUS);
        assert_eq!(r.misses[0].candidates, ["pc/s3", "mac/s4"]);
        // By key it works, and the text's @maple is no miss then.
        let r = resolve(&roster(), "pc/me", "@maple", &["mac/s4".into()]);
        assert_eq!(keys(&r), ["mac/s4"]);
        assert!(r.misses.is_empty(), "{:?}", r.misses);
    }

    #[test]
    fn gone_and_unknown() {
        // A gone agent's name names nobody; its key is stored gone.
        let r = resolve(
            &roster(),
            "pc/me",
            "@fern",
            &["pc/s5".into(), "pc/nope".into()],
        );
        assert_eq!(keys(&r), ["pc/s5"]);
        assert!(r.mentions[0].gone);
        let names: Vec<&str> = r.misses.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["pc/nope", "fern"]);
        assert!(r.misses[0].is_key() && !r.misses[1].is_key());
    }

    #[test]
    fn drops_self_people_irori_and_dupes() {
        let r = resolve(
            &roster(),
            "pc/me",
            "@me @cedar @cedar",
            &[
                "user:pc".into(),
                "irori".into(),
                "pc/s1".into(),
                "@me".into(),
            ],
        );
        assert_eq!(keys(&r), ["pc/s1"]);
        assert!(r.misses.is_empty());
    }

    #[test]
    fn at_most_ten() {
        let agents: Vec<AgentCard> = (0..12)
            .map(|i| card(&format!("pc/a{i}"), &format!("n{i}"), "idle"))
            .collect();
        let text: String = (0..12).map(|i| format!("@n{i} ")).collect();
        let r = resolve(&agents, "pc/me", &text, &[]);
        assert_eq!(r.mentions.len(), MAX_MENTIONS);
        // The ones past the cap are reported, not dropped quietly.
        let over: Vec<(&str, &str)> = r
            .misses
            .iter()
            .map(|m| (m.name.as_str(), m.reason.as_str()))
            .collect();
        assert_eq!(over, [("n10", MISS_CAP), ("n11", MISS_CAP)]);
    }
}
