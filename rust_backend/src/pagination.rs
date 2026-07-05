use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.per_page.clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        ((self.page.clamp(1, i64::MAX) - 1) * self.limit()).clamp(0, i64::MAX)
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: i64, per_page: i64, total: i64) -> Self {
        let per_page = per_page.clamp(1, 100);
        let pages = (total + per_page - 1) / per_page;
        Self {
            data,
            page,
            per_page,
            total,
            pages,
        }
    }
}
