use serde::{Deserialize, Serialize};

const DEFAULT_PER_PAGE: i64 = 25;
// Internal safety cap, well above the largest UI-offered page size (100).
// "Fetch everything" call sites (assignee dropdowns, kanban board, dashboard
// aggregation) request this explicitly via `per_page` rather than bypassing
// pagination, so the response shape stays uniform everywhere.
const MAX_PER_PAGE: i64 = 500;

// Used by internal server-side aggregations (e.g. the Gantt chart) that
// genuinely need every row rather than a paginated slice.
pub const FETCH_ALL_LIMIT: i64 = MAX_PER_PAGE;

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl PageQuery {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(DEFAULT_PER_PAGE).clamp(1, MAX_PER_PAGE)
    }

    pub fn limit(&self) -> i64 {
        self.per_page()
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl<T> Paginated<T> {
    pub fn new(items: Vec<T>, total: i64, q: &PageQuery) -> Self {
        Self {
            items,
            total,
            page: q.page(),
            per_page: q.per_page(),
        }
    }
}
