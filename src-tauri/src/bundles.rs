//! Spotting the same rule twice (a condition and its Rules Glossary term, an entry two packs both have): the same
//! name, ignoring [tags], case and a plural s, and at least 80% of the shorter text repeated in order in the other.
//! Only links entries for the Catalog; the data is never changed.

/// The least text two same-named entries must share to be one rule (see `similarity`).
const SAME_RULE: u32 = 80;

/// Name for spotting duplicates: no [tag], no plural s, lowercase (as `docs/convert_srd_rules.py`'s `norm`).
pub fn norm(name: &str) -> String {
    let mut plain = String::new();
    let mut in_tag = false;
    for c in name.chars() {
        match c {
            '[' => in_tag = true,
            ']' => in_tag = false,
            '\'' => {}
            _ if !in_tag => plain.extend(c.to_lowercase()),
            _ => {}
        }
    }
    let plain: String = plain.chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() { c } else { ' ' }).collect();
    let singular = |w: &str| {
        let plural = w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") && !w.ends_with("us");
        if plural { w[..w.len() - 1].to_string() } else { w.to_string() }
    };
    plain.split_whitespace().map(singular).collect::<Vec<_>>().join(" ")
}

fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

/// How much of the shorter word list the other repeats in order, 0–100: the longest common word subsequence.
fn similarity(a: &[String], b: &[String]) -> u32 {
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let mut row = vec![0u32; b.len() + 1];
    for wa in a {
        let mut diagonal = 0;
        for (j, wb) in b.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = if wa == wb { diagonal + 1 } else { up.max(row[j]) };
            diagonal = up;
        }
    }
    (100.0 * row[b.len()] as f64 / a.len().min(b.len()) as f64).round() as u32
}

/// For texts of entries that share a `norm`ed name: for each text, the group it belongs to (the lowest index in it).
/// Connected: a like b and b like c puts all three together. A text alone is its own group.
pub fn groups(texts: &[&str]) -> Vec<usize> {
    let words: Vec<_> = texts.iter().map(|t| words_of(t)).collect();
    let mut group: Vec<usize> = (0..texts.len()).collect();
    for a in 0..texts.len() {
        for b in a + 1..texts.len() {
            if group[a] != group[b] && similarity(&words[a], &words[b]) >= SAME_RULE {
                let (keep, merge) = (group[a].min(group[b]), group[a].max(group[b]));
                group.iter_mut().filter(|g| **g == merge).for_each(|g| *g = keep);
            }
        }
    }
    group
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_and_similarity_match_the_conversion_script() {
        assert_eq!(norm("Blinded [Condition]"), "blinded");
        assert_eq!(norm("Breaking Objects"), "breaking object");
        assert_eq!(norm("Bonus Actions"), "bonus action");
        assert_eq!(norm("Grass"), "grass");
        assert_eq!(norm("Creature's Status"), "creature status");
        let w = |s: &str| words_of(s);
        assert_eq!(similarity(&w("a b c d"), &w("x a b y c d z")), 100);
        assert_eq!(similarity(&w("a b c d"), &w("a b")), 100);
        assert_eq!(similarity(&w("a b c d"), &w("a x c y")), 50);
        assert_eq!(similarity(&w(""), &w("a")), 0);
    }

    #[test]
    fn groups_join_texts_that_are_alike_also_through_a_third() {
        let a = "one two three four five";
        let b = "one two three four five six seven";
        let c = "one two three four five six seven eight";
        assert_eq!(groups(&[a, "something else entirely", b, c]), [0, 1, 0, 0]);
        assert_eq!(groups(&["x y", ""]), [0, 1]);
    }
}
