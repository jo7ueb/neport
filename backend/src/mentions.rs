use std::collections::HashSet;

use regex::Regex;

// Matches the "@[Display Name](user:123)" tokens MarkdownField's mention
// autocomplete inserts into comment/description/report text - mirrors the
// unambiguous "#123" ticket-ref convention already used the same way, since
// a bare "@Display Name" would leave the name's end boundary ambiguous
// (display names can contain spaces).
fn mention_regex() -> Regex {
    Regex::new(r"@\[[^\]]*\]\(user:(\d+)\)").expect("mention regex is valid")
}

pub fn extract_mentioned_user_ids(text: &str) -> HashSet<i64> {
    mention_regex()
        .captures_iter(text)
        .filter_map(|c| c[1].parse::<i64>().ok())
        .collect()
}

// Users mentioned in `new_text` but not already in `old_text` - so editing a
// comment/description/report doesn't re-notify someone who was already
// mentioned before the edit. Pass "" as `old_text` for a brand new text.
pub fn newly_mentioned(old_text: &str, new_text: &str) -> Vec<i64> {
    let old_ids = extract_mentioned_user_ids(old_text);
    extract_mentioned_user_ids(new_text)
        .into_iter()
        .filter(|id| !old_ids.contains(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_multiple_distinct_mentions() {
        let text = "Hey @[Taro Yamada](user:42), please loop in @[Bob](user:7) too.";
        let ids = extract_mentioned_user_ids(text);
        assert_eq!(ids, HashSet::from([42, 7]));
    }

    #[test]
    fn dedupes_repeated_mentions() {
        let text = "@[Bob](user:7) ping @[Bob](user:7) again";
        let ids = extract_mentioned_user_ids(text);
        assert_eq!(ids, HashSet::from([7]));
    }

    #[test]
    fn ignores_text_without_mentions() {
        assert!(extract_mentioned_user_ids("no mentions here, just #123").is_empty());
    }

    #[test]
    fn newly_mentioned_excludes_ids_already_present_before() {
        let old = "@[Bob](user:7)";
        let new = "@[Bob](user:7) and @[Alice](user:9)";
        assert_eq!(newly_mentioned(old, new), vec![9]);
    }

    #[test]
    fn newly_mentioned_from_empty_old_returns_all() {
        let new = "@[Bob](user:7) and @[Alice](user:9)";
        let mut ids = newly_mentioned("", new);
        ids.sort_unstable();
        assert_eq!(ids, vec![7, 9]);
    }

    #[test]
    fn newly_mentioned_is_empty_when_unchanged() {
        let text = "@[Bob](user:7)";
        assert!(newly_mentioned(text, text).is_empty());
    }
}
