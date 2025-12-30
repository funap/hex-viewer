#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SearchMode {
    Text,
    Hex,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum SearchLimit {
    /// Limit to a maximum number of results
    Count(usize),
    /// Limit to results within N bytes from the first match
    Range(usize),
    /// No limit
    Unlimited,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SearchOptions {
    pub mode: SearchMode,
    pub limit: SearchLimit,
    pub range: Option<std::ops::Range<usize>>,
}

#[allow(dead_code)]
impl SearchOptions {
    pub fn new(mode: SearchMode) -> Self {
        Self {
            mode,
            limit: SearchLimit::Unlimited,
            range: None,
        }
    }

    pub fn with_count_limit(mode: SearchMode, max_results: usize) -> Self {
        Self {
            mode,
            limit: SearchLimit::Count(max_results),
            range: None,
        }
    }

    pub fn with_range_limit(mode: SearchMode, range_bytes: usize) -> Self {
        Self {
            mode,
            limit: SearchLimit::Range(range_bytes),
            range: None,
        }
    }

    pub fn with_range(mode: SearchMode, range: std::ops::Range<usize>) -> Self {
        Self {
            mode,
            limit: SearchLimit::Unlimited,
            range: Some(range),
        }
    }
}

/// A stateless function to find occurrences of a pattern in a byte slice.
pub fn find_occurrences(data: &[u8], pattern: &[u8], limit: SearchLimit, range: Option<std::ops::Range<usize>>) -> Vec<usize> {
    if pattern.is_empty() || pattern.len() > data.len() {
        return Vec::new();
    }

    let mut results = Vec::new();
    let pattern_len = pattern.len();
    let data_len = data.len();

    // Determine search range
    let (start, end) = if let Some(r) = range {
        (r.start.min(data_len), r.end.min(data_len))
    } else {
        (0, data_len)
    };

    if start >= end || end < pattern_len {
        return Vec::new();
    }

    let search_end = end - pattern_len;
    let mut first_match: Option<usize> = None;

    // Ensure start is within bounds for the loop
    let start = start.min(search_end + 1);

    for i in start..=search_end {
        if &data[i..i + pattern_len] == pattern {
            // Track first match for range-based limiting
            if first_match.is_none() {
                first_match = Some(i);
            }

            // Check limit
            match limit {
                SearchLimit::Count(max) => {
                    if results.len() >= max {
                        break;
                    }
                    results.push(i);
                }
                SearchLimit::Range(range_bytes) => {
                    if let Some(first) = first_match {
                        if i >= first + range_bytes {
                            break;
                        }
                    }
                    results.push(i);
                }
                SearchLimit::Unlimited => {
                    results.push(i);
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_occurrences_text() {
        let data = b"Hello World Hello";
        let pattern = b"Hello";
        let results = find_occurrences(data, pattern, SearchLimit::Unlimited, None);
        assert_eq!(results, vec![0, 12]);
    }

    #[test]
    fn test_find_occurrences_limit_count() {
        let data = b"AA AA AA AA";
        let pattern = b"AA";
        let results = find_occurrences(data, pattern, SearchLimit::Count(2), None);
        assert_eq!(results, vec![0, 3]);
    }

    #[test]
    fn test_find_occurrences_limit_range() {
        let data = b"AA..AA....AA";
        let pattern = b"AA";
        // First match at 0. Range limit 5 means look until index 0 + 5.
        // Second match at 4. (4 < 5) should be found.
        // Third match at 10. (10 >= 5) should be excluded.
        let results = find_occurrences(data, pattern, SearchLimit::Range(5), None);
        assert_eq!(results, vec![0, 4]);
    }

    #[test]
    fn test_find_occurrences_range_restriction() {
        let data = b"0123456789";
        let pattern = b"34";

        // Range inclusive of match
        let results = find_occurrences(data, pattern, SearchLimit::Unlimited, Some(2..6));
        assert_eq!(results, vec![3]);

        // Range exclusive of match (start after)
        let results = find_occurrences(data, pattern, SearchLimit::Unlimited, Some(5..8));
        assert!(results.is_empty());

        // Range exclusive of match (end before)
        let results = find_occurrences(data, pattern, SearchLimit::Unlimited, Some(0..3));
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_pattern_or_data() {
        assert!(find_occurrences(b"", b"test", SearchLimit::Unlimited, None).is_empty());
        assert!(find_occurrences(b"data", b"", SearchLimit::Unlimited, None).is_empty());
    }
}
