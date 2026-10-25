//! Did-you-mean suggestions.

/// The nearest candidate by edit distance, if it is close enough to be a typo.
#[must_use]
pub fn closest<'a>(word: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let word16: Vec<u16> = word.to_lowercase().encode_utf16().collect();
    let mut best = None;
    let mut best_distance = (word.encode_utf16().count() / 3).max(2) + 1;
    for &c in candidates {
        let c16: Vec<u16> = c.to_lowercase().encode_utf16().collect();
        let d = levenshtein(&word16, &c16);
        if d < best_distance {
            best = Some(c);
            best_distance = d;
        }
    }
    best
}

fn levenshtein(a: &[u16], b: &[u16]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut row = vec![i];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            row.push((prev[j] + 1).min(row[j - 1] + 1).min(prev[j - 1] + cost));
        }
        prev = row;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_a_near_candidate_or_none() {
        assert_eq!(closest("resorces", &["resources", "readme"]), Some("resources"));
        assert_eq!(closest("qqqqqq", &["resources"]), None);
        assert_eq!(closest("", &[]), None);
    }
}
