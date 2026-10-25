//! Forge group names for stainless.yml resource paths (ADR 0003, `$client` amendment).
//!
//! Forge drops an operation whose `x-fern-sdk-group-name` is empty, and splits names into words
//! (`$client` becomes the group `client`, merging with a real `client` resource). Root client
//! methods therefore go to a reserved group; the plugin reads `x-ferrule-resource` instead.

use crate::unicode::is_letter_or_number;

/// The reserved Forge group of `$client` methods; Forge sees the words `ferrule client`.
pub const CLIENT_GROUP: &str = "$ferrule_client";

/// `s.replace(/(a)(b)/g, '$1 $2')` for single-character classes `a` and `b`, with a lookahead
/// class `c` for the character after `b` (`None`: no third character).
fn split_pairs(
    chars: &[char],
    first: fn(char) -> bool,
    second: fn(char) -> bool,
    third: Option<fn(char) -> bool>,
) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len() + 4);
    let mut i = 0;
    while i < chars.len() {
        let ok = first(chars[i])
            && chars.get(i + 1).is_some_and(|c| second(*c))
            && third.is_none_or(|t| chars.get(i + 2).is_some_and(|c| t(*c)));
        if ok {
            out.push(chars[i]);
            out.push(' ');
            let width = if third.is_some() { 2 } else { 1 };
            out.extend_from_slice(&chars[i + 1..=i + width]);
            i += 1 + width;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// The words Forge's `Name::new` splits a name into (letters/digits runs and case changes).
#[must_use]
pub fn forge_words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let a = split_pairs(
        &chars,
        |c| c.is_ascii_lowercase() || c.is_ascii_digit(),
        |c| c.is_ascii_uppercase(),
        None,
    );
    let b = split_pairs(
        &a,
        |c| c.is_ascii_uppercase(),
        |c| c.is_ascii_uppercase(),
        Some(|c| c.is_ascii_lowercase()),
    );
    let lower = b.into_iter().collect::<String>().to_lowercase();
    lower
        .split(|c: char| !is_letter_or_number(c))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// `x-fern-sdk-group-name` for a resource path: `$client` maps to [`CLIENT_GROUP`].
#[must_use]
pub fn forge_group_path(path: &[String]) -> Vec<String> {
    path.iter()
        .enumerate()
        .map(|(i, seg)| {
            if i == 0 && seg == "$client" {
                CLIENT_GROUP.to_owned()
            } else {
                seg.clone()
            }
        })
        .collect()
}

/// Whether a top-level resource name would land in the same Forge group as `$client`.
#[must_use]
pub fn collides_with_client_group(name: &str) -> bool {
    name != "$client" && forge_words(name).join(" ") == forge_words(CLIENT_GROUP).join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_names_into_words_as_forge_does() {
        assert_eq!(forge_words("$ferrule_client"), ["ferrule", "client"]);
        assert_eq!(forge_words("XMLDocument"), ["xml", "document"]);
        assert_eq!(forge_words("ipv4Cidr"), ["ipv4", "cidr"]);
    }

    #[test]
    fn maps_a_leading_client_to_the_reserved_group_only() {
        let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        assert_eq!(forge_group_path(&s(&["$client"])), [CLIENT_GROUP]);
        assert_eq!(forge_group_path(&s(&["$client", "x"])), [CLIENT_GROUP, "x"]);
        assert_eq!(forge_group_path(&s(&["a", "$client"])), ["a", "$client"]);
    }

    #[test]
    fn flags_resources_that_share_the_reserved_group() {
        assert!(collides_with_client_group("ferrule_client"));
        assert!(collides_with_client_group("FerruleClient"));
        assert!(!collides_with_client_group("$client"));
        assert!(!collides_with_client_group("client"));
    }
}
