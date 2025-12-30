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
            results.push(i);

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
                }
                SearchLimit::Range(range_bytes) => {
                    if let Some(first) = first_match {
                        if i >= first + range_bytes {
                            break;
                        }
                    }
                }
                SearchLimit::Unlimited => {
                    // Continue searching
                }
            }
        }
    }

    results
}
