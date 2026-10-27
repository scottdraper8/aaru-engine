pub mod account;
pub mod comparison;
pub mod decision;
pub mod normalization;
pub mod numeric;
pub mod prepared;
pub mod protocol;
pub mod references;
pub mod sensitivity;
pub mod setup;
pub mod transport;
pub mod wire;
pub mod work;

pub use account::*;
pub use decision::*;
pub use normalization::*;
pub use numeric::{Fixed, NumericError};
pub use prepared::*;
pub use protocol::*;
use serde::{Deserialize, Serialize};

macro_rules! identifier {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);
        impl $name {
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl TryFrom<String> for $name {
            type Error = String;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() || value.len() > 128 || !value.bytes().all(|b|
                    b.is_ascii_alphanumeric() || b"-_.:/".contains(&b)) {
                    return Err(concat!("invalid ", stringify!($name)).into());
                }
                Ok(Self(value))
            }
        }
        impl From<$name> for String { fn from(value: $name) -> Self { value.0 } }
    )+};
}
identifier!(
    SnapshotId,
    CharacterId,
    AvailabilityId,
    SessionId,
    ItemId,
    DefinitionId,
    RequestId,
    PreparedToken,
    CandidateId,
    BuildId,
    AllocationId,
    TeamId,
    RuleId,
    ReasonId,
    ScenarioId,
    RevisionId
);

macro_rules! version {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(try_from = "u16", into = "u16")]
        pub struct $name;
        impl TryFrom<u16> for $name {
            type Error = String;
            fn try_from(value: u16) -> Result<Self, Self::Error> {
                if value == 1 { Ok(Self) } else { Err(format!("unsupported {}: {value}", stringify!($name))) }
            }
        }
        impl From<$name> for u16 { fn from(_: $name) -> Self { 1 } }
    )+};
}
version!(
    AccountVersion,
    ProtocolVersion,
    ImportProfileVersion,
    CatalogVersion
);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Counter(pub u64);
impl TryFrom<String> for Counter {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let n = value.parse::<u64>().map_err(|e| e.to_string())?;
        if n.to_string() != value {
            return Err("noncanonical counter".into());
        }
        Ok(Self(n))
    }
}
impl From<Counter> for String {
    fn from(value: Counter) -> Self {
        value.0.to_string()
    }
}
impl Counter {
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Digest(String);
impl TryFrom<String> for Digest {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("expected lowercase SHA-256".into());
        }
        Ok(Self(value))
    }
}
impl From<Digest> for String {
    fn from(value: Digest) -> Self {
        value.0
    }
}
impl Digest {
    pub fn of(bytes: &[u8]) -> Self {
        use sha2::{Digest as _, Sha256};
        Self(
            Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fidelity {
    ExactUnderModel,
    Estimated,
    Qualitative,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    Synthetic,
    Official,
    CommunityTest,
    Inference,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub id: RevisionId,
    pub origin: EvidenceOrigin,
    pub fidelity: Fidelity,
    pub source: String,
    pub applicable_game_version: Option<String>,
    pub license: Option<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCode {
    InvalidInput,
    UnsupportedVersion,
    ResourceLimit,
    DuplicateProperty,
    DuplicateIdentity,
    InvalidReference,
    EquipmentConflict,
    MissingSection,
    MissingProgression,
    UnknownDefinition,
    UnsupportedMechanics,
    SyntheticOnImportedAccount,
    LockedItem,
    ProtectedItem,
    UnresolvedOwner,
    InvalidObjective,
    UnavailableDimension,
    InvalidEncounter,
    InvalidAssumption,
    NumericOverflow,
    InexactInput,
    StaleGeneration,
    UnknownAvailability,
    StaleAuthority,
    StalePrepared,
    UnknownRequest,
    Cancelled,
    StorageFailure,
    InvalidSourceDigest,
    InvalidAssignment,
    InvalidSearchReport,
    InternalFailure,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Information,
    Warning,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Parameter {
    Text(String),
    Quantity(Fixed),
    Count(Counter),
    Character(CharacterId),
    Item(ItemId),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportIssue {
    pub code: IssueCode,
    pub path: String,
    pub severity: Severity,
    pub parameters: std::collections::BTreeMap<String, Parameter>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractError {
    pub code: IssueCode,
    pub path: String,
    pub message: String,
}
impl ContractError {
    pub fn new(code: IssueCode, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }
}
impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} at {}: {}", self.code, self.path, self.message)
    }
}
impl std::error::Error for ContractError {}
pub type ContractResult<T> = Result<T, ContractError>;
