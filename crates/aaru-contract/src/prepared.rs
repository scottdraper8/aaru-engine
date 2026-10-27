//! Prepared imports are inert until a matching explicit activation succeeds.
use crate::*;

#[derive(Debug, Default)]
pub struct PreparedRegistry {
    pending: Option<(PreparedToken, Counter, Digest)>,
}
impl PreparedRegistry {
    pub fn prepare(&mut self, preview: &ImportPreview, current: Counter) -> ContractResult<()> {
        if preview.replaces_generation != current {
            return Err(ContractError::new(
                IssueCode::StaleGeneration,
                "/replaces_generation",
                "preview generation is stale",
            ));
        }
        if wire::canonical_backup_digest(&preview.account)? != preview.canonical_digest {
            return Err(ContractError::new(
                IssueCode::InvalidSourceDigest,
                "/canonical_digest",
                "preview digest differs from backup",
            ));
        }
        if preview.issues.len() > MAX_DIAGNOSTICS {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/issues",
                "diagnostic budget exceeded",
            ));
        }
        self.pending = Some((
            preview.token.clone(),
            current,
            preview.canonical_digest.clone(),
        ));
        Ok(())
    }
    /// Consume only on success. Caller atomically swaps the already validated account
    /// and emits Activated at the returned generation; no old computation survives.
    pub fn activate(
        &mut self,
        token: &PreparedToken,
        current: Counter,
        storage: &StorageDisposition,
    ) -> ContractResult<Counter> {
        let Some((expected, generation, digest)) = &self.pending else {
            return Err(ContractError::new(
                IssueCode::StalePrepared,
                "/token",
                "no prepared account",
            ));
        };
        if token != expected || current != *generation {
            return Err(ContractError::new(
                IssueCode::StalePrepared,
                "/token",
                "prepared token or generation is stale",
            ));
        }
        if let StorageDisposition::Persisted { account_digest } = storage
            && account_digest != digest
        {
            return Err(ContractError::new(
                IssueCode::StorageFailure,
                "/storage/account_digest",
                "save acknowledgement does not identify prepared backup",
            ));
        }
        let next = current.next().ok_or_else(|| {
            ContractError::new(
                IssueCode::ResourceLimit,
                "/account_generation",
                "generation exhausted",
            )
        })?;
        self.pending = None;
        Ok(next)
    }
    pub fn clear(&mut self) {
        self.pending = None;
    }
}
