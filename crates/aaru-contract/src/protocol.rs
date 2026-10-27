use crate::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};

pub const WORKER_MODULE_PATH: &str = "pkg/aaru-wasm/worker.js";
pub const WORKER_BINDINGS_PATH: &str = "pkg/aaru-wasm/aaru_wasm.js";
pub const WEB_MODULE_PATH: &str = "pkg/aaru-web/main.js";
pub const SUPERVISOR_MODULE_PATH: &str = "pkg/aaru-web/supervisor.js";
pub const WEB_BINDINGS_PATH: &str = "pkg/aaru-web/aaru_web.js";
pub const FIXTURE_CATALOG_PATH: &str = "fixtures/catalog.json";
pub const MECHANICS_MANIFEST_PATH: &str = "data/mechanics/manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope<T> {
    pub protocol_version: ProtocolVersion,
    pub context: RequestContext,
    pub fingerprint: Digest,
    pub request_id: RequestId,
    pub account_generation: Counter,
    pub payload: T,
}
pub type WorkerRequest = Envelope<Request>;
pub type WorkerEvent = Envelope<Event>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    Good,
    Aaru,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImportInput {
    File {
        format: ImportFormat,
        text: Arc<str>,
        declarations: Vec<ImportDeclaration>,
    },
    Sample {
        id: SampleId,
    },
    Restore {
        backup: Arc<Backup>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageDisposition {
    Persisted { receipt: CommitReceipt },
    SessionOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Canonical,
    PreservedSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    GetCatalog,
    PrepareImport {
        input: ImportInput,
    },
    ActivatePrepared {
        token: PreparedToken,
        storage: StorageDisposition,
    },
    Evaluate {
        spec: Arc<DecisionSpec>,
    },
    Compare {
        comparison: Arc<ComparisonSpec>,
    },
    PreparePreferences {
        preferences: AllocationPreferences,
    },
    Bootstrap {
        input: RecoveryInput,
    },
    DiscardSession,
    Sensitivity {
        baseline: Arc<DecisionSpec>,
        scenarios: Arc<Vec<SensitivityScenario>>,
    },
    Export {
        format: ExportFormat,
    },
    Cancel {
        target: RequestId,
    },
    Reset {
        receipt: CommitReceipt,
    },
}
impl WorkerRequest {
    pub fn validate(&self) -> ContractResult<()> {
        if self.fingerprint
            != wire::operation_fingerprint(&self.context, self.account_generation, &self.payload)?
        {
            return Err(ContractError::new(
                IssueCode::StaleAuthority,
                "/fingerprint",
                "request fingerprint differs from full operation context",
            ));
        }
        if let Request::PrepareImport {
            input: ImportInput::File { format, text, .. },
        } = &self.payload
            && text.len()
                > match format {
                    ImportFormat::Good => MAX_INPUT_BYTES,
                    ImportFormat::Aaru => wire::MAX_WIRE_BYTES,
                }
        {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/payload/input/text",
                "import exceeds format-specific byte budget",
            ));
        }
        if let Request::Reset { receipt } = &self.payload {
            let tombstone = DurableRecord {
                revision: receipt.revision,
                backup: None,
            };
            if receipt.revision == Counter(0)
                || receipt.epochs != self.context.epochs
                || receipt.record_digest != tombstone.digest()?
                || receipt.backup_digest != Digest::of(b"aaru-reset-v1")
            {
                return Err(ContractError::new(
                    IssueCode::StorageFailure,
                    "/payload/receipt",
                    "reset requires the matching committed tombstone receipt",
                ));
            }
        }
        if let Request::Sensitivity { scenarios, .. } = &self.payload {
            let ids: BTreeSet<_> = scenarios.iter().map(|s| &s.id).collect();
            if scenarios.is_empty() || scenarios.len() > 16 || ids.len() != scenarios.len() {
                return Err(ContractError::new(
                    IssueCode::ResourceLimit,
                    "/payload/scenarios",
                    "supply 1 to 16 uniquely identified scenarios",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleEntry {
    pub id: SampleId,
    pub label: String,
    pub description: String,
    pub account_path: String,
    pub account_sha256: Digest,
    pub fabricated_inventory: bool,
    pub mechanics: Option<MechanicsRef>,
    pub supported_operations: BTreeSet<Operation>,
    pub expected_diagnostics: Vec<IssueCode>,
    pub oracle_cases: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MechanicsEntry {
    pub mechanics: MechanicsRef,
    pub label: String,
    pub path: String,
    pub normalization: Normalization,
    pub presets: Vec<setup::DecisionPreset>,
    pub covered_definitions: BTreeSet<DefinitionId>,
    pub unsupported_definitions: BTreeSet<DefinitionId>,
    pub provenance: Vec<Provenance>,
    pub capabilities: BTreeSet<Operation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: CatalogVersion,
    pub samples: Vec<SampleEntry>,
    pub mechanics: Vec<MechanicsEntry>,
    pub definitions: Vec<setup::PublicDefinition>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportPreview {
    pub token: PreparedToken,
    pub replaces_generation: Counter,
    pub context: RequestContext,
    pub backup: Arc<Backup>,
    pub canonical_digest: Digest,
    pub issues: Vec<ImportIssue>,
    pub capability: EvaluationCapability,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkPhase {
    Validating,
    Builds,
    Teams,
    Allocation,
    Counterfactual,
    Sensitivity,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub phase: WorkPhase,
    pub completed_units: Counter,
    pub total_units: Option<Counter>,
    pub counters: SearchCounters,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Download {
    pub format: ExportFormat,
    pub filename: String,
    pub media_type: String,
    pub utf8: String,
    pub sha256: Digest,
}
impl Download {
    pub fn validate(&self) -> ContractResult<()> {
        if self.filename.is_empty()
            || self.filename.len() > 128
            || self.filename.contains(['/', '\\'])
            || self.filename.chars().any(char::is_control)
            || self.media_type != "application/json"
        {
            return Err(ContractError::new(
                IssueCode::InvalidInput,
                "/download",
                "download requires a literal filename and JSON MIME type",
            ));
        }
        if self.sha256 != Digest::of(self.utf8.as_bytes()) {
            return Err(ContractError::new(
                IssueCode::InvalidSourceDigest,
                "/download/sha256",
                "download digest differs from payload bytes",
            ));
        }
        if self.utf8.len()
            > match self.format {
                ExportFormat::Canonical => wire::MAX_WIRE_BYTES,
                ExportFormat::PreservedSource => MAX_INPUT_BYTES,
            }
        {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/download/utf8",
                "download exceeds format-specific byte budget",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Catalog {
        catalog: Catalog,
    },
    ImportPrepared {
        preview: Box<ImportPreview>,
    },
    Activated {
        snapshot: SnapshotId,
        account_semantics: Digest,
        previous_generation: Counter,
        capability: EvaluationCapability,
    },
    Progress {
        progress: Progress,
    },
    Evaluated {
        result: Box<DecisionResult>,
    },
    Compared {
        result: Box<ComparisonResult>,
    },
    SensitivityCompleted {
        result: Box<SensitivityResult>,
    },
    Exported {
        download: Download,
    },
    CancelAcknowledged {
        target: RequestId,
        was_running: bool,
    },
    ResetAcknowledged {
        previous_generation: Counter,
    },
    SessionDiscarded {
        previous_generation: Counter,
    },
    Error {
        error: ProtocolError,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolError {
    pub code: IssueCode,
    pub message: String,
    pub issues: Vec<ImportIssue>,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Epochs {
    pub session: SessionId,
    pub worker: Counter,
    pub reset: Counter,
}
impl Epochs {
    pub fn succeeds(&self, previous: &Self) -> bool {
        self.session == previous.session
            && self.worker >= previous.worker
            && self.reset >= previous.reset
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineIdentity {
    pub version: String,
    pub revision: RevisionId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestContext {
    pub epochs: Epochs,
    pub account_semantics: Option<Digest>,
    pub engine: EngineIdentity,
}
#[derive(Debug)]
enum PreparedSave {
    Proposed,
    Writing,
    Committed(CommitReceipt),
    SessionPromoted,
}
#[derive(Debug)]
enum ResetLifecycle {
    Armed,
    Proposed,
    Writing,
    Committed(transport::DurableHead),
    SessionDiscard,
}
#[derive(Debug)]
pub struct SessionGate {
    context: RequestContext,
    generation: Counter,
    issued: BTreeSet<RequestId>,
    pending: Option<WorkerRequest>,
    reset: Option<ResetLifecycle>,
    recovery: Option<RecoveryInput>,
    prepared: Option<ImportPreview>,
    save: Option<PreparedSave>,
}
impl SessionGate {
    pub fn new(context: RequestContext, generation: Counter) -> Self {
        Self {
            context,
            generation,
            issued: BTreeSet::new(),
            pending: None,
            reset: None,
            recovery: None,
            prepared: None,
            save: None,
        }
    }
    pub fn begin_bootstrap(&mut self, input: RecoveryInput) -> ContractResult<()> {
        if !self.issued.is_empty()
            || self.pending.is_some()
            || self.reset.is_some()
            || self.recovery.is_some()
            || self.prepared.is_some()
            || self.save.is_some()
        {
            return Err(authority_error());
        }
        input.selected_backup()?;
        self.recovery = Some(input);
        Ok(())
    }
    pub fn context(&self) -> &RequestContext {
        &self.context
    }
    pub fn generation(&self) -> Counter {
        self.generation
    }
    pub fn restart_worker(&mut self, machine: &CommitMachine) -> ContractResult<()> {
        if machine.epochs() != &self.context.epochs
            || machine.phase() == &CommitPhase::Writing
            || matches!(
                self.reset,
                Some(ResetLifecycle::Armed | ResetLifecycle::Writing)
            )
            || (matches!(self.save, Some(PreparedSave::Writing))
                && machine.phase() != &CommitPhase::Committed)
        {
            return Err(authority_error());
        }
        if let Some(ResetLifecycle::Committed(record)) = &self.reset
            && machine
                .durable()
                .map(transport::DurableHead::from_record)
                .transpose()?
                .as_ref()
                != Some(record)
        {
            return Err(authority_error());
        }
        self.context.epochs.worker = self
            .context
            .epochs
            .worker
            .next()
            .ok_or_else(authority_error)?;
        self.pending = None;
        self.issued.clear();
        self.recovery = Some(machine.recovery_input());
        self.prepared = None;
        self.save = None;
        Ok(())
    }
    pub fn propose_prepared_save(
        &mut self,
        machine: &mut CommitMachine,
        token: &PreparedToken,
    ) -> ContractResult<Arc<DurableRecord>> {
        if self.save.is_some() || !self.preview_authoritative(token, &self.context) {
            return Err(authority_error());
        }
        let backup = self
            .prepared
            .as_ref()
            .ok_or_else(authority_error)?
            .backup
            .clone();
        machine.prepare(Some(backup), &self.context.epochs)?;
        let candidate = machine.proposed_record()?;
        self.save = Some(PreparedSave::Proposed);
        Ok(candidate)
    }
    pub fn reserve_prepared_save(
        &mut self,
        machine: &mut CommitMachine,
        expected: &transport::DurableHead,
    ) -> ContractResult<()> {
        if !matches!(self.save, Some(PreparedSave::Proposed)) {
            return Err(authority_error());
        }
        machine.reserve_save(&self.context.epochs, expected)?;
        self.save = Some(PreparedSave::Writing);
        Ok(())
    }
    pub fn cancel_prepared_save(&mut self, machine: &mut CommitMachine) -> ContractResult<()> {
        if !matches!(self.save, Some(PreparedSave::Proposed)) {
            return Err(authority_error());
        }
        machine.invalidate_prepared(self.context.epochs.clone())?;
        self.save = None;
        self.prepared = None;
        Ok(())
    }
    pub fn promote_prepared_session(
        &mut self,
        machine: &mut CommitMachine,
        token: &PreparedToken,
    ) -> ContractResult<()> {
        if self.save.is_some() || !self.preview_authoritative(token, &self.context) {
            return Err(authority_error());
        }
        machine.prepare(
            self.prepared.as_ref().map(|p| p.backup.clone()),
            &self.context.epochs,
        )?;
        machine.promote_session_only()?;
        self.save = Some(PreparedSave::SessionPromoted);
        Ok(())
    }
    pub fn storage_failed(&mut self, machine: &mut CommitMachine) -> ContractResult<()> {
        if !matches!(self.save, Some(PreparedSave::Writing))
            && !matches!(self.reset, Some(ResetLifecycle::Writing))
        {
            return Err(authority_error());
        }
        machine.storage_failed(&self.context.epochs)?;
        self.release_aborted_save(machine)
    }
    fn release_aborted_save(&mut self, machine: &mut CommitMachine) -> ContractResult<()> {
        if matches!(self.reset, Some(ResetLifecycle::Writing)) {
            machine.invalidate_prepared(self.context.epochs.clone())?;
            self.reset = None;
        }
        self.save = None;
        Ok(())
    }
    pub fn storage_committed(
        &mut self,
        machine: &mut CommitMachine,
        receipt: &CommitReceipt,
    ) -> ContractResult<()> {
        if receipt.epochs != self.context.epochs
            || (!matches!(self.save, Some(PreparedSave::Writing))
                && !matches!(self.reset, Some(ResetLifecycle::Writing)))
        {
            return Err(authority_error());
        }
        machine.storage_committed(receipt)?;
        self.record_committed_storage(machine)
    }
    fn record_committed_storage(&mut self, machine: &CommitMachine) -> ContractResult<()> {
        if matches!(self.reset, Some(ResetLifecycle::Writing)) {
            let record = machine.durable().ok_or_else(authority_error)?;
            if record.backup.is_some() {
                return Err(authority_error());
            }
            self.reset = Some(ResetLifecycle::Committed(
                transport::DurableHead::from_record(record)?,
            ));
        } else if self.save.is_some() {
            let record = machine.durable().ok_or_else(authority_error)?;
            self.save = Some(PreparedSave::Committed(CommitReceipt {
                epochs: self.context.epochs.clone(),
                revision: record.revision,
                record_digest: record.digest()?,
                backup_digest: record
                    .backup
                    .as_ref()
                    .ok_or_else(authority_error)?
                    .digest()?,
            }));
        }
        Ok(())
    }
    pub fn settle_storage(
        &mut self,
        machine: &mut CommitMachine,
        observed: DurableRecord,
    ) -> ContractResult<()> {
        if !matches!(self.save, Some(PreparedSave::Writing))
            && !matches!(self.reset, Some(ResetLifecycle::Writing))
        {
            return Err(authority_error());
        }
        machine.settle_storage(&self.context.epochs, observed)?;
        match machine.phase() {
            CommitPhase::Prepared => self.release_aborted_save(machine),
            CommitPhase::Committed => self.record_committed_storage(machine),
            _ => Err(authority_error()),
        }
    }
    pub fn propose_reset_save(
        &mut self,
        machine: &mut CommitMachine,
    ) -> ContractResult<Arc<DurableRecord>> {
        if !matches!(self.reset, Some(ResetLifecycle::Armed)) || self.recovery.is_some() {
            return Err(authority_error());
        }
        machine.invalidate_prepared(self.context.epochs.clone())?;
        machine.prepare(None, &self.context.epochs)?;
        let candidate = machine.proposed_record()?;
        self.reset = Some(ResetLifecycle::Proposed);
        Ok(candidate)
    }
    pub fn reserve_reset_save(
        &mut self,
        machine: &mut CommitMachine,
        expected: &transport::DurableHead,
    ) -> ContractResult<()> {
        if !matches!(self.reset, Some(ResetLifecycle::Proposed)) {
            return Err(authority_error());
        }
        machine.reserve_save(&self.context.epochs, expected)?;
        self.reset = Some(ResetLifecycle::Writing);
        Ok(())
    }
    pub fn abort_reset_before_save(&mut self, machine: &mut CommitMachine) -> ContractResult<()> {
        if !matches!(
            self.reset,
            Some(ResetLifecycle::Armed | ResetLifecycle::Proposed)
        ) {
            return Err(authority_error());
        }
        machine.invalidate_prepared(self.context.epochs.clone())?;
        self.reset = None;
        Ok(())
    }
    pub fn install_privacy_clear(
        &mut self,
        clear: &transport::PrivacyClearMachine,
    ) -> ContractResult<()> {
        let epochs = &clear.authority().epochs;
        if !epochs.succeeds(&self.context.epochs)
            || epochs.worker != self.context.epochs.worker
            || clear.authority().generation != self.generation
            || !(self.context.epochs.reset.next() == Some(epochs.reset)
                || (epochs.reset == self.context.epochs.reset
                    && matches!(self.reset, Some(ResetLifecycle::Armed))))
            || matches!(
                self.save,
                Some(
                    PreparedSave::Writing
                        | PreparedSave::Committed(_)
                        | PreparedSave::SessionPromoted
                )
            )
            || matches!(
                self.reset,
                Some(ResetLifecycle::Writing | ResetLifecycle::Committed(_))
            )
        {
            return Err(authority_error());
        }
        let input = clear.cleared_bootstrap()?;
        let StorageObservation::Readable { record } = &input.storage else {
            return Err(authority_error());
        };
        self.reset = Some(ResetLifecycle::Committed(
            transport::DurableHead::from_record(record)?,
        ));
        self.context.epochs = epochs.clone();
        self.pending = None;
        self.prepared = None;
        self.save = None;
        self.recovery = Some(input);
        Ok(())
    }
    pub fn discard_session_barrier(&mut self, machine: &mut CommitMachine) -> ContractResult<()> {
        if machine.epochs() != &self.context.epochs
            || matches!(
                machine.phase(),
                CommitPhase::Writing | CommitPhase::Committed | CommitPhase::ActivatingSession
            )
        {
            return Err(authority_error());
        }
        self.reset_barrier()?;
        machine.discard_session(self.context.epochs.clone())?;
        self.reset = Some(ResetLifecycle::SessionDiscard);
        Ok(())
    }
    pub fn reset_barrier(&mut self) -> ContractResult<()> {
        if self.save.is_some() || self.reset.is_some() || self.recovery.is_some() {
            return Err(authority_error());
        }
        if self.pending.as_ref().is_some_and(|p| {
            matches!(
                p.payload,
                Request::ActivatePrepared { .. }
                    | Request::Reset { .. }
                    | Request::Bootstrap { .. }
                    | Request::DiscardSession
            )
        }) {
            return Err(authority_error());
        }
        self.context.epochs.reset = self
            .context
            .epochs
            .reset
            .next()
            .ok_or_else(authority_error)?;
        self.pending = None;
        self.reset = Some(ResetLifecycle::Armed);
        self.prepared = None;
        Ok(())
    }
    pub fn register(&mut self, request: &WorkerRequest) -> ContractResult<()> {
        request.validate()?;
        if matches!(&request.payload, Request::Bootstrap { .. }) && self.recovery.is_none() {
            return Err(authority_error());
        }
        if request.context != self.context || request.account_generation != self.generation {
            return Err(authority_error());
        }
        if let Some(save) = &self.save {
            match (save, &request.payload) {
                (
                    PreparedSave::Committed(expected),
                    Request::ActivatePrepared {
                        storage: StorageDisposition::Persisted { receipt },
                        ..
                    },
                ) if receipt == expected => {}
                (
                    PreparedSave::SessionPromoted,
                    Request::ActivatePrepared {
                        storage: StorageDisposition::SessionOnly,
                        ..
                    },
                ) => {}
                _ => return Err(authority_error()),
            }
        }
        match (&self.reset, &request.payload) {
            (Some(ResetLifecycle::Committed(record)), Request::Reset { receipt })
                if self.recovery.is_none()
                    && record.revision == receipt.revision
                    && record.record_digest == receipt.record_digest => {}
            (Some(ResetLifecycle::Committed(record)), Request::Bootstrap { input })
                if matches!(&input.storage, StorageObservation::Readable { record: observed } if transport::DurableHead::from_record(observed).ok().as_ref() == Some(record))
                    && matches!(input.active, ActiveSelection::ObservedDurable) => {}
            (Some(ResetLifecycle::SessionDiscard), Request::DiscardSession)
                if self.recovery.is_none() => {}
            (
                Some(ResetLifecycle::SessionDiscard),
                Request::Bootstrap {
                    input:
                        RecoveryInput {
                            active: ActiveSelection::EmptySession,
                            ..
                        },
                },
            ) => {}
            (None, Request::Reset { .. } | Request::DiscardSession) => {
                return Err(authority_error());
            }
            (None, _) => {}
            _ => return Err(authority_error()),
        }
        if let Some(expected) = &self.recovery
            && !matches!(&request.payload, Request::Bootstrap { input } if input == expected)
        {
            return Err(authority_error());
        }
        if self.issued.len() >= 4096 || self.issued.contains(&request.request_id) {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/request_id",
                "request budget exhausted or ID reused",
            ));
        }
        if self.pending.as_ref().is_some_and(|p| {
            matches!(
                p.payload,
                Request::ActivatePrepared { .. }
                    | Request::Reset { .. }
                    | Request::Bootstrap { .. }
                    | Request::DiscardSession
            )
        }) {
            return Err(authority_error());
        }
        if let Request::Cancel { target } = &request.payload
            && !self
                .pending
                .as_ref()
                .is_some_and(|p| &p.request_id == target && is_cancellable(&p.payload))
        {
            return Err(authority_error());
        }
        if let Request::ActivatePrepared { token, .. } = &request.payload
            && (self.save.is_none() || !self.preview_authoritative(token, &request.context))
        {
            return Err(authority_error());
        }
        if matches!(
            request.payload,
            Request::PrepareImport { .. } | Request::PreparePreferences { .. }
        ) {
            self.prepared = None;
        }
        self.issued.insert(request.request_id.clone());
        self.pending = Some(request.clone());
        Ok(())
    }
    pub fn accept(&mut self, event: &WorkerEvent) -> bool {
        if matches!(
            event.payload,
            Event::Evaluated { .. } | Event::Compared { .. } | Event::SensitivityCompleted { .. }
        ) {
            return false;
        }
        self.accept_ordered(event)
    }
    pub fn accept_result(
        &mut self,
        event: &WorkerEvent,
        account: &ValidatedAccount,
    ) -> ContractResult<bool> {
        match &event.payload {
            Event::Evaluated { result } => result.validate_for(account)?,
            Event::Compared { result } => result.validate_for(account)?,
            Event::SensitivityCompleted { result } => result.validate_for(account)?,
            _ => {
                return Err(ContractError::new(
                    IssueCode::InvalidInput,
                    "/event",
                    "contextual admission requires a decision, comparison or sensitivity result",
                ));
            }
        }
        Ok(self.accept_ordered(event))
    }
    fn accept_ordered(&mut self, event: &WorkerEvent) -> bool {
        let Some(request) = &self.pending else {
            return false;
        };
        if request.request_id != event.request_id
            || event.context != self.context
            || event.fingerprint != request.fingerprint
        {
            return false;
        }
        let transition = match (&request.payload, &event.payload) {
            (
                Request::ActivatePrepared { .. } | Request::Bootstrap { .. },
                Event::Activated {
                    previous_generation,
                    ..
                },
            )
            | (
                Request::Reset { .. }
                | Request::Bootstrap {
                    input:
                        RecoveryInput {
                            storage: StorageObservation::Readable { .. },
                            active: ActiveSelection::ObservedDurable,
                        },
                },
                Event::ResetAcknowledged {
                    previous_generation,
                },
            )
            | (
                Request::DiscardSession
                | Request::Bootstrap {
                    input:
                        RecoveryInput {
                            active: ActiveSelection::EmptySession,
                            ..
                        },
                },
                Event::SessionDiscarded {
                    previous_generation,
                },
            ) => Some(*previous_generation),
            _ => None,
        };
        if let Some(previous) = transition {
            if previous != self.generation
                || self.generation.next() != Some(event.account_generation)
            {
                return false;
            }
            let expected = match &request.payload {
                Request::ActivatePrepared { .. } => self.prepared.as_ref().map(|p| &p.backup),
                Request::Bootstrap { input } => match input.selected_backup() {
                    Ok(backup) => backup,
                    Err(_) => return false,
                },
                _ => None,
            };
            match (&event.payload, expected) {
                (
                    Event::Activated {
                        snapshot,
                        account_semantics,
                        ..
                    },
                    Some(backup),
                ) if snapshot == &backup.account.id
                    && wire::account_semantic_digest(&backup.account).ok().as_ref()
                        == Some(account_semantics) => {}
                (Event::ResetAcknowledged { .. }, None)
                | (Event::SessionDiscarded { .. }, None) => {}
                _ => return false,
            }
            self.generation = event.account_generation;
            self.prepared = None;
            self.context.account_semantics = match &event.payload {
                Event::Activated {
                    account_semantics, ..
                } => Some(account_semantics.clone()),
                _ => None,
            };
            self.reset = None;
            self.recovery = None;
            self.save = None;
            self.pending = None;
            return true;
        }
        if event.account_generation != self.generation {
            return false;
        }
        let compatible = match (&request.payload, &event.payload) {
            (Request::GetCatalog, Event::Catalog { catalog }) => catalog.validate().is_ok(),
            (Request::Export { .. }, Event::Exported { download }) => download.validate().is_ok(),
            (
                Request::PrepareImport { .. } | Request::PreparePreferences { .. },
                Event::ImportPrepared { preview },
            ) => preview.context == self.context && preview.replaces_generation == self.generation,
            (Request::Cancel { target }, Event::CancelAcknowledged { target: actual, .. }) => {
                target == actual
            }
            (Request::Evaluate { spec }, Event::Evaluated { result }) => {
                &result.spec == spec.as_ref() && self.result_context(result)
            }
            (Request::Compare { comparison }, Event::Compared { result }) => {
                &result.comparison == comparison.as_ref()
                    && result.before.spec == comparison.baseline
                    && result.after.spec == comparison.intervention_spec()
                    && self.result_context(&result.before)
                    && self.result_context(&result.after)
            }
            (
                Request::Sensitivity {
                    baseline,
                    scenarios,
                },
                Event::SensitivityCompleted { result },
            ) => {
                &result.baseline.spec == baseline.as_ref()
                    && result
                        .scenarios
                        .iter()
                        .map(|s| &s.scenario)
                        .eq(scenarios.iter())
                    && self.result_context(&result.baseline)
                    && result.scenarios.iter().all(|s| {
                        let mut expected = baseline.as_ref().clone();
                        expected.assumptions = s.scenario.assumptions.clone();
                        expected.encounters = s.scenario.encounters.clone();
                        s.result.spec == expected && self.result_context(&s.result)
                    })
            }
            (_, Event::Error { .. }) => true,
            (payload, Event::Progress { .. }) => is_decision(payload),
            _ => false,
        };
        if compatible && let Event::ImportPrepared { preview } = &event.payload {
            if preview.backup.digest().ok().as_ref() != Some(&preview.canonical_digest) {
                return false;
            }
            self.prepared = Some(preview.as_ref().clone());
        }
        if compatible && !matches!(event.payload, Event::Progress { .. }) {
            self.pending = None;
        }
        compatible
    }
    pub fn preview_authoritative(&self, token: &PreparedToken, context: &RequestContext) -> bool {
        self.reset.is_none()
            && self.recovery.is_none()
            && self.pending.is_none()
            && context == &self.context
            && self.prepared.as_ref().is_some_and(|p| {
                &p.token == token
                    && &p.context == context
                    && p.replaces_generation == self.generation
            })
    }
    fn result_context(&self, result: &DecisionResult) -> bool {
        result.reproducibility.mechanics == result.spec.mechanics
            && result.scope == result.spec.scope
            && self.context.account_semantics.as_ref()
                == Some(&result.reproducibility.account_semantics)
            && result.reproducibility.engine_version == self.context.engine.version
            && result.reproducibility.engine_revision == self.context.engine.revision
            && wire::decision_spec_digest(&result.spec).ok().as_ref()
                == Some(&result.reproducibility.decision_spec)
            && wire::fingerprint_from_digests(
                &result.reproducibility.account_semantics,
                &result.reproducibility.decision_spec,
                &self.context.engine,
            )
            .ok()
            .as_ref()
                == Some(&result.reproducibility.fingerprint)
    }
}
fn is_cancellable(request: &Request) -> bool {
    matches!(
        request,
        Request::GetCatalog
            | Request::PrepareImport { .. }
            | Request::PreparePreferences { .. }
            | Request::Export { .. }
    ) || is_decision(request)
}
fn is_decision(request: &Request) -> bool {
    matches!(
        request,
        Request::Evaluate { .. } | Request::Compare { .. } | Request::Sensitivity { .. }
    )
}
fn authority_error() -> ContractError {
    ContractError::new(
        IssueCode::StaleAuthority,
        "/context",
        "operation no longer has session authority",
    )
}
