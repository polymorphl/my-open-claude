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
