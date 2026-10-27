use crate::*;
pub const MAX_SLICE_MICROSECONDS: u64 = 4_000;
pub const CANCELLATION_ACCEPTANCE_MILLISECONDS: u64 = 50;
pub const MAX_QUEUED_REQUESTS: usize = 1;
pub const MAX_PREPARED_CANDIDATES: usize = 1;
pub const MAX_RETAINED_SOURCES: usize = 2;
pub const MAX_CACHE_BYTES: usize = 32 * 1024 * 1024;
#[derive(Debug)]
pub struct SliceBudget {
    limit: u32,
    used: u32,
}
impl SliceBudget {
    pub fn new(limit: u32) -> ContractResult<Self> {
        if limit == 0 || limit > MAX_WORK_UNITS_PER_SLICE {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/slice",
                "invalid work slice limit",
            ));
        }
        Ok(Self { limit, used: 0 })
    }
    pub fn admit(&mut self, elapsed_microseconds: u64, cancelled: bool) -> bool {
        if cancelled || elapsed_microseconds >= MAX_SLICE_MICROSECONDS || self.used >= self.limit {
            return false;
        }
        self.used += 1;
        true
    }
}
