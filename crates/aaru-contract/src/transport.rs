use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_CONTROL_BYTES: usize = 16 * 1024;
pub const MAX_PAGE_ROWS: usize = 24;
pub const MAX_WORKER_LINEAR_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_BLOB_LEDGER_BYTES: u64 = 256 * 1024 * 1024;
pub const PRIMARY_LOCK_NAME: &str = "aaru-primary-v1";
pub const DATABASE_NAME: &str = "aaru-private-v1";
pub const DATABASE_VERSION: u32 = 1;
pub const CURRENT_RECORD_STORE: &str = "current-record";
pub const CURRENT_RECORD_KEY: &str = "current";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportAuthority {
    pub epochs: Epochs,
    pub generation: Counter,
    pub action: RequestId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionBinding {
    pub authority: TransportAuthority,
    pub step: Counter,
    pub operation: Digest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRole {
    UiSupervisor,
    Engine,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    Dom,
    UiSupervisor,
    Engine,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkPurpose {
    LogicalRequest,
    LogicalEvent,
    PreparedBackup,
    DurableRecord,
    RecoveryBackup,
    RecoveryRecord,
    UiCheckpoint,
    CanonicalDownload,
    SourceDownload,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkEncoding {
    StrictUtf8Json,
    PreservedUtf8,
    PrivateUiState,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceOffer {
    pub authority: TransportAuthority,
    pub step: Counter,
    pub attachment: Counter,
    pub format: ImportFormat,
    pub byte_length: Counter,
}
impl SourceOffer {
    pub fn validate(&self) -> ContractResult<()> {
        let maximum = match self.format {
            ImportFormat::Good => MAX_INPUT_BYTES,
            ImportFormat::Aaru => wire::MAX_WIRE_BYTES,
        };
        require(
            self.byte_length.0 > 0 && self.byte_length.0 <= maximum as u64,
            "source offer size",
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulkDescriptor {
    pub authority: TransportAuthority,
    pub step: Counter,
    pub attachment: Counter,
    pub purpose: BulkPurpose,
    pub encoding: BulkEncoding,
    pub byte_length: Counter,
    pub sha256: Digest,
    pub operation: Option<Digest>,
}
impl BulkDescriptor {
    pub fn validate(&self) -> ContractResult<()> {
        let (encoding, maximum) = match self.purpose {
            BulkPurpose::SourceDownload => (BulkEncoding::PreservedUtf8, MAX_INPUT_BYTES),
            BulkPurpose::UiCheckpoint => (BulkEncoding::PrivateUiState, MAX_RESULT_BYTES),
            _ => (BulkEncoding::StrictUtf8Json, wire::MAX_WIRE_BYTES),
        };
        require(
            self.encoding == encoding
                && self.byte_length.0 > 0
                && self.byte_length.0 <= maximum as u64,
            "bulk purpose, encoding or length",
        )?;
        require(
            self.operation.is_some()
                || matches!(
                    self.purpose,
                    BulkPurpose::RecoveryBackup
                        | BulkPurpose::RecoveryRecord
                        | BulkPurpose::UiCheckpoint
                ),
            "verified operation binding required",
        )
    }
    pub fn verify_anchor(
        &self,
        authority: &TransportAuthority,
        step: Counter,
        purpose: BulkPurpose,
        digest: &Digest,
        bytes: &[u8],
    ) -> ContractResult<()> {
        self.validate()?;
        require(
            matches!(
                purpose,
                BulkPurpose::RecoveryBackup
                    | BulkPurpose::RecoveryRecord
                    | BulkPurpose::UiCheckpoint
            ) && &self.authority == authority
                && self.step == step
                && self.purpose == purpose
                && &self.sha256 == digest
                && self.byte_length.0 == bytes.len() as u64
                && self.sha256 == Digest::of(bytes),
            "restoration attachment mismatch",
        )?;
        if self.encoding != BulkEncoding::PrivateUiState {
            std::str::from_utf8(bytes)
                .map_err(|_| invalid("restoration attachment is not exact UTF-8"))?;
        }
        Ok(())
    }
    pub fn verify_bytes(
        &self,
        binding: &ActionBinding,
        purpose: BulkPurpose,
        bytes: &[u8],
    ) -> ContractResult<()> {
        self.validate()?;
        require(
            self.authority == binding.authority
                && self.step == binding.step
                && self.operation.as_ref() == Some(&binding.operation)
                && self.purpose == purpose
                && self.byte_length.0 == bytes.len() as u64
                && self.sha256 == Digest::of(bytes),
            "bulk attachment does not match authority, purpose or bytes",
        )?;
        if self.encoding != BulkEncoding::PrivateUiState {
            std::str::from_utf8(bytes)
                .map_err(|_| invalid("bulk attachment is not exact UTF-8"))?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountHead {
    pub snapshot: SnapshotId,
    pub semantics: Digest,
    pub backup_digest: Digest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DurableContent {
    Account {
        account: AccountHead,
    },
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Tombstone,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableHead {
    pub revision: Counter,
    pub record_digest: Digest,
    pub content: DurableContent,
}
impl DurableHead {
    pub fn from_record(record: &DurableRecord) -> ContractResult<Self> {
        Ok(Self {
            revision: record.revision,
            record_digest: record.digest()?,
            content: match &record.backup {
                Some(backup) => DurableContent::Account {
                    account: AccountHead {
                        snapshot: backup.account.id.clone(),
                        semantics: wire::account_semantic_digest(&backup.account)?,
                        backup_digest: backup.digest()?,
                    },
                },
                None => DurableContent::Tombstone,
            },
        })
    }
    pub fn receipt(&self, epochs: Epochs) -> CommitReceipt {
        CommitReceipt {
            epochs,
            revision: self.revision,
            record_digest: self.record_digest.clone(),
            backup_digest: match &self.content {
                DurableContent::Account { account } => account.backup_digest.clone(),
                DurableContent::Tombstone => Digest::of(b"aaru-reset-v1"),
            },
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedHead {
    pub binding: ActionBinding,
    pub token: PreparedToken,
    pub account: AccountHead,
    pub characters: u16,
    pub weapons: u16,
    pub artifacts: u16,
    pub material_kinds: u16,
    pub issues: Vec<IssueCode>,
}
impl PreparedHead {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            usize::from(self.characters) <= MAX_CHARACTERS
                && usize::from(self.weapons) <= MAX_WEAPONS
                && usize::from(self.artifacts) <= MAX_ARTIFACTS
                && usize::from(self.material_kinds) <= MAX_MATERIAL_KINDS
                && self.issues.len() <= MAX_DIAGNOSTICS,
            "prepared preview bounds",
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageReservation {
    pub binding: ActionBinding,
    pub token: PreparedToken,
    pub expected: DurableHead,
    pub proposed: DurableHead,
    pub record: BulkDescriptor,
}
impl StorageReservation {
    pub fn validate(&self, prepared: &PreparedHead) -> ContractResult<()> {
        prepared.validate()?;
        self.record.validate()?;
        require(
            self.binding == prepared.binding
                && self.token == prepared.token
                && self.expected.revision.next() == Some(self.proposed.revision)
                && self.proposed.content
                    == (DurableContent::Account {
                        account: prepared.account.clone(),
                    })
                && self.record.authority == self.binding.authority
                && self.record.step == self.binding.step
                && self.record.operation.as_ref() == Some(&self.binding.operation)
                && self.record.purpose == BulkPurpose::DurableRecord
                && self.record.sha256 == self.proposed.record_digest,
            "storage proposal differs from prepared commitments",
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiCheckpointHead {
    pub account_semantics: Option<Digest>,
    pub revision: Counter,
    pub schema: RevisionId,
    pub payload: BulkDescriptor,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiCheckpointCommitment {
    pub account_semantics: Option<Digest>,
    pub revision: Counter,
    pub schema: RevisionId,
    pub byte_length: Counter,
    pub sha256: Digest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplacementSettings {
    CarryAcceptedSettings,
    ResetAfterConfirmation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountChangeConfirmation {
    pub binding: ActionBinding,
    pub previous: Option<AccountHead>,
    pub target: AccountHead,
    pub checkpoint: UiCheckpointCommitment,
    pub settings: ReplacementSettings,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CheckpointPolicy {
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    PreserveAccepted,
    ConfirmedReplacement {
        confirmation: Box<AccountChangeConfirmation>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedAccountTransition {
    pub previous_checkpoint: Option<UiCheckpointCommitment>,
    pub checkpoint: UiCheckpointHead,
    pub policy: CheckpointPolicy,
}
impl UiCheckpointHead {
    pub fn commitment(&self) -> UiCheckpointCommitment {
        UiCheckpointCommitment {
            account_semantics: self.account_semantics.clone(),
            revision: self.revision,
            schema: self.schema.clone(),
            byte_length: self.payload.byte_length,
            sha256: self.payload.sha256.clone(),
        }
    }
    pub fn validate(&self) -> ContractResult<()> {
        self.payload.validate()?;
        require(
            self.payload.purpose == BulkPurpose::UiCheckpoint,
            "checkpoint purpose",
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum FormEdit {
    SelectPreset {
        preset: DefinitionId,
    },
    ObjectiveWeight {
        dimension: Dimension,
        value: Fixed,
    },
    Assumption {
        key: DefinitionId,
        value: AssumptionValue,
    },
    EncounterParameter {
        encounter: u16,
        key: DefinitionId,
        value: AssumptionValue,
    },
    EncounterTargets {
        encounter: u16,
        count: u16,
    },
    RequireCharacter {
        character: CharacterId,
        required: bool,
    },
    ExcludeItem {
        item: ItemId,
        excluded: bool,
    },
    MoveRecipient {
        item: ItemId,
        recipient: CharacterId,
    },
}
#[derive(Debug)]
pub struct CheckpointGate {
    accepted: Option<UiCheckpointHead>,
    account: Option<Digest>,
    pending: Option<(ActionBinding, FormEdit)>,
}
impl CheckpointGate {
    pub fn new(
        account: Option<Digest>,
        accepted: Option<UiCheckpointHead>,
    ) -> ContractResult<Self> {
        if let Some(head) = &accepted {
            head.validate()?;
            require(
                head.account_semantics == account,
                "checkpoint account mismatch",
            )?;
        }
        Ok(Self {
            accepted,
            account,
            pending: None,
        })
    }
    pub fn revision(&self) -> Counter {
        self.accepted.as_ref().map_or(Counter(0), |h| h.revision)
    }
    pub fn accepted(&self) -> Option<&UiCheckpointHead> {
        self.accepted.as_ref()
    }
    pub fn offer_edit(
        &mut self,
        binding: ActionBinding,
        base: Counter,
        edit: FormEdit,
    ) -> ContractResult<()> {
        require(
            self.pending.is_none() && base == self.revision(),
            "one unacknowledged UI edit; apply backpressure",
        )?;
        self.pending = Some((binding, edit));
        Ok(())
    }
    pub fn retain_checkpoint(&mut self, head: UiCheckpointHead) -> ContractResult<Counter> {
        head.validate()?;
        let (binding, _) = self
            .pending
            .as_ref()
            .ok_or_else(|| invalid("unsolicited checkpoint"))?;
        require(
            head.account_semantics == self.account
                && self.revision().next() == Some(head.revision)
                && head.payload.authority == binding.authority
                && head.payload.step == binding.step
                && head.payload.operation.as_ref() == Some(&binding.operation),
            "checkpoint authority or revision mismatch",
        )?;
        let revision = head.revision;
        self.accepted = Some(head);
        self.pending = None;
        Ok(revision)
    }
    pub fn abandon_unaccepted_edit(&mut self) {
        self.pending = None;
    }
    pub fn restore_acknowledged(
        &self,
        account: &Option<Digest>,
        revision: Counter,
        digest: Option<&Digest>,
    ) -> ContractResult<()> {
        require(
            account == &self.account
                && revision == self.revision()
                && digest == self.accepted.as_ref().map(|h| &h.payload.sha256),
            "restoration must acknowledge the retained account and UI checkpoint",
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "context", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresentationSubject {
    Decision { subject: Subject },
    Comparison { reference: ComparisonSubject },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresentationRow {
    Label {
        definition: DefinitionId,
        label: String,
    },
    Preset {
        id: DefinitionId,
        revision: RevisionId,
        label: String,
    },
    Parameter {
        description: setup::EditableParameter,
        value: FormValue,
    },
    Allocation {
        allocation: AllocationId,
        score: Fixed,
        vector: ScoreVector,
        pareto: ParetoStatus,
    },
    Reason {
        id: ReasonId,
        code: ReasonCode,
        subjects: Vec<PresentationSubject>,
        causes: Vec<ReasonId>,
    },
    Issue {
        code: IssueCode,
        message: String,
    },
    Assumption {
        key: DefinitionId,
        value: AssumptionValue,
        label: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FormValue {
    Assumption(AssumptionValue),
    Count(u16),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationPage {
    pub binding: ActionBinding,
    pub view_digest: Digest,
    pub page: Counter,
    pub total_pages: Counter,
    pub search: Option<SearchStatus>,
    pub frontier: Option<FrontierPresentation>,
    pub rows: Vec<PresentationRow>,
}
impl PresentationPage {
    pub fn validate(&self) -> ContractResult<()> {
        require(
            self.total_pages.0 > 0
                && self.page < self.total_pages
                && self.rows.len() <= MAX_PAGE_ROWS,
            "presentation page bounds",
        )?;
        for row in &self.rows {
            match row {
                PresentationRow::Allocation { vector, .. } => vector.validate()?,
                PresentationRow::Reason {
                    subjects, causes, ..
                } => require(
                    subjects.len() <= 12 && causes.len() <= 12,
                    "paged causal reference bounds",
                )?,
                _ => {}
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionIntent {
    ImportSample {
        sample: SampleId,
    },
    ImportFile {
        offer: SourceOffer,
    },
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Evaluate,
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Compare,
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Sensitivity,
    Export {
        format: ExportFormat,
    },
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Catalog,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HeadObservation {
    Readable {
        head: DurableHead,
    },
    Corrupt {
        readable_head: DurableHead,
    },
    Unreadable {
        last_known_head: Option<DurableHead>,
    },
    Unavailable {
        last_readable: Option<DurableHead>,
    },
}
impl HeadObservation {
    pub fn head(&self) -> Option<&DurableHead> {
        match self {
            Self::Readable { head } => Some(head),
            Self::Corrupt { readable_head } => Some(readable_head),
            Self::Unreadable { last_known_head } => last_known_head.as_ref(),
            Self::Unavailable { last_readable } => last_readable.as_ref(),
        }
    }
    pub fn validate_refresh(&self, next: &Self) -> ContractResult<()> {
        if let Self::Unavailable {
            last_readable: known,
        }
        | Self::Unreadable {
            last_known_head: known,
        } = next
        {
            return require(
                known.as_ref() == self.head(),
                "unread head observation cannot invent or forget durable authority",
            );
        }
        if let Some(old) = self.head() {
            let head = next
                .head()
                .ok_or_else(|| invalid("refresh cannot forget observed authority"))?;
            require(
                head.revision > old.revision || head == old,
                "regressed or inconsistent storage refresh",
            )?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClearPlan {
    CompareHead {
        expected: DurableHead,
        tombstone: DurableHead,
    },
    Reinitialize {
        new_store: SessionId,
        tombstone: DurableHead,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredHead {
    pub store: SessionId,
    pub head: DurableHead,
}
#[derive(Debug)]
pub struct PrivacyClearMachine {
    authority: TransportAuthority,
    plan: ClearPlan,
    phase: ClearPhase,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClearPhase {
    Writing,
    Aborted,
    Committed,
}
impl PrivacyClearMachine {
    pub fn reserve(
        authority: TransportAuthority,
        observation: HeadObservation,
        new_store: SessionId,
    ) -> ContractResult<Self> {
        let plan = match observation {
            HeadObservation::Unavailable { .. } => {
                return Err(invalid("unavailable storage cannot establish erasure"));
            }
            HeadObservation::Readable { head }
            | HeadObservation::Corrupt {
                readable_head: head,
            } => match head.revision.next() {
                Some(revision) => {
                    let tombstone = DurableHead::from_record(&DurableRecord {
                        revision,
                        backup: None,
                    })?;
                    ClearPlan::CompareHead {
                        expected: head,
                        tombstone,
                    }
                }
                None => ClearPlan::Reinitialize {
                    new_store,
                    tombstone: DurableHead::from_record(&DurableRecord {
                        revision: Counter(1),
                        backup: None,
                    })?,
                },
            },
            HeadObservation::Unreadable { .. } => ClearPlan::Reinitialize {
                new_store,
                tombstone: DurableHead::from_record(&DurableRecord {
                    revision: Counter(1),
                    backup: None,
                })?,
            },
        };
        Ok(Self {
            authority,
            plan,
            phase: ClearPhase::Writing,
        })
    }
    pub fn authority(&self) -> &TransportAuthority {
        &self.authority
    }
    pub fn plan(&self) -> &ClearPlan {
        &self.plan
    }
    pub fn transaction_complete(
        &mut self,
        authority: &TransportAuthority,
        observed: &ClearPlan,
    ) -> ContractResult<()> {
        require(
            self.phase == ClearPhase::Writing
                && authority == &self.authority
                && observed == &self.plan,
            "stale privacy-clear completion",
        )?;
        self.phase = ClearPhase::Committed;
        Ok(())
    }
    pub fn transaction_aborted(&mut self, authority: &TransportAuthority) -> ContractResult<()> {
        require(
            self.phase == ClearPhase::Writing && authority == &self.authority,
            "stale privacy-clear abort",
        )?;
        self.phase = ClearPhase::Aborted;
        Ok(())
    }
    pub fn settle(
        &mut self,
        authority: &TransportAuthority,
        observed: &StoredHead,
    ) -> ContractResult<()> {
        require(
            self.phase == ClearPhase::Writing && authority == &self.authority,
            "stale clear settlement",
        )?;
        self.phase = match &self.plan {
            ClearPlan::CompareHead { tombstone, .. } if &observed.head == tombstone => {
                ClearPhase::Committed
            }
            ClearPlan::CompareHead { expected, .. } if &observed.head == expected => {
                ClearPhase::Aborted
            }
            ClearPlan::Reinitialize {
                new_store,
                tombstone,
            } if new_store == &observed.store && tombstone == &observed.head => {
                ClearPhase::Committed
            }
            _ => {
                return Err(invalid(
                    "ordered reread does not prove clear completion or abort",
                ));
            }
        };
        Ok(())
    }
    pub fn phase(&self) -> &ClearPhase {
        &self.phase
    }
    pub fn erased(&self) -> bool {
        self.phase == ClearPhase::Committed
    }
    pub fn cleared_bootstrap(&self) -> ContractResult<RecoveryInput> {
        require(self.erased(), "erasure has not committed")?;
        let head = match &self.plan {
            ClearPlan::CompareHead { tombstone, .. }
            | ClearPlan::Reinitialize { tombstone, .. } => tombstone,
        };
        Ok(RecoveryInput {
            storage: StorageObservation::Readable {
                record: std::sync::Arc::new(DurableRecord {
                    revision: head.revision,
                    backup: None,
                }),
            },
            active: ActiveSelection::ObservedDurable,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestorationAck {
    pub binding: ActionBinding,
    pub account: Option<AccountHead>,
    pub generation: Counter,
    pub checkpoint: Option<UiCheckpointCommitment>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Control {
    Bootstrap {
        role: WorkerRole,
        port: Counter,
        active: Option<AccountHead>,
        checkpoint: Option<UiCheckpointHead>,
        storage: HeadObservation,
    },
    WorkerReady {
        role: WorkerRole,
        engine: EngineIdentity,
    },
    WorkerFailed {
        role: WorkerRole,
        code: IssueCode,
        message: String,
    },
    Submit {
        intent: ActionIntent,
    },
    Offer {
        offer: SourceOffer,
    },
    Bound {
        binding: ActionBinding,
    },
    Bulk {
        descriptor: BulkDescriptor,
    },
    Prepared {
        head: PreparedHead,
        backup: BulkDescriptor,
    },
    Edit {
        base_revision: Counter,
        edit: FormEdit,
    },
    Checkpoint {
        head: UiCheckpointHead,
    },
    StageAccountTransition {
        transition: StagedAccountTransition,
    },
    CheckpointAcknowledged {
        revision: Counter,
    },
    Confirm {
        token: PreparedToken,
    },
    ProposeSave {
        reservation: StorageReservation,
    },
    ProposeClear {
        plan: ClearPlan,
    },
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    ClearReservationAccepted,
    ClearComplete {
        plan: ClearPlan,
    },
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    ClearAborted,
    ReservationAccepted {
        token: PreparedToken,
    },
    StorageComplete {
        receipt: CommitReceipt,
    },
    StorageAborted {
        token: PreparedToken,
    },
    StorageUncertain {
        token: PreparedToken,
    },
    PromoteSession {
        head: PreparedHead,
        backup: BulkDescriptor,
    },
    SessionAnchorRetained {
        token: PreparedToken,
    },
    Activated {
        acknowledgement: RestorationAck,
    },
    Restored {
        acknowledgement: RestorationAck,
    },
    RequestPage {
        view_digest: Digest,
        page: Counter,
    },
    Page {
        page: PresentationPage,
    },
    PageAcknowledged {
        view_digest: Digest,
        page: Counter,
    },
    Progress {
        phase: WorkPhase,
        completed: Counter,
        total: Option<Counter>,
    },
    Error {
        code: IssueCode,
        message: String,
        recoverable: bool,
    },
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Cancel,
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Retry,
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    ClearLocalData,
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    DiscardSession,
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Shutdown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlFrame {
    pub version: ProtocolVersion,
    pub from: Endpoint,
    pub to: Endpoint,
    pub authority: TransportAuthority,
    pub step: Counter,
    pub payload: Control,
}
impl ControlFrame {
    pub fn validate(&self) -> ContractResult<()> {
        let route = match &self.payload {
            Control::Bootstrap { role, .. } => {
                self.from == Endpoint::Dom
                    && self.to
                        == match role {
                            WorkerRole::UiSupervisor => Endpoint::UiSupervisor,
                            WorkerRole::Engine => Endpoint::Engine,
                        }
            }
            Control::WorkerReady { role, .. } | Control::WorkerFailed { role, .. } => {
                self.to == Endpoint::Dom
                    && self.from
                        == match role {
                            WorkerRole::UiSupervisor => Endpoint::UiSupervisor,
                            WorkerRole::Engine => Endpoint::Engine,
                        }
            }
            Control::Bulk { descriptor } => match descriptor.purpose {
                BulkPurpose::LogicalRequest => {
                    self.from == Endpoint::UiSupervisor && self.to == Endpoint::Engine
                }
                BulkPurpose::LogicalEvent => {
                    self.from == Endpoint::Engine && self.to == Endpoint::UiSupervisor
                }
                BulkPurpose::RecoveryBackup
                | BulkPurpose::RecoveryRecord
                | BulkPurpose::UiCheckpoint => {
                    self.from == Endpoint::Dom && self.to == Endpoint::UiSupervisor
                }
                _ => self.from == Endpoint::UiSupervisor && self.to == Endpoint::Dom,
            },
            Control::Submit { .. }
            | Control::Offer { .. }
            | Control::Edit { .. }
            | Control::CheckpointAcknowledged { .. }
            | Control::Confirm { .. }
            | Control::ReservationAccepted { .. }
            | Control::StorageComplete { .. }
            | Control::StorageAborted { .. }
            | Control::StorageUncertain { .. }
            | Control::SessionAnchorRetained { .. }
            | Control::RequestPage { .. }
            | Control::PageAcknowledged { .. }
            | Control::Cancel
            | Control::Retry
            | Control::ClearLocalData
            | Control::DiscardSession
            | Control::ClearReservationAccepted
            | Control::ClearComplete { .. }
            | Control::ClearAborted => {
                self.from == Endpoint::Dom && self.to == Endpoint::UiSupervisor
            }
            Control::Shutdown => self.from == Endpoint::Dom && self.to != Endpoint::Dom,
            _ => self.from == Endpoint::UiSupervisor && self.to == Endpoint::Dom,
        };
        require(route, "control sent on the wrong endpoint/channel")?;
        let binding = |b: &ActionBinding| {
            require(
                b.authority == self.authority && b.step == self.step,
                "control binding mismatch",
            )
        };
        let bulk = |b: &BulkDescriptor| {
            b.validate()?;
            require(
                b.authority == self.authority && b.step == self.step,
                "control attachment mismatch",
            )
        };
        match &self.payload {
            Control::Bound { binding: b } => binding(b)?,
            Control::Restored { acknowledgement } | Control::Activated { acknowledgement } => {
                binding(&acknowledgement.binding)?
            }
            Control::Offer { offer }
            | Control::Submit {
                intent: ActionIntent::ImportFile { offer },
            } => {
                offer.validate()?;
                require(
                    offer.authority == self.authority && offer.step == self.step,
                    "source offer authority mismatch",
                )?;
            }
            Control::Bulk { descriptor } => bulk(descriptor)?,
            Control::Prepared { head, backup } | Control::PromoteSession { head, backup } => {
                head.validate()?;
                binding(&head.binding)?;
                bulk(backup)?;
                require(
                    backup.purpose == BulkPurpose::PreparedBackup
                        && backup.sha256 == head.account.backup_digest
                        && backup.operation.as_ref() == Some(&head.binding.operation),
                    "prepared backup attachment mismatch",
                )?;
            }
            Control::Checkpoint { head } => {
                head.validate()?;
                bulk(&head.payload)?;
            }
            Control::StageAccountTransition { transition } => {
                transition.checkpoint.validate()?;
                bulk(&transition.checkpoint.payload)?;
            }
            Control::ProposeSave { reservation } => {
                binding(&reservation.binding)?;
                bulk(&reservation.record)?;
            }
            Control::Page { page } => {
                binding(&page.binding)?;
                page.validate()?;
            }
            Control::Bootstrap {
                active,
                checkpoint: Some(head),
                ..
            } => {
                head.validate()?;
                require(
                    head.account_semantics == active.as_ref().map(|a| a.semantics.clone()),
                    "bootstrap checkpoint bound to another active account",
                )?;
            }
            Control::StorageComplete { receipt } => require(
                receipt.epochs == self.authority.epochs && receipt.revision.0 > 0,
                "receipt authority mismatch",
            )?,
            Control::Progress {
                completed,
                total: Some(total),
                ..
            } => require(completed <= total, "progress exceeds total")?,
            _ => {}
        }
        Ok(())
    }
    pub fn attachment(&self) -> Option<AttachmentRequirement> {
        match &self.payload {
            Control::Bootstrap { port, .. } => Some(AttachmentRequirement::Port { id: *port }),
            Control::Offer { offer }
            | Control::Submit {
                intent: ActionIntent::ImportFile { offer },
            } => Some(AttachmentRequirement::File {
                id: offer.attachment,
                bytes: offer.byte_length,
            }),
            Control::Bulk { descriptor } => Some(AttachmentRequirement::Blob {
                id: descriptor.attachment,
                bytes: descriptor.byte_length,
            }),
            Control::Prepared { backup, .. } | Control::PromoteSession { backup, .. } => {
                Some(AttachmentRequirement::Blob {
                    id: backup.attachment,
                    bytes: backup.byte_length,
                })
            }
            Control::ProposeSave { reservation } => Some(AttachmentRequirement::Blob {
                id: reservation.record.attachment,
                bytes: reservation.record.byte_length,
            }),
            Control::StageAccountTransition { transition } => Some(AttachmentRequirement::Blob {
                id: transition.checkpoint.payload.attachment,
                bytes: transition.checkpoint.payload.byte_length,
            }),
            Control::Checkpoint { head } => Some(AttachmentRequirement::Blob {
                id: head.payload.attachment,
                bytes: head.payload.byte_length,
            }),
            _ => None,
        }
    }
    pub fn check_attachment(&self, actual: Option<AttachmentRequirement>) -> ContractResult<()> {
        self.validate()?;
        require(
            self.attachment() == actual,
            "native attachment count, kind, id or length mismatch",
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentRequirement {
    Port { id: Counter },
    File { id: Counter, bytes: Counter },
    Blob { id: Counter, bytes: Counter },
}
pub fn encode_control(frame: &ControlFrame) -> ContractResult<String> {
    frame.validate()?;
    let text = wire::encode(frame)?;
    require(text.len() <= MAX_CONTROL_BYTES, "control exceeds 16 KiB")?;
    Ok(text)
}
pub fn decode_control(text: &str) -> ContractResult<ControlFrame> {
    require(text.len() <= MAX_CONTROL_BYTES, "control exceeds 16 KiB")?;
    let frame: ControlFrame = wire::decode(text)?;
    frame.validate()?;
    Ok(frame)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomPhase {
    Bulk,
    Reserved,
    Activating,
    Recovering,
    Refreshing,
    Idle,
}
#[derive(Debug, PartialEq, Eq)]
pub struct TerminatePair {
    pub workers: [WorkerRole; 2],
    pub close_private_ports: bool,
    pub next_epochs: Epochs,
}
#[derive(Debug)]
pub struct DomGate {
    epochs: Epochs,
    generation: Counter,
    current: Option<TransportAuthority>,
    step: Counter,
    binding: Option<ActionBinding>,
    phase: DomPhase,
    issued: BTreeSet<RequestId>,
    attachments: BTreeSet<Counter>,
    replacement: Option<ActionIntent>,
    prepared: Option<PreparedHead>,
    reservation: Option<StorageReservation>,
    intended: Option<AccountHead>,
    checkpoint: Option<UiCheckpointHead>,
    staged: Option<StagedAccountTransition>,
    storage: HeadObservation,
    clear: Option<PrivacyClearMachine>,
    clear_armed: bool,
    activation_step: Option<Counter>,
    page_pending: Option<(Digest, Counter)>,
}
impl DomGate {
    pub fn new(
        epochs: Epochs,
        generation: Counter,
        storage: HeadObservation,
        intended: Option<AccountHead>,
        checkpoint: Option<UiCheckpointHead>,
    ) -> ContractResult<Self> {
        CheckpointGate::new(
            intended.as_ref().map(|a| a.semantics.clone()),
            checkpoint.clone(),
        )?;
        Ok(Self {
            epochs,
            generation,
            current: None,
            step: Counter(0),
            binding: None,
            phase: DomPhase::Recovering,
            issued: BTreeSet::new(),
            attachments: BTreeSet::new(),
            replacement: None,
            prepared: None,
            reservation: None,
            intended,
            checkpoint,
            staged: None,
            storage,
            clear: None,
            clear_armed: false,
            activation_step: None,
            page_pending: None,
        })
    }
    pub fn generation(&self) -> Counter {
        self.generation
    }
    pub fn checkpoint(&self) -> Option<&UiCheckpointHead> {
        self.checkpoint.as_ref()
    }
    pub fn storage(&self) -> &HeadObservation {
        &self.storage
    }
    pub fn phase(&self) -> &DomPhase {
        &self.phase
    }
    pub fn epochs(&self) -> &Epochs {
        &self.epochs
    }
    pub fn intended(&self) -> Option<&AccountHead> {
        self.intended.as_ref()
    }
    pub fn begin_storage_refresh(
        &mut self,
        action: RequestId,
    ) -> ContractResult<TransportAuthority> {
        let authority = self.start(action)?;
        self.phase = DomPhase::Refreshing;
        Ok(authority)
    }
    pub fn finish_storage_refresh(
        &mut self,
        authority: &TransportAuthority,
        observation: HeadObservation,
    ) -> ContractResult<()> {
        require(
            self.phase == DomPhase::Refreshing && self.admits(authority, Counter(0)),
            "stale or unsettled storage refresh",
        )?;
        self.storage.validate_refresh(&observation)?;
        self.storage = observation;
        self.phase = DomPhase::Idle;
        self.current = None;
        Ok(())
    }
    pub fn discard_session(&mut self, action: RequestId) -> ContractResult<TransportAuthority> {
        let authority = self.begin_clear(action)?;
        self.intended = None;
        self.checkpoint = None;
        self.staged = None;
        self.clear_armed = false;
        self.activation_step = Some(Counter(0));
        self.phase = DomPhase::Activating;
        Ok(authority)
    }
    pub fn retain_checkpoint(&mut self, head: UiCheckpointHead) -> ContractResult<()> {
        head.validate()?;
        let binding = self
            .binding
            .as_ref()
            .ok_or_else(|| invalid("unbound checkpoint"))?;
        require(
            self.phase == DomPhase::Bulk
                && self.staged.is_none()
                && head.account_semantics == self.intended.as_ref().map(|a| a.semantics.clone())
                && self
                    .checkpoint
                    .as_ref()
                    .map_or(Counter(0), |h| h.revision)
                    .next()
                    == Some(head.revision)
                && head.payload.authority == binding.authority
                && head.payload.step == binding.step
                && head.payload.operation.as_ref() == Some(&binding.operation),
            "checkpoint does not extend accepted state",
        )?;
        self.checkpoint = Some(head);
        Ok(())
    }
    pub fn stage_account_transition(
        &mut self,
        transition: StagedAccountTransition,
    ) -> ContractResult<()> {
        let prepared = self
            .prepared
            .as_ref()
            .ok_or_else(|| invalid("no prepared account for checkpoint"))?;
        transition.checkpoint.validate()?;
        let head = &transition.checkpoint;
        let binding = &prepared.binding;
        require(
            self.phase == DomPhase::Bulk
                && self.generation.next().is_some()
                && self.binding.as_ref() == Some(binding)
                && self.staged.is_none()
                && transition.previous_checkpoint
                    == self.checkpoint.as_ref().map(UiCheckpointHead::commitment)
                && self
                    .checkpoint
                    .as_ref()
                    .map_or(Counter(0), |h| h.revision)
                    .next()
                    == Some(head.revision)
                && head.account_semantics.as_ref() == Some(&prepared.account.semantics)
                && head.payload.authority == binding.authority
                && head.payload.step == binding.step
                && head.payload.operation.as_ref() == Some(&binding.operation),
            "target checkpoint does not extend the exact accepted pair",
        )?;
        let same_account = self
            .intended
            .as_ref()
            .is_some_and(|a| a.snapshot == prepared.account.snapshot);
        match &transition.policy {
            CheckpointPolicy::PreserveAccepted => require(
                same_account,
                "replacement requires explicit confirmed settings policy",
            )?,
            CheckpointPolicy::ConfirmedReplacement { confirmation } => require(
                !same_account
                    && confirmation.binding == *binding
                    && confirmation.previous == self.intended
                    && confirmation.target == prepared.account
                    && confirmation.checkpoint == head.commitment(),
                "account change confirmation differs from staged pair",
            )?,
        }
        self.staged = Some(transition);
        Ok(())
    }
    fn promote_checkpoint(&mut self) -> ContractResult<()> {
        self.checkpoint = Some(
            self.staged
                .take()
                .ok_or_else(|| invalid("target checkpoint not retained"))?
                .checkpoint,
        );
        Ok(())
    }
    pub fn start(&mut self, action: RequestId) -> ContractResult<TransportAuthority> {
        require(
            self.phase == DomPhase::Idle
                && self.issued.len() < 4096
                && !self.issued.contains(&action),
            "foreground busy or action reused",
        )?;
        self.issued.insert(action.clone());
        let authority = TransportAuthority {
            epochs: self.epochs.clone(),
            generation: self.generation,
            action,
        };
        self.current = Some(authority.clone());
        self.step = Counter(0);
        self.binding = None;
        self.clear_armed = false;
        self.phase = DomPhase::Bulk;
        Ok(authority)
    }
    pub fn admits(&self, authority: &TransportAuthority, step: Counter) -> bool {
        self.current.as_ref() == Some(authority)
            && self.step == step
            && authority.epochs == self.epochs
    }
    pub fn bind(&mut self, binding: ActionBinding) -> ContractResult<()> {
        require(
            matches!(self.phase, DomPhase::Bulk | DomPhase::Recovering)
                && self.binding.is_none()
                && self.admits(&binding.authority, binding.step),
            "stale or duplicate action binding",
        )?;
        self.binding = Some(binding);
        Ok(())
    }
    pub fn begin_bootstrap(&mut self, action: RequestId) -> ContractResult<TransportAuthority> {
        require(
            self.phase == DomPhase::Recovering
                && self.current.is_none()
                && !self.issued.contains(&action),
            "bootstrap already pending",
        )?;
        let authority = TransportAuthority {
            epochs: self.epochs.clone(),
            generation: self.generation,
            action: action.clone(),
        };
        self.issued.insert(action);
        self.current = Some(authority.clone());
        self.step = Counter(0);
        Ok(authority)
    }
    pub fn finish(&mut self, binding: &ActionBinding) -> ContractResult<()> {
        require(
            self.phase == DomPhase::Bulk
                && self.binding.as_ref() == Some(binding)
                && self.page_pending.is_none(),
            "cannot finish stale or unacknowledged presentation",
        )?;
        self.phase = DomPhase::Idle;
        self.current = None;
        self.binding = None;
        self.prepared = None;
        self.staged = None;
        Ok(())
    }
    pub fn next_step(&mut self, binding: &ActionBinding) -> ContractResult<Counter> {
        require(
            self.phase == DomPhase::Bulk && self.binding.as_ref() == Some(binding),
            "cannot advance bound step",
        )?;
        self.step = self.step.next().ok_or_else(|| invalid("step exhausted"))?;
        self.binding = None;
        self.prepared = None;
        self.staged = None;
        Ok(self.step)
    }
    pub fn admit_offer(&mut self, offer: &SourceOffer) -> ContractResult<()> {
        offer.validate()?;
        require(
            self.phase == DomPhase::Bulk
                && self.binding.is_none()
                && self.admits(&offer.authority, offer.step)
                && self.attachments.len() < 4096
                && !self.attachments.contains(&offer.attachment),
            "stale or repeated unverified offer",
        )?;
        self.attachments.insert(offer.attachment);
        Ok(())
    }
    pub fn admit_attachment(&mut self, descriptor: &BulkDescriptor) -> ContractResult<()> {
        descriptor.validate()?;
        require(
            (self.phase == DomPhase::Bulk
                || (self.phase == DomPhase::Recovering
                    && matches!(
                        descriptor.purpose,
                        BulkPurpose::RecoveryBackup
                            | BulkPurpose::RecoveryRecord
                            | BulkPurpose::UiCheckpoint
                    )))
                && self.admits(&descriptor.authority, descriptor.step)
                && self.binding.as_ref().map(|b| &b.operation) == descriptor.operation.as_ref()
                && self.attachments.len() < 4096
                && !self.attachments.contains(&descriptor.attachment),
            "stale or duplicate attachment",
        )?;
        self.attachments.insert(descriptor.attachment);
        Ok(())
    }
    pub fn prepared(&mut self, head: PreparedHead) -> ContractResult<()> {
        head.validate()?;
        require(
            self.phase == DomPhase::Bulk && self.binding.as_ref() == Some(&head.binding),
            "stale prepared head",
        )?;
        require(self.staged.is_none(), "cannot replace staged account")?;
        self.prepared = Some(head);
        Ok(())
    }
    pub fn begin_clear(&mut self, action: RequestId) -> ContractResult<TransportAuthority> {
        require(
            matches!(self.phase, DomPhase::Idle | DomPhase::Bulk)
                && self.generation.next().is_some()
                && self.issued.len() < 4096
                && !self.issued.contains(&action),
            "clear barrier unavailable",
        )?;
        self.epochs.reset = self
            .epochs
            .reset
            .next()
            .ok_or_else(|| invalid("reset epoch exhausted"))?;
        self.phase = DomPhase::Idle;
        self.prepared = None;
        self.staged = None;
        self.page_pending = None;
        let authority = self.start(action)?;
        self.clear_armed = true;
        Ok(authority)
    }
    pub fn reserve_privacy_clear(
        &mut self,
        binding: &ActionBinding,
        new_store: SessionId,
    ) -> ContractResult<ClearPlan> {
        require(
            self.phase == DomPhase::Bulk
                && self.clear_armed
                && self.binding.as_ref() == Some(binding)
                && self.step.next().is_some(),
            "stale clear reservation",
        )?;
        let clear = PrivacyClearMachine::reserve(
            binding.authority.clone(),
            self.storage.clone(),
            new_store,
        )?;
        let plan = clear.plan().clone();
        self.clear = Some(clear);
        self.phase = DomPhase::Reserved;
        Ok(plan)
    }
    pub fn privacy_clear_aborted(&mut self, binding: &ActionBinding) -> ContractResult<()> {
        require(
            self.phase == DomPhase::Reserved && self.binding.as_ref() == Some(binding),
            "stale clear abort",
        )?;
        self.clear
            .as_mut()
            .ok_or_else(|| invalid("no clear reservation"))?
            .transaction_aborted(&binding.authority)?;
        self.clear = None;
        self.phase = DomPhase::Bulk;
        Ok(())
    }
    pub fn privacy_clear_complete(
        &mut self,
        binding: &ActionBinding,
        plan: &ClearPlan,
    ) -> ContractResult<CommitReceipt> {
        require(
            self.phase == DomPhase::Reserved && self.binding.as_ref() == Some(binding),
            "stale clear complete",
        )?;
        let clear = self
            .clear
            .as_mut()
            .ok_or_else(|| invalid("no clear reservation"))?;
        clear.transaction_complete(&binding.authority, plan)?;
        self.record_clear_commit()
    }
    pub fn settle_privacy_clear(
        &mut self,
        binding: &ActionBinding,
        observed: &StoredHead,
    ) -> ContractResult<Option<CommitReceipt>> {
        require(
            self.phase == DomPhase::Reserved && self.binding.as_ref() == Some(binding),
            "stale clear settlement",
        )?;
        let clear = self
            .clear
            .as_mut()
            .ok_or_else(|| invalid("no clear reservation"))?;
        clear.settle(&binding.authority, observed)?;
        if clear.erased() {
            self.record_clear_commit().map(Some)
        } else {
            self.clear = None;
            self.phase = DomPhase::Bulk;
            Ok(None)
        }
    }
    fn record_clear_commit(&mut self) -> ContractResult<CommitReceipt> {
        let clear = self
            .clear
            .as_ref()
            .ok_or_else(|| invalid("no clear reservation"))?;
        require(clear.erased(), "clear not committed")?;
        let head = match clear.plan() {
            ClearPlan::CompareHead { tombstone, .. }
            | ClearPlan::Reinitialize { tombstone, .. } => tombstone.clone(),
        };
        let receipt = head.receipt(self.epochs.clone());
        self.storage = HeadObservation::Readable { head };
        self.intended = None;
        self.checkpoint = None;
        self.binding = None;
        self.activation_step = self.step.next();
        self.phase = DomPhase::Activating;
        Ok(receipt)
    }
    pub fn reserve(&mut self, reservation: StorageReservation) -> ContractResult<()> {
        require(
            self.phase == DomPhase::Bulk && self.staged.is_some() && self.step.next().is_some(),
            "storage requires retained target checkpoint and activation step",
        )?;
        reservation.validate(
            self.prepared
                .as_ref()
                .ok_or_else(|| invalid("no prepared head"))?,
        )?;
        require(
            self.storage
                == (HeadObservation::Readable {
                    head: reservation.expected.clone(),
                }),
            "CAS requires current readable head",
        )?;
        self.reservation = Some(reservation);
        self.phase = DomPhase::Reserved;
        Ok(())
    }
    pub fn settle_reservation(
        &mut self,
        binding: &ActionBinding,
        observed: &DurableHead,
    ) -> ContractResult<Option<CommitReceipt>> {
        require(self.phase == DomPhase::Reserved, "no unsettled reservation")?;
        let reservation = self
            .reservation
            .as_ref()
            .ok_or_else(|| invalid("no save reservation"))?;
        require(&reservation.binding == binding, "stale settled read")?;
        if observed == &reservation.expected {
            self.transaction_aborted(binding)?;
            Ok(None)
        } else if observed == &reservation.proposed {
            self.transaction_complete(binding).map(Some)
        } else {
            Err(invalid(
                "ordered reread is neither prior head nor candidate",
            ))
        }
    }
    pub fn transaction_aborted(&mut self, binding: &ActionBinding) -> ContractResult<()> {
        require(
            self.phase == DomPhase::Reserved
                && self.reservation.as_ref().map(|r| &r.binding) == Some(binding),
            "stale abort",
        )?;
        self.reservation = None;
        self.phase = DomPhase::Bulk;
        Ok(())
    }
    pub fn transaction_complete(
        &mut self,
        binding: &ActionBinding,
    ) -> ContractResult<CommitReceipt> {
        require(self.phase == DomPhase::Reserved, "no reserved transaction")?;
        let r = self
            .reservation
            .as_ref()
            .ok_or_else(|| invalid("no reservation"))?;
        require(&r.binding == binding, "stale complete")?;
        let receipt = r.proposed.receipt(self.epochs.clone());
        self.storage = HeadObservation::Readable {
            head: r.proposed.clone(),
        };
        self.intended = match &r.proposed.content {
            DurableContent::Account { account } => Some(account.clone()),
            DurableContent::Tombstone => None,
        };
        self.promote_checkpoint()?;
        self.binding = None;
        self.activation_step = self.step.next();
        self.phase = DomPhase::Activating;
        Ok(receipt)
    }
    pub fn promote_session_anchor(&mut self, head: &PreparedHead) -> ContractResult<()> {
        require(
            self.phase == DomPhase::Bulk
                && self.prepared.as_ref() == Some(head)
                && self.binding.as_ref() == Some(&head.binding)
                && self.staged.is_some()
                && self.step.next().is_some(),
            "stale memory promotion",
        )?;
        self.promote_checkpoint()?;
        self.intended = Some(head.account.clone());
        self.binding = None;
        self.activation_step = self.step.next();
        self.phase = DomPhase::Activating;
        Ok(())
    }
    pub fn bind_activation(&mut self, binding: ActionBinding) -> ContractResult<()> {
        require(
            self.phase == DomPhase::Activating
                && self.current.as_ref() == Some(&binding.authority)
                && self.activation_step == Some(binding.step)
                && (self.binding.is_none() || self.binding.as_ref() == Some(&binding)),
            "activation must bind the next logical operation step",
        )?;
        self.step = binding.step;
        self.binding = Some(binding);
        Ok(())
    }
    pub fn activation_acknowledged(
        &mut self,
        acknowledgement: &RestorationAck,
    ) -> ContractResult<()> {
        let binding = &acknowledgement.binding;
        let account = &acknowledgement.account;
        let generation = acknowledgement.generation;
        require(
            self.phase == DomPhase::Activating
                && self.binding.as_ref() == Some(binding)
                && self.activation_step == Some(binding.step)
                && self.intended.as_ref() == account.as_ref()
                && acknowledgement.checkpoint
                    == self.checkpoint.as_ref().map(UiCheckpointHead::commitment)
                && self.generation.next() == Some(generation),
            "activation acknowledgement mismatch",
        )?;
        self.generation = generation;
        self.phase = DomPhase::Idle;
        self.current = None;
        self.binding = None;
        self.prepared = None;
        self.reservation = None;
        self.clear = None;
        self.clear_armed = false;
        self.activation_step = None;
        Ok(())
    }
    pub fn cancel(&mut self) -> ContractResult<TerminatePair> {
        require(
            self.phase == DomPhase::Bulk,
            "cancellation is forbidden across storage/activation/recovery barriers",
        )?;
        self.restart_plan()
    }
    pub fn crash_after_settlement(&mut self) -> ContractResult<TerminatePair> {
        require(
            self.phase != DomPhase::Reserved,
            "settle the actual DOM transaction before restart",
        )?;
        self.restart_plan()
    }
    fn restart_plan(&mut self) -> ContractResult<TerminatePair> {
        self.epochs.worker = self
            .epochs
            .worker
            .next()
            .ok_or_else(|| invalid("incarnation exhausted"))?;
        self.current = None;
        self.binding = None;
        self.prepared = None;
        self.staged = None;
        self.reservation = None;
        self.clear = None;
        self.clear_armed = false;
        self.activation_step = None;
        self.page_pending = None;
        self.attachments.clear();
        self.issued.clear();
        self.phase = DomPhase::Recovering;
        Ok(TerminatePair {
            workers: [WorkerRole::UiSupervisor, WorkerRole::Engine],
            close_private_ports: true,
            next_epochs: self.epochs.clone(),
        })
    }
    pub fn replace_after_cancel(
        &mut self,
        intent: ActionIntent,
    ) -> ContractResult<Option<ActionIntent>> {
        require(
            self.phase == DomPhase::Recovering,
            "replacement only while restoring pair",
        )?;
        Ok(self.replacement.replace(intent))
    }
    pub fn restored(
        &mut self,
        acknowledgement: &RestorationAck,
    ) -> ContractResult<Option<ActionIntent>> {
        let binding = &acknowledgement.binding;
        require(
            self.phase == DomPhase::Recovering
                && self.binding.as_ref() == Some(binding)
                && self.admits(&binding.authority, binding.step)
                && acknowledgement.account.as_ref() == self.intended.as_ref()
                && self.generation.next() == Some(acknowledgement.generation),
            "restart anchor mismatch",
        )?;
        require(
            acknowledgement.checkpoint
                == self.checkpoint.as_ref().map(UiCheckpointHead::commitment),
            "restoration checkpoint mismatch",
        )?;
        self.generation = acknowledgement.generation;
        self.phase = DomPhase::Idle;
        self.current = None;
        self.binding = None;
        Ok(self.replacement.take())
    }
    pub fn present(&mut self, page: &PresentationPage) -> ContractResult<()> {
        page.validate()?;
        require(
            self.phase == DomPhase::Bulk
                && self.binding.as_ref() == Some(&page.binding)
                && self.page_pending.is_none(),
            "stale page or page acknowledgement required",
        )?;
        self.page_pending = Some((page.view_digest.clone(), page.page));
        Ok(())
    }
    pub fn acknowledge_page(&mut self, digest: &Digest, page: Counter) -> ContractResult<()> {
        require(
            self.page_pending.as_ref() == Some(&(digest.clone(), page)),
            "page acknowledgement mismatch",
        )?;
        self.page_pending = None;
        Ok(())
    }
}
#[derive(Debug, Default)]
pub struct BlobLedger {
    entries: std::collections::BTreeMap<Counter, (Counter, Counter)>,
    bytes: u64,
}
impl BlobLedger {
    pub fn retain(&mut self, id: Counter, bytes: Counter) -> ContractResult<()> {
        require(
            bytes.0 > 0 && !self.entries.contains_key(&id),
            "duplicate or empty Blob ledger entry",
        )?;
        let total = self
            .bytes
            .checked_add(bytes.0)
            .ok_or_else(|| invalid("Blob ledger overflow"))?;
        require(
            total <= MAX_BLOB_LEDGER_BYTES && self.entries.len() < 4096,
            "Blob ledger exhausted",
        )?;
        self.entries.insert(id, (bytes, Counter(1)));
        self.bytes = total;
        Ok(())
    }
    pub fn share(&mut self, id: Counter) -> ContractResult<()> {
        let (_, references) = self
            .entries
            .get_mut(&id)
            .ok_or_else(|| invalid("unknown Blob ledger entry"))?;
        *references = references
            .next()
            .ok_or_else(|| invalid("Blob reference counter exhausted"))?;
        Ok(())
    }
    pub fn release(&mut self, id: Counter) -> ContractResult<()> {
        let (bytes, references) = self
            .entries
            .get_mut(&id)
            .ok_or_else(|| invalid("unknown Blob ledger entry"))?;
        references.0 -= 1;
        if references.0 == 0 {
            self.bytes -= bytes.0;
            self.entries.remove(&id);
        }
        Ok(())
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}
fn require(condition: bool, message: &str) -> ContractResult<()> {
    if condition {
        Ok(())
    } else {
        Err(invalid(message))
    }
}
fn invalid(message: &str) -> ContractError {
    ContractError::new(IssueCode::StaleAuthority, "/transport", message)
}
