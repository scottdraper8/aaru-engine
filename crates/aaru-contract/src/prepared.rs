use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleBinding {
    pub id: SampleId,
    pub inventory_digest: Digest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    pub account: AccountSnapshot,
    pub sample: Option<SampleBinding>,
}
fn inventory_digest(account: &AccountSnapshot) -> ContractResult<Digest> {
    let mut inventory = account.clone();
    inventory.preferences = AllocationPreferences::default();
    wire::snapshot_digest(&inventory)
}
impl Backup {
    pub fn from_validated(account: &ValidatedAccount) -> ContractResult<Self> {
        Ok(Self {
            account: account.snapshot().clone(),
            sample: account
                .sample()
                .map(|id| {
                    Ok(SampleBinding {
                        id,
                        inventory_digest: inventory_digest(account.snapshot())?,
                    })
                })
                .transpose()?,
        })
    }
    pub fn restore(&self, bundled: Option<&ValidatedAccount>) -> ContractResult<ValidatedAccount> {
        match &self.sample {
            None => ValidatedAccount::imported(self.account.clone()),
            Some(binding) => {
                let reference = bundled.ok_or_else(|| {
                    failure(
                        IssueCode::SyntheticOnImportedAccount,
                        "sample restoration needs the trusted bundled reference",
                    )
                })?;
                if reference.sample() != Some(binding.id)
                    || inventory_digest(reference.snapshot())? != binding.inventory_digest
                    || inventory_digest(&self.account)? != binding.inventory_digest
                {
                    return Err(failure(
                        IssueCode::SyntheticOnImportedAccount,
                        "sample backup differs from bundled inventory",
                    ));
                }
                ValidatedAccount::bundled(self.account.clone(), binding.id)
            }
        }
    }
    pub fn digest(&self) -> ContractResult<Digest> {
        self.account.validate()?;
        Ok(Digest::of(wire::encode(self)?.as_bytes()))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableRecord {
    pub revision: Counter,
    pub backup: Option<Backup>,
}
impl DurableRecord {
    pub fn digest(&self) -> ContractResult<Digest> {
        if let Some(backup) = &self.backup {
            backup.digest()?;
        }
        Ok(Digest::of(wire::encode(self)?.as_bytes()))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitReceipt {
    pub epochs: Epochs,
    pub revision: Counter,
    pub record_digest: Digest,
    pub backup_digest: Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitPhase {
    Idle,
    Prepared,
    Writing,
    Committed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecoveryState {
    Durable { record: Box<DurableRecord> },
    SessionOnly { backup: Option<Box<Backup>> },
}
#[derive(Debug)]
enum StorageCandidate {
    Durable(DurableRecord),
    SessionOnly(Backup),
}
#[derive(Debug)]
pub struct CommitMachine {
    epochs: Epochs,
    durable: Option<DurableRecord>,
    active: Option<Backup>,
    candidate: Option<StorageCandidate>,
    phase: CommitPhase,
}
impl CommitMachine {
    pub fn new(epochs: Epochs, primary: bool, durable: DurableRecord) -> ContractResult<Self> {
        if !primary {
            return Err(failure(
                IssueCode::StaleAuthority,
                "secondary tabs cannot open private account state",
            ));
        }
        durable.digest()?;
        Ok(Self {
            epochs,
            active: durable.backup.clone(),
            durable: Some(durable),
            candidate: None,
            phase: CommitPhase::Idle,
        })
    }
    pub fn session_only(epochs: Epochs, primary: bool) -> ContractResult<Self> {
        if !primary {
            return Err(failure(
                IssueCode::StaleAuthority,
                "session-only state requires primary ownership",
            ));
        }
        Ok(Self {
            epochs,
            durable: None,
            active: None,
            candidate: None,
            phase: CommitPhase::Idle,
        })
    }
    pub fn recovery_state(&self) -> RecoveryState {
        match &self.durable {
            Some(record) => RecoveryState::Durable {
                record: Box::new(record.clone()),
            },
            None => RecoveryState::SessionOnly {
                backup: self.active.clone().map(Box::new),
            },
        }
    }
    pub fn epochs(&self) -> &Epochs {
        &self.epochs
    }
    pub fn phase(&self) -> &CommitPhase {
        &self.phase
    }
    pub fn active(&self) -> Option<&Backup> {
        self.active.as_ref()
    }
    pub fn durable(&self) -> Option<&DurableRecord> {
        self.durable.as_ref()
    }
    pub fn prepare(&mut self, backup: Option<Backup>, epochs: &Epochs) -> ContractResult<()> {
        if epochs != &self.epochs
            || self.phase == CommitPhase::Writing
            || self.phase == CommitPhase::Committed
        {
            return Err(failure(
                IssueCode::StaleAuthority,
                "write transition blocks preparation",
            ));
        }
        if let Some(b) = &backup {
            b.digest()?;
        }
        self.candidate = Some(match &self.durable {
            Some(durable) => StorageCandidate::Durable(DurableRecord {
                revision: durable.revision.next().ok_or_else(|| {
                    failure(IssueCode::ResourceLimit, "storage revision exhausted")
                })?,
                backup,
            }),
            None => StorageCandidate::SessionOnly(backup.ok_or_else(|| {
                failure(
                    IssueCode::StorageFailure,
                    "unavailable storage cannot commit a clear-local-data tombstone",
                )
            })?),
        });
        self.phase = CommitPhase::Prepared;
        Ok(())
    }
    pub fn invalidate_prepared(&mut self, epochs: Epochs) -> ContractResult<()> {
        if matches!(self.phase, CommitPhase::Writing | CommitPhase::Committed)
            || !epochs.succeeds(&self.epochs)
        {
            return Err(failure(
                IssueCode::StaleAuthority,
                "settle committed or in-flight storage before changing epochs",
            ));
        }
        self.epochs = epochs;
        self.candidate = None;
        self.phase = CommitPhase::Idle;
        Ok(())
    }
    pub fn begin_save(&mut self, epochs: &Epochs) -> ContractResult<&DurableRecord> {
        if epochs != &self.epochs || self.phase != CommitPhase::Prepared {
            return Err(failure(
                IssueCode::StaleAuthority,
                "no authorized prepared save",
            ));
        }
        let Some(StorageCandidate::Durable(record)) = &self.candidate else {
            return Err(failure(
                IssueCode::StorageFailure,
                "session-only preparation cannot be saved without a durable read",
            ));
        };
        self.phase = CommitPhase::Writing;
        Ok(record)
    }
    pub fn storage_failed(&mut self, epochs: &Epochs) -> ContractResult<()> {
        if epochs != &self.epochs || self.phase != CommitPhase::Writing {
            return Err(failure(IssueCode::StaleAuthority, "stale storage failure"));
        }
        self.phase = CommitPhase::Prepared;
        Ok(())
    }
    pub fn storage_committed(&mut self, receipt: &CommitReceipt) -> ContractResult<()> {
        if receipt.epochs != self.epochs || self.phase != CommitPhase::Writing {
            return Err(failure(IssueCode::StaleAuthority, "stale commit callback"));
        }
        let Some(StorageCandidate::Durable(candidate)) = &self.candidate else {
            return Err(failure(
                IssueCode::StalePrepared,
                "missing durable candidate",
            ));
        };
        let backup_digest = match &candidate.backup {
            Some(b) => b.digest()?,
            None => Digest::of(b"aaru-reset-v1"),
        };
        if candidate.revision != receipt.revision
            || candidate.digest()? != receipt.record_digest
            || backup_digest != receipt.backup_digest
        {
            return Err(failure(
                IssueCode::StorageFailure,
                "commit receipt differs from candidate",
            ));
        }
        self.durable = Some(candidate.clone());
        self.phase = CommitPhase::Committed;
        Ok(())
    }
    pub fn activate_committed(&mut self) -> ContractResult<()> {
        if self.phase != CommitPhase::Committed {
            return Err(failure(
                IssueCode::StorageFailure,
                "durable commit has not completed",
            ));
        }
        self.active = self.durable.as_ref().and_then(|r| r.backup.clone());
        self.candidate = None;
        self.phase = CommitPhase::Idle;
        Ok(())
    }
    pub fn activate_session_only(&mut self) -> ContractResult<()> {
        if self.phase != CommitPhase::Prepared {
            return Err(failure(
                IssueCode::StorageFailure,
                "session activation requires an uncommitted candidate",
            ));
        }
        let backup = match &self.candidate {
            Some(StorageCandidate::Durable(record)) => record.backup.as_ref(),
            Some(StorageCandidate::SessionOnly(backup)) => Some(backup),
            None => None,
        }
        .ok_or_else(|| {
            failure(
                IssueCode::StorageFailure,
                "clear local data requires a durable tombstone",
            )
        })?;
        self.active = Some(backup.clone());
        self.candidate = None;
        self.phase = CommitPhase::Idle;
        Ok(())
    }
    pub fn settle_storage(
        &mut self,
        epochs: &Epochs,
        observed: DurableRecord,
    ) -> ContractResult<()> {
        if epochs != &self.epochs || self.phase != CommitPhase::Writing {
            return Err(failure(
                IssueCode::StaleAuthority,
                "no unsettled storage transaction",
            ));
        }
        let Some(StorageCandidate::Durable(candidate)) = &self.candidate else {
            return Err(failure(
                IssueCode::StorageFailure,
                "missing durable candidate",
            ));
        };
        if self.durable.as_ref() == Some(&observed) {
            self.phase = CommitPhase::Prepared;
        } else if candidate == &observed {
            self.durable = Some(observed);
            self.phase = CommitPhase::Committed;
        } else {
            return Err(failure(
                IssueCode::StorageFailure,
                "settled read is neither prior record nor complete candidate",
            ));
        }
        Ok(())
    }
    pub fn discard_session(&mut self, epochs: Epochs) -> ContractResult<()> {
        if self.durable.is_some() {
            return Err(failure(
                IssueCode::StorageFailure,
                "discard session cannot clear durable local data",
            ));
        }
        self.invalidate_prepared(epochs)?;
        self.active = None;
        Ok(())
    }
    pub fn recover(&mut self, epochs: Epochs, observed: RecoveryState) -> ContractResult<()> {
        if !epochs.succeeds(&self.epochs) || self.phase == CommitPhase::Writing {
            return Err(failure(
                IssueCode::StaleAuthority,
                "settle storage before recovery",
            ));
        }
        let (durable, active) = match observed {
            RecoveryState::Durable { record } => {
                record.digest()?;
                if self.durable.as_ref().is_some_and(|old| {
                    record.revision < old.revision
                        || (record.revision == old.revision && record.as_ref() != old)
                }) {
                    return Err(failure(
                        IssueCode::StorageFailure,
                        "storage revision or recovery authority is inconsistent",
                    ));
                }
                let active = record.backup.clone();
                (Some(*record), active)
            }
            RecoveryState::SessionOnly { backup } => {
                if self.durable.is_some() {
                    return Err(failure(
                        IssueCode::StorageFailure,
                        "known durable state requires durable recovery",
                    ));
                }
                if let Some(backup) = &backup {
                    backup.digest()?;
                }
                (None, backup.map(|b| *b))
            }
        };
        self.epochs = epochs;
        self.durable = durable;
        self.active = active;
        self.candidate = None;
        self.phase = CommitPhase::Idle;
        Ok(())
    }
}
fn failure(code: IssueCode, message: &str) -> ContractError {
    ContractError::new(code, "/prepared", message)
}

#[derive(Debug, Default)]
pub struct PreparedRegistry {
    pending: Option<(ImportPreview, ValidatedAccount)>,
    activated: Option<(PreparedToken, RequestContext, StorageDisposition, Counter)>,
    active: Option<ValidatedAccount>,
    issued: BTreeSet<PreparedToken>,
}
impl PreparedRegistry {
    pub fn prepare(
        &mut self,
        preview: &ImportPreview,
        current: Counter,
        context: &RequestContext,
        validated: ValidatedAccount,
    ) -> ContractResult<()> {
        if preview.context != *context || preview.replaces_generation != current {
            return Err(failure(IssueCode::StaleAuthority, "stale preview context"));
        }
        if preview.issues.len() > MAX_DIAGNOSTICS
            || self.issued.len() >= 4096
            || self.issued.contains(&preview.token)
        {
            return Err(failure(
                IssueCode::ResourceLimit,
                "prepared candidate budget or repeated token",
            ));
        }
        if preview.backup != Backup::from_validated(&validated)?
            || preview.backup.digest()? != preview.canonical_digest
        {
            return Err(failure(
                IssueCode::InvalidSourceDigest,
                "preview is not the validated backup",
            ));
        }
        self.issued.insert(preview.token.clone());
        self.pending = Some((preview.clone(), validated));
        Ok(())
    }
    pub fn activate(
        &mut self,
        token: &PreparedToken,
        current: Counter,
        context: &RequestContext,
        storage: &StorageDisposition,
    ) -> ContractResult<Counter> {
        if let Some((last, authority, disposition, generation)) = &self.activated
            && last == token
            && authority == context
            && disposition == storage
            && current == *generation
        {
            return Ok(*generation);
        }
        let (preview, _) = self
            .pending
            .as_ref()
            .ok_or_else(|| failure(IssueCode::StalePrepared, "no prepared candidate"))?;
        if &preview.token != token
            || preview.replaces_generation != current
            || &preview.context != context
        {
            return Err(failure(IssueCode::StalePrepared, "stale token or context"));
        }
        if let StorageDisposition::Persisted { receipt } = storage {
            let record = DurableRecord {
                revision: receipt.revision,
                backup: Some(preview.backup.clone()),
            };
            if receipt.epochs != context.epochs
                || receipt.backup_digest != preview.canonical_digest
                || receipt.record_digest != record.digest()?
            {
                return Err(failure(
                    IssueCode::StorageFailure,
                    "receipt is not bound to prepared account/source/preferences",
                ));
            }
        }
        let next = current
            .next()
            .ok_or_else(|| failure(IssueCode::ResourceLimit, "generation exhausted"))?;
        let (_, account) = self
            .pending
            .take()
            .ok_or_else(|| failure(IssueCode::StalePrepared, "missing candidate"))?;
        self.active = Some(account);
        self.activated = Some((token.clone(), context.clone(), storage.clone(), next));
        Ok(next)
    }
    pub fn active(&self) -> Option<&ValidatedAccount> {
        self.active.as_ref()
    }
    pub fn clear(&mut self) {
        self.pending = None;
        self.activated = None;
        self.active = None;
    }
}
