//! Generic utilities used across core modules.

/// Filter items by case-insensitive query matching on two string fields.
/// Returns all items when query is empty.
pub fn filter_by_query<'a, T, F>(items: &'a [T], query: &str, get_fields: F) -> Vec<&'a T>
where
    F: Fn(&'a T) -> (&str, &str),
{
    if query.is_empty() {
        return items.iter().collect();
    }
    let q = query.to_lowercase();
    items
        .iter()
        .filter(|item| {
            let (a, b) = get_fields(item);
            a.to_lowercase().contains(&q) || b.to_lowercase().contains(&q)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_empty_query_returns_all() {
        let items = vec!["a", "b", "c"];
        let out = filter_by_query(&items, "", |s| (s, ""));
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn filter_match_first_field() {
        let items = vec!["hello", "world"];
        let out = filter_by_query(&items, "hel", |s| (s, ""));
        assert_eq!(out, vec![&"hello"]);
    }

    #[test]
    fn filter_match_second_field() {
        let items = vec![("a", "hello"), ("b", "world")];
        let out = filter_by_query(&items, "orld", |t| (t.0, t.1));
        assert_eq!(out, vec![&("b", "world")]);
    }

    #[test]
    fn filter_case_insensitive() {
        let items = vec!["Hello", "World"];
        let out = filter_by_query(&items, "world", |s| (s, ""));
        assert_eq!(out, vec![&"World"]);
    }

    #[test]
    fn filter_no_match_returns_empty() {
        let items = vec!["hello", "world"];
        let out = filter_by_query(&items, "xyz", |s| (s, ""));
        assert!(out.is_empty());
    }
}
