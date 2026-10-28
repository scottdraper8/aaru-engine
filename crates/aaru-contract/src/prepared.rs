use crate::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleBinding {
    pub id: SampleId,
    pub inventory_digest: Digest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    pub account: Arc<AccountSnapshot>,
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
            account: account.shared_snapshot(),
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
                        "sample restoration needs a fresh trusted bundled reference",
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
    pub backup: Option<Arc<Backup>>,
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
    ActivatingSession,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageObservation {
    Readable {
        record: Arc<DurableRecord>,
    },
    RetainedHead {
        head: transport::DurableHead,
    },
    Corrupt {
        head: transport::DurableHead,
    },
    Unreadable {
        last_known_head: Option<transport::DurableHead>,
    },
    Unavailable {
        last_observed_head: Option<transport::DurableHead>,
    },
}
impl StorageObservation {
    pub fn head(&self) -> ContractResult<Option<transport::DurableHead>> {
        match self {
            Self::Readable { record } => transport::DurableHead::from_record(record).map(Some),
            Self::RetainedHead { head } => Ok(Some(head.clone())),
            Self::Corrupt { head } => Ok(Some(head.clone())),
            Self::Unreadable { last_known_head } => Ok(last_known_head.clone()),
            Self::Unavailable { last_observed_head } => Ok(last_observed_head.clone()),
        }
    }
    pub fn head_observation(&self) -> ContractResult<transport::HeadObservation> {
        Ok(match self {
            Self::Readable { .. } | Self::RetainedHead { .. } => {
                transport::HeadObservation::Readable {
                    head: self
                        .head()?
                        .ok_or_else(|| failure(IssueCode::StorageFailure, "missing head"))?,
                }
            }
            Self::Corrupt { head } => transport::HeadObservation::Corrupt {
                readable_head: head.clone(),
            },
            Self::Unreadable { last_known_head } => transport::HeadObservation::Unreadable {
                last_known_head: last_known_head.clone(),
            },
            Self::Unavailable { last_observed_head } => transport::HeadObservation::Unavailable {
                last_readable: last_observed_head.clone(),
            },
        })
    }
    fn compact(self) -> ContractResult<Self> {
        match self {
            Self::Readable { record } => Ok(Self::RetainedHead {
                head: transport::DurableHead::from_record(&record)?,
            }),
            other => Ok(other),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActiveSelection {
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    ObservedDurable,
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    EmptySession,
    RetainedSession {
        backup: Arc<Backup>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryInput {
    pub storage: StorageObservation,
    pub active: ActiveSelection,
}
impl RecoveryInput {
    pub fn selected_backup(&self) -> ContractResult<Option<&Arc<Backup>>> {
        match &self.active {
            ActiveSelection::ObservedDurable => match &self.storage {
                StorageObservation::Readable { record } => Ok(record.backup.as_ref()),
                _ => Err(failure(
                    IssueCode::StorageFailure,
                    "unread storage is not an empty durable record",
                )),
            },
            ActiveSelection::EmptySession => Ok(None),
            ActiveSelection::RetainedSession { backup } => Ok(Some(backup)),
        }
    }
    pub fn restore(
        &self,
        bundled: Option<&ValidatedAccount>,
    ) -> ContractResult<Option<ValidatedAccount>> {
        self.selected_backup()?
            .map(|b| b.restore(bundled))
            .transpose()
    }
}
#[derive(Debug)]
pub struct CommitMachine {
    epochs: Epochs,
    storage: StorageObservation,
    active: Option<Arc<Backup>>,
    session_selected: bool,
    candidate: Option<Arc<DurableRecord>>,
    session_candidate: Option<Arc<Backup>>,
    phase: CommitPhase,
}
impl CommitMachine {
    pub fn new(epochs: Epochs, primary: bool, input: RecoveryInput) -> ContractResult<Self> {
        if !primary {
            return Err(failure(
                IssueCode::StaleAuthority,
                "private state requires primary ownership",
            ));
        }
        if matches!(
            input.storage,
            StorageObservation::Corrupt { .. } | StorageObservation::Unreadable { .. }
        ) && matches!(input.active, ActiveSelection::ObservedDurable)
        {
            return Err(failure(
                IssueCode::StorageFailure,
                "corrupt storage needs explicit memory selection or privacy clear",
            ));
        }
        let active = input.selected_backup()?.cloned();
        if let StorageObservation::Readable { record } = &input.storage {
            record.digest()?;
        }
        if let Some(backup) = &active {
            backup.digest()?;
        }
        Ok(Self {
            epochs,
            storage: if matches!(input.active, ActiveSelection::ObservedDurable) {
                input.storage
            } else {
                input.storage.compact()?
            },
            active,
            session_selected: !matches!(input.active, ActiveSelection::ObservedDurable),
            candidate: None,
            session_candidate: None,
            phase: CommitPhase::Idle,
        })
    }
    pub fn recovery_input(&self) -> RecoveryInput {
        let active = if self.phase == CommitPhase::Committed || !self.session_selected {
            ActiveSelection::ObservedDurable
        } else {
            self.active
                .clone()
                .map_or(ActiveSelection::EmptySession, |backup| {
                    ActiveSelection::RetainedSession { backup }
                })
        };
        RecoveryInput {
            storage: self.storage.clone(),
            active,
        }
    }
    pub fn epochs(&self) -> &Epochs {
        &self.epochs
    }
    pub fn phase(&self) -> &CommitPhase {
        &self.phase
    }
    pub fn active(&self) -> Option<&Arc<Backup>> {
        self.active.as_ref()
    }
    pub fn durable(&self) -> Option<&DurableRecord> {
        match &self.storage {
            StorageObservation::Readable { record } => Some(record),
            _ => None,
        }
    }
    pub fn durable_head(&self) -> ContractResult<Option<transport::DurableHead>> {
        self.storage.head()
    }
    pub fn refresh_storage(
        &mut self,
        epochs: &Epochs,
        observation: StorageObservation,
    ) -> ContractResult<()> {
        if epochs != &self.epochs || self.phase != CommitPhase::Idle {
            return Err(failure(
                IssueCode::StaleAuthority,
                "refresh requires settled idle storage",
            ));
        }
        self.storage
            .head_observation()?
            .validate_refresh(&observation.head_observation()?)?;
        if let StorageObservation::Readable { record } = &observation {
            record.digest()?;
        }
        self.storage = observation.compact()?;
        self.session_selected = true;
        Ok(())
    }
    pub fn prepare(&mut self, backup: Option<Arc<Backup>>, epochs: &Epochs) -> ContractResult<()> {
        if epochs != &self.epochs
            || !matches!(self.phase, CommitPhase::Idle | CommitPhase::Prepared)
        {
            return Err(failure(
                IssueCode::StaleAuthority,
                "settle write or activation before preparation",
            ));
        }
        if let Some(b) = &backup {
            b.digest()?;
        }
        self.candidate = None;
        self.session_candidate = backup;
        self.phase = CommitPhase::Prepared;
        Ok(())
    }
    pub fn invalidate_prepared(&mut self, epochs: Epochs) -> ContractResult<()> {
        if !matches!(self.phase, CommitPhase::Idle | CommitPhase::Prepared)
            || !epochs.succeeds(&self.epochs)
        {
            return Err(failure(
                IssueCode::StaleAuthority,
                "settle write or activation before changing epochs",
            ));
        }
        self.epochs = epochs;
        self.candidate = None;
        self.session_candidate = None;
        self.phase = CommitPhase::Idle;
        Ok(())
    }
    pub fn proposed_record(&mut self) -> ContractResult<Arc<DurableRecord>> {
        if self.phase != CommitPhase::Prepared {
            return Err(failure(IssueCode::StalePrepared, "no prepared record"));
        }
        if let Some(candidate) = &self.candidate {
            return Ok(candidate.clone());
        }
        let transport::HeadObservation::Readable { head } = self.storage.head_observation()? else {
            return Err(failure(
                IssueCode::StorageFailure,
                "save requires a readable durable observation",
            ));
        };
        let candidate = Arc::new(DurableRecord {
            revision: head
                .revision
                .next()
                .ok_or_else(|| failure(IssueCode::ResourceLimit, "storage revision exhausted"))?,
            backup: self.session_candidate.clone(),
        });
        self.candidate = Some(candidate.clone());
        Ok(candidate)
    }
    pub fn reserve_save(
        &mut self,
        epochs: &Epochs,
        expected: &transport::DurableHead,
    ) -> ContractResult<Arc<DurableRecord>> {
        if epochs != &self.epochs
            || self.phase != CommitPhase::Prepared
            || self.storage.head_observation()?
                != (transport::HeadObservation::Readable {
                    head: expected.clone(),
                })
        {
            return Err(failure(
                IssueCode::StaleAuthority,
                "storage reservation no longer matches the prepared read",
            ));
        }
        let candidate = self.proposed_record()?;
        self.phase = CommitPhase::Writing;
        Ok(candidate)
    }
    pub fn storage_failed(&mut self, epochs: &Epochs) -> ContractResult<()> {
        if epochs != &self.epochs || self.phase != CommitPhase::Writing {
            return Err(failure(IssueCode::StaleAuthority, "stale abort callback"));
        }
        self.phase = CommitPhase::Prepared;
        Ok(())
    }
    pub fn storage_committed(&mut self, receipt: &CommitReceipt) -> ContractResult<()> {
        if receipt.epochs != self.epochs || self.phase != CommitPhase::Writing {
            return Err(failure(
                IssueCode::StaleAuthority,
                "stale complete callback",
            ));
        }
        let candidate = self
            .candidate
            .as_ref()
            .ok_or_else(|| failure(IssueCode::StalePrepared, "missing candidate"))?;
        let head = transport::DurableHead::from_record(candidate)?;
        if head.receipt(self.epochs.clone()) != *receipt {
            return Err(failure(
                IssueCode::StorageFailure,
                "receipt differs from candidate",
            ));
        }
        self.storage = StorageObservation::Readable {
            record: candidate.clone(),
        };
        self.phase = CommitPhase::Committed;
        Ok(())
    }
    pub fn activate_committed(&mut self) -> ContractResult<()> {
        if self.phase != CommitPhase::Committed {
            return Err(failure(
                IssueCode::StorageFailure,
                "transaction has not completed",
            ));
        }
        self.active = self.durable().and_then(|r| r.backup.clone());
        self.session_selected = false;
        self.candidate = None;
        self.session_candidate = None;
        self.phase = CommitPhase::Idle;
        Ok(())
    }
    pub fn promote_session_only(&mut self) -> ContractResult<()> {
        if self.phase != CommitPhase::Prepared {
            return Err(failure(
                IssueCode::StorageFailure,
                "session promotion requires an uncommitted candidate",
            ));
        }
        let active = self.session_candidate.clone().ok_or_else(|| {
            failure(
                IssueCode::StorageFailure,
                "memory discard is not durable erasure",
            )
        })?;
        self.storage = self.storage.clone().compact()?;
        self.active = Some(active);
        self.session_selected = true;
        self.candidate = None;
        self.session_candidate = None;
        self.phase = CommitPhase::ActivatingSession;
        Ok(())
    }
    pub fn acknowledge_session_activation(&mut self) -> ContractResult<()> {
        if self.phase != CommitPhase::ActivatingSession {
            return Err(failure(
                IssueCode::StaleAuthority,
                "no pending memory activation",
            ));
        }
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
                "no unsettled transaction",
            ));
        }
        if self.durable_head()?.as_ref() == Some(&transport::DurableHead::from_record(&observed)?) {
            self.phase = CommitPhase::Prepared;
        } else if self.candidate.as_deref() == Some(&observed) {
            self.storage = StorageObservation::Readable {
                record: Arc::new(observed),
            };
            self.phase = CommitPhase::Committed;
        } else {
            return Err(failure(
                IssueCode::StorageFailure,
                "settlement is neither prior record nor candidate",
            ));
        }
        Ok(())
    }
    pub fn discard_session(&mut self, epochs: Epochs) -> ContractResult<()> {
        if self.epochs.reset.next() != Some(epochs.reset) || self.epochs.worker != epochs.worker {
            return Err(failure(
                IssueCode::StaleAuthority,
                "memory discard requires the next reset authority",
            ));
        }
        self.invalidate_prepared(epochs)?;
        self.active = None;
        self.session_selected = true;
        self.storage = self.storage.clone().compact()?;
        Ok(())
    }
    pub fn apply_privacy_clear(
        &mut self,
        clear: &transport::PrivacyClearMachine,
    ) -> ContractResult<()> {
        let epochs = &clear.authority().epochs;
        if !epochs.succeeds(&self.epochs)
            || epochs.reset <= self.epochs.reset
            || matches!(
                self.phase,
                CommitPhase::Writing | CommitPhase::Committed | CommitPhase::ActivatingSession
            )
        {
            return Err(failure(
                IssueCode::StaleAuthority,
                "privacy clear requires an advanced reset and settled prior storage",
            ));
        }
        let input = clear.cleared_bootstrap()?;
        self.epochs = epochs.clone();
        self.storage = input.storage;
        self.active = None;
        self.session_selected = false;
        self.candidate = None;
        self.session_candidate = None;
        self.phase = CommitPhase::Committed;
        Ok(())
    }
    pub fn bootstrap(&mut self, epochs: Epochs, input: RecoveryInput) -> ContractResult<()> {
        if !epochs.succeeds(&self.epochs) || self.phase == CommitPhase::Writing {
            return Err(failure(
                IssueCode::StaleAuthority,
                "settle storage before bootstrap",
            ));
        }
        if let StorageObservation::Readable { record } = &input.storage {
            record.digest()?;
            let old = self.storage.head()?;
            if let Some(old) = old {
                let next = transport::DurableHead::from_record(record)?;
                if next.revision < old.revision || (next.revision == old.revision && next != old) {
                    return Err(failure(
                        IssueCode::StorageFailure,
                        "regressed or inconsistent ordinary recovery",
                    ));
                }
            }
        } else if input.storage != self.storage {
            return Err(failure(
                IssueCode::StorageFailure,
                "unread observation cannot replace known state",
            ));
        }
        if self.phase == CommitPhase::Committed && input != self.recovery_input() {
            return Err(failure(
                IssueCode::StorageFailure,
                "committed storage controls recovery",
            ));
        }
        if let ActiveSelection::RetainedSession { backup } = &input.active
            && (!self.session_selected || self.active.as_ref() != Some(backup))
        {
            return Err(failure(
                IssueCode::StaleAuthority,
                "retained session must be the intended memory anchor",
            ));
        }
        if input.active != self.recovery_input().active
            || (self.phase != CommitPhase::Committed
                && input.selected_backup()? != self.active.as_ref())
        {
            return Err(failure(
                IssueCode::StaleAuthority,
                "ordinary restart cannot change the intended active selection",
            ));
        }
        let active = input.selected_backup()?.cloned();
        if let Some(b) = &active {
            b.digest()?;
        }
        self.epochs = epochs;
        self.storage = if matches!(input.active, ActiveSelection::ObservedDurable) {
            input.storage
        } else {
            input.storage.compact()?
        };
        self.active = active;
        self.session_selected = !matches!(input.active, ActiveSelection::ObservedDurable);
        self.candidate = None;
        self.session_candidate = None;
        self.phase = CommitPhase::Idle;
        Ok(())
    }
}
fn failure(code: IssueCode, message: &str) -> ContractError {
    ContractError::new(code, "/prepared", message)
}

#[derive(Debug, Default)]
pub struct PreparedRegistry {
    pending: Option<(PreparedCommitment, ValidatedAccount)>,
    activated: Option<(PreparedToken, RequestContext, StorageDisposition, Counter)>,
    active: Option<ValidatedAccount>,
    issued: BTreeSet<PreparedToken>,
}
#[derive(Debug)]
struct PreparedCommitment {
    token: PreparedToken,
    context: RequestContext,
    replaces_generation: Counter,
    backup: Arc<Backup>,
    digest: Digest,
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
            return Err(failure(IssueCode::StaleAuthority, "stale preview"));
        }
        if preview.issues.len() > MAX_DIAGNOSTICS
            || self.issued.len() >= 4096
            || self.issued.contains(&preview.token)
        {
            return Err(failure(
                IssueCode::ResourceLimit,
                "candidate budget or reused token",
            ));
        }
        if *preview.backup != Backup::from_validated(&validated)?
            || preview.backup.digest()? != preview.canonical_digest
        {
            return Err(failure(
                IssueCode::InvalidSourceDigest,
                "preview is not the validated backup",
            ));
        }
        self.issued.insert(preview.token.clone());
        self.pending = Some((
            PreparedCommitment {
                token: preview.token.clone(),
                context: context.clone(),
                replaces_generation: current,
                backup: preview.backup.clone(),
                digest: preview.canonical_digest.clone(),
            },
            validated,
        ));
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
            .ok_or_else(|| failure(IssueCode::StalePrepared, "no candidate"))?;
        if &preview.token != token
            || preview.replaces_generation != current
            || &preview.context != context
        {
            return Err(failure(IssueCode::StalePrepared, "stale candidate"));
        }
        if let StorageDisposition::Persisted { receipt } = storage {
            let record = DurableRecord {
                revision: receipt.revision,
                backup: Some(preview.backup.clone()),
            };
            if receipt.revision == Counter(0)
                || receipt.epochs != context.epochs
                || receipt.backup_digest != preview.digest
                || receipt.record_digest != record.digest()?
            {
                return Err(failure(
                    IssueCode::StorageFailure,
                    "receipt differs from prepared state",
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
