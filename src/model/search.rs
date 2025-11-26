#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SearchMode {
    Text,
    Hex,
}

#[derive(Clone, Copy, Debug)]
pub enum SearchLimit {
    /// Limit to a maximum number of results
    Count(usize),
    /// Limit to results within N bytes from the first match
    Range(usize),
    /// No limit
    Unlimited,
}

#[derive(Clone, Debug)]
pub struct SearchOptions {
    pub mode: SearchMode,
    pub limit: SearchLimit,
}

impl SearchOptions {
    pub fn new(mode: SearchMode) -> Self {
        Self {
            mode,
            limit: SearchLimit::Unlimited,
        }
    }

    pub fn with_count_limit(mode: SearchMode, max_results: usize) -> Self {
        Self {
            mode,
            limit: SearchLimit::Count(max_results),
        }
    }

    pub fn with_range_limit(mode: SearchMode, range_bytes: usize) -> Self {
        Self {
            mode,
            limit: SearchLimit::Range(range_bytes),
        }
    }
}
