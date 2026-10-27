use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const WORKER_MODULE_PATH: &str = "pkg/aaru-wasm/worker.js";
pub const WORKER_BINDINGS_PATH: &str = "pkg/aaru-wasm/aaru_wasm.js";
pub const WEB_MODULE_PATH: &str = "pkg/aaru-web/main.js";
pub const FIXTURE_CATALOG_PATH: &str = "fixtures/catalog.json";
pub const MECHANICS_MANIFEST_PATH: &str = "data/mechanics/manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope<T> {
    pub protocol_version: ProtocolVersion,
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
    File { format: ImportFormat, text: String },
    Sample { id: SampleId },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageDisposition {
    Persisted { account_digest: Digest },
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
        spec: DecisionSpec,
    },
    Compare {
        baseline: DecisionSpec,
        movement: ItemMove,
    },
    Sensitivity {
        baseline: DecisionSpec,
        scenarios: Vec<SensitivityScenario>,
    },
    Export {
        format: ExportFormat,
    },
    Cancel {
        target: RequestId,
    },
    Reset,
}
impl WorkerRequest {
    pub fn validate(&self) -> ContractResult<()> {
        if let Request::PrepareImport {
            input: ImportInput::File { format, text },
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
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportPreview {
    pub token: PreparedToken,
    pub replaces_generation: Counter,
    /// Full validated backup for the UI's atomic storage transaction before activation.
    pub account: AccountSnapshot,
    pub canonical_digest: Digest,
    pub issues: Vec<ImportIssue>,
    pub capability: EvaluationCapability,
    pub sample: Option<SampleId>,
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

/// UI-side stale-event filter. Generation changes are accepted only for the pending
/// activation/reset request, never because an arbitrary event has a larger number.
#[derive(Debug, Default)]
pub struct SessionGate {
    generation: Counter,
    issued: BTreeSet<RequestId>,
    pending: std::collections::BTreeMap<RequestId, Request>,
    current_decision: Option<RequestId>,
    transition: Option<RequestId>,
}
impl SessionGate {
    pub fn generation(&self) -> Counter {
        self.generation
    }
    pub fn register(&mut self, request: &WorkerRequest) -> ContractResult<()> {
        request.validate()?;
        if request.account_generation != self.generation {
            return Err(ContractError::new(
                IssueCode::StaleGeneration,
                "/account_generation",
                "request is not for active generation",
            ));
        }
        if self.issued.len() >= 4096 || self.issued.contains(&request.request_id) {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/request_id",
                "request id reused or session request budget exhausted",
            ));
        }
        if self.transition.is_some()
            && matches!(
                request.payload,
                Request::Evaluate { .. } | Request::Compare { .. } | Request::Sensitivity { .. }
            )
        {
            return Err(ContractError::new(
                IssueCode::StaleGeneration,
                "/request_id",
                "decision cannot start while account transition is pending",
            ));
        }
        if let Request::Cancel { target } = &request.payload
            && self.pending.get(target).is_some_and(|pending| {
                !matches!(
                    pending,
                    Request::Evaluate { .. }
                        | Request::Compare { .. }
                        | Request::Sensitivity { .. }
                )
            })
        {
            return Err(ContractError::new(
                IssueCode::InvalidInput,
                "/target",
                "only decision computations can be cancelled",
            ));
        }
        if matches!(
            request.payload,
            Request::ActivatePrepared { .. } | Request::Reset
        ) {
            if self.transition.is_some() {
                return Err(ContractError::new(
                    IssueCode::StalePrepared,
                    "/request_id",
                    "account transition already pending",
                ));
            }
            self.transition = Some(request.request_id.clone());
            self.current_decision = None;
        }
        if matches!(
            request.payload,
            Request::Evaluate { .. } | Request::Compare { .. } | Request::Sensitivity { .. }
        ) {
            self.current_decision = Some(request.request_id.clone());
        }
        if let Request::Cancel { target } = &request.payload {
            self.pending.remove(target);
            if self.current_decision.as_ref() == Some(target) {
                self.current_decision = None;
            }
        }
        self.issued.insert(request.request_id.clone());
        self.pending
            .insert(request.request_id.clone(), request.payload.clone());
        Ok(())
    }
    /// Returns false for stale, unsolicited, mismatched, or duplicate terminal events.
    pub fn accept(&mut self, event: &WorkerEvent) -> bool {
        let Some(request) = self.pending.get(&event.request_id) else {
            return false;
        };
        let transition = matches!(
            event.payload,
            Event::Activated { .. } | Event::ResetAcknowledged { .. }
        );
        if transition {
            let previous = match &event.payload {
                Event::Activated {
                    previous_generation,
                    ..
                }
                | Event::ResetAcknowledged {
                    previous_generation,
                } => *previous_generation,
                _ => return false,
            };
            let matches_operation = matches!(
                (request, &event.payload),
                (Request::ActivatePrepared { .. }, Event::Activated { .. })
                    | (Request::Reset, Event::ResetAcknowledged { .. })
            );
            if !matches_operation
                || self.transition.as_ref() != Some(&event.request_id)
                || previous != self.generation
                || self.generation.next() != Some(event.account_generation)
            {
                return false;
            }
            self.generation = event.account_generation;
            self.pending.clear();
            self.current_decision = None;
            self.transition = None;
            return true;
        }
        if event.account_generation != self.generation {
            return false;
        }
        let decision = matches!(
            request,
            Request::Evaluate { .. } | Request::Compare { .. } | Request::Sensitivity { .. }
        );
        if decision && self.current_decision.as_ref() != Some(&event.request_id) {
            return false;
        }
        let matches_operation = matches!(
            (request, &event.payload),
            (Request::GetCatalog, Event::Catalog { .. })
                | (Request::PrepareImport { .. }, Event::ImportPrepared { .. })
                | (Request::Evaluate { .. }, Event::Evaluated { .. })
                | (Request::Compare { .. }, Event::Compared { .. })
                | (
                    Request::Sensitivity { .. },
                    Event::SensitivityCompleted { .. }
                )
                | (Request::Export { .. }, Event::Exported { .. })
                | (Request::Cancel { .. }, Event::CancelAcknowledged { .. })
        ) || matches!(event.payload, Event::Error { .. })
            || (decision && matches!(event.payload, Event::Progress { .. }));
        if !matches_operation {
            return false;
        }
        if !matches!(event.payload, Event::Progress { .. }) {
            self.pending.remove(&event.request_id);
            if self.transition.as_ref() == Some(&event.request_id) {
                self.transition = None;
            }
        }
        true
    }
}
