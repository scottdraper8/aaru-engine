use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CHARACTERS: usize = 256;
pub const MAX_WEAPONS: usize = 4096;
pub const MAX_ARTIFACTS: usize = 16384;
pub const MAX_DIAGNOSTICS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SampleId {
    SynWelcome,
    SynScarcity,
    SynStress,
    GoodUnsupportedPreview,
}
impl SampleId {
    pub fn is_synthetic(self) -> bool {
        self != Self::GoodUnsupportedPreview
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Unspecified,
    DeclaredPartial,
    DeclaredComplete,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "coverage", rename_all = "snake_case", deny_unknown_fields)]
pub enum Section<T> {
    Omitted,
    Supplied {
        completeness: Completeness,
        records: Vec<T>,
    },
}
impl<T> Section<T> {
    pub fn records(&self) -> &[T] {
        match self {
            Self::Omitted => &[],
            Self::Supplied { records, .. } => records,
        }
    }
    pub fn is_supplied(&self) -> bool {
        matches!(self, Self::Supplied { .. })
    }
}
pub type SectionCoverage<T> = Section<T>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OwnerRef {
    Unequipped,
    Character(CharacterId),
    UnresolvedExternal(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactSlot {
    Flower,
    Plume,
    Sands,
    Goblet,
    Circlet,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    pub archive_digest: Digest,
    pub pointer: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Talents {
    pub normal: Option<u16>,
    pub skill: Option<u16>,
    pub burst: Option<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedCharacter {
    pub id: CharacterId,
    pub definition: DefinitionId,
    pub level: Option<u16>,
    pub ascension: Option<u16>,
    pub constellation: Option<u16>,
    pub talents: Option<Talents>,
    pub source: Option<SourceRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponInstance {
    pub id: ItemId,
    pub definition: DefinitionId,
    pub level: Option<u16>,
    pub ascension: Option<u16>,
    pub refinement: Option<u16>,
    pub owner: OwnerRef,
    pub game_lock: Option<bool>,
    pub source: Option<SourceRef>,
}
/// An unsupported numeric lexeme remains inert and cannot be evaluated as zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImportedQuantity {
    Evaluable { value: Fixed, unit: QuantityUnit },
    Unsupported { lexeme: String, issue: IssueCode },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityUnit {
    Flat,
    Ratio,
    Seconds,
    Energy,
    Comparative,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactStat {
    pub key: DefinitionId,
    pub quantity: ImportedQuantity,
    pub initial: Option<ImportedQuantity>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactMetadata {
    pub total_rolls: Option<u16>,
    pub astral_mark: Option<bool>,
    pub elixir_crafted: Option<bool>,
    pub unactivated_substats: Option<Vec<ArtifactStat>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactInstance {
    pub id: ItemId,
    pub set: DefinitionId,
    pub slot: ArtifactSlot,
    pub level: Option<u16>,
    pub rarity: Option<u16>,
    pub main_stat: DefinitionId,
    pub substats: Option<Vec<ArtifactStat>>,
    pub owner: OwnerRef,
    pub game_lock: Option<bool>,
    pub metadata: ArtifactMetadata,
    pub source: Option<SourceRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceFormat {
    Good {
        dialect_version: u16,
        profile: ImportProfileVersion,
    },
    Aaru {
        schema_version: AccountVersion,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchive {
    pub format: SourceFormat,
    pub utf8: String,
    pub sha256: Digest,
}
impl SourceArchive {
    pub fn validate(&self) -> ContractResult<()> {
        if self.utf8.len() > MAX_INPUT_BYTES {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/source_archive",
                "source exceeds 16 MiB",
            ));
        }
        if Digest::of(self.utf8.as_bytes()) != self.sha256 {
            return Err(ContractError::new(
                IssueCode::InvalidSourceDigest,
                "/source_archive/sha256",
                "source bytes do not match digest",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameLockPolicy {
    ProtectFromReassignment,
    AllowReassignment,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedAssignment {
    pub item: ItemId,
    pub owner: OwnerRef,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationPreferences {
    pub game_lock_policy: GameLockPolicy,
    pub protected_items: BTreeSet<ItemId>,
    pub pinned: Vec<PinnedAssignment>,
    pub excluded_characters: BTreeSet<CharacterId>,
    pub excluded_items: BTreeSet<ItemId>,
}
impl Default for AllocationPreferences {
    fn default() -> Self {
        Self {
            game_lock_policy: GameLockPolicy::ProtectFromReassignment,
            protected_items: BTreeSet::new(),
            pinned: Vec::new(),
            excluded_characters: BTreeSet::new(),
            excluded_items: BTreeSet::new(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountSnapshot {
    pub schema_version: AccountVersion,
    pub id: SnapshotId,
    pub characters: Section<OwnedCharacter>,
    pub weapons: Section<WeaponInstance>,
    pub artifacts: Section<ArtifactInstance>,
    pub preferences: AllocationPreferences,
    pub source_archive: Option<SourceArchive>,
    pub provenance: Vec<Provenance>,
}
/// Non-deserializable trust marker: external imports cannot assert a bundled sample origin.
#[derive(Debug, Clone)]
pub struct ValidatedAccount {
    snapshot: AccountSnapshot,
    sample: Option<SampleId>,
}
impl ValidatedAccount {
    pub fn imported(snapshot: AccountSnapshot) -> ContractResult<Self> {
        snapshot.validate()?;
        Ok(Self {
            snapshot,
            sample: None,
        })
    }
    /// Only the engine's bundled catalog loader calls this, never an import discriminator.
    pub fn bundled(snapshot: AccountSnapshot, sample: SampleId) -> ContractResult<Self> {
        snapshot.validate()?;
        if sample.is_synthetic()
            && snapshot
                .source_archive
                .as_ref()
                .is_some_and(|archive| matches!(archive.format, SourceFormat::Good { .. }))
        {
            return Err(ContractError::new(
                IssueCode::SyntheticOnImportedAccount,
                "/source_archive",
                "GOOD archives cannot acquire synthetic trust",
            ));
        }
        Ok(Self {
            snapshot,
            sample: Some(sample),
        })
    }
    pub fn snapshot(&self) -> &AccountSnapshot {
        &self.snapshot
    }
    pub fn sample(&self) -> Option<SampleId> {
        self.sample
    }
    pub fn permits(&self, mechanics: &MechanicsRef) -> bool {
        match mechanics {
            MechanicsRef::Synthetic { sample, .. } => {
                self.sample == Some(*sample) && sample.is_synthetic()
            }
            MechanicsRef::Live { .. } => !self.sample.is_some_and(SampleId::is_synthetic),
        }
    }
}

impl AccountSnapshot {
    pub fn validate(&self) -> ContractResult<()> {
        let fail = |code, path: &str, message: &str| Err(ContractError::new(code, path, message));
        if self.characters.records().len() > MAX_CHARACTERS
            || self.weapons.records().len() > MAX_WEAPONS
            || self.artifacts.records().len() > MAX_ARTIFACTS
        {
            return fail(
                IssueCode::ResourceLimit,
                "",
                "inventory record budget exceeded",
            );
        }
        if let Some(archive) = &self.source_archive {
            archive.validate()?;
        }
        let mut identities = BTreeSet::new();
        let mut characters = BTreeSet::new();
        for c in self.characters.records() {
            if !identities.insert(c.id.as_str()) {
                return fail(
                    IssueCode::DuplicateIdentity,
                    "/characters",
                    "duplicate canonical identity",
                );
            }
            characters.insert(c.id.clone());
            self.validate_source(&c.source)?;
            if c.level == Some(0) {
                return fail(
                    IssueCode::InvalidInput,
                    "/characters",
                    "supplied level must be positive",
                );
            }
        }
        let mut items = BTreeMap::new();
        let mut occupied = BTreeSet::new();
        for w in self.weapons.records() {
            if !identities.insert(w.id.as_str()) {
                return fail(
                    IssueCode::DuplicateIdentity,
                    "/weapons",
                    "duplicate canonical identity",
                );
            }
            self.validate_source(&w.source)?;
            validate_owner(&w.owner, &characters)?;
            if w.owner != OwnerRef::Unequipped
                && !occupied.insert((owner_key(&w.owner), "weapon".to_owned()))
            {
                return fail(
                    IssueCode::EquipmentConflict,
                    "/weapons",
                    "multiple weapons on one owner",
                );
            }
            if w.level == Some(0) || w.refinement == Some(0) {
                return fail(
                    IssueCode::InvalidInput,
                    "/weapons",
                    "supplied level/refinement must be positive",
                );
            }
            items.insert(w.id.clone(), &w.owner);
        }
        for a in self.artifacts.records() {
            if !identities.insert(a.id.as_str()) {
                return fail(
                    IssueCode::DuplicateIdentity,
                    "/artifacts",
                    "duplicate canonical identity",
                );
            }
            self.validate_source(&a.source)?;
            validate_owner(&a.owner, &characters)?;
            if a.owner != OwnerRef::Unequipped
                && !occupied.insert((owner_key(&a.owner), format!("{:?}", a.slot)))
            {
                return fail(
                    IssueCode::EquipmentConflict,
                    "/artifacts",
                    "multiple artifacts in one owner slot",
                );
            }
            if a.rarity == Some(0) {
                return fail(
                    IssueCode::InvalidInput,
                    "/artifacts",
                    "supplied rarity must be positive",
                );
            }
            items.insert(a.id.clone(), &a.owner);
        }
        for id in self
            .preferences
            .protected_items
            .iter()
            .chain(&self.preferences.excluded_items)
        {
            if !items.contains_key(id) {
                return fail(IssueCode::InvalidReference, "/preferences", "unknown item");
            }
        }
        for id in &self.preferences.excluded_characters {
            if !characters.contains(id) {
                return fail(
                    IssueCode::InvalidReference,
                    "/preferences",
                    "unknown character",
                );
            }
        }
        let mut pinned = BTreeSet::new();
        for pin in &self.preferences.pinned {
            validate_owner(&pin.owner, &characters)?;
            if !items.contains_key(&pin.item) || !pinned.insert(&pin.item) {
                return fail(
                    IssueCode::InvalidReference,
                    "/preferences/pinned",
                    "unknown or repeated pinned item",
                );
            }
        }
        Ok(())
    }
    fn validate_source(&self, source: &Option<SourceRef>) -> ContractResult<()> {
        if let Some(source) = source {
            let archive = self.source_archive.as_ref().ok_or_else(|| {
                ContractError::new(
                    IssueCode::InvalidReference,
                    "/source",
                    "missing source archive",
                )
            })?;
            if source.archive_digest != archive.sha256
                || (!source.pointer.is_empty() && !source.pointer.starts_with('/'))
            {
                return Err(ContractError::new(
                    IssueCode::InvalidReference,
                    "/source",
                    "invalid source reference",
                ));
            }
        }
        Ok(())
    }
    pub fn item_owner(&self, id: &ItemId) -> Option<(&OwnerRef, Option<bool>)> {
        self.weapons
            .records()
            .iter()
            .find(|w| &w.id == id)
            .map(|w| (&w.owner, w.game_lock))
            .or_else(|| {
                self.artifacts
                    .records()
                    .iter()
                    .find(|a| &a.id == id)
                    .map(|a| (&a.owner, a.game_lock))
            })
    }
    pub fn validate_move(&self, movement: &ItemMove) -> ContractResult<()> {
        let (owner, lock) = self.item_owner(&movement.item).ok_or_else(|| {
            ContractError::new(
                IssueCode::InvalidReference,
                "/item",
                "unknown physical item",
            )
        })?;
        if !self
            .characters
            .records()
            .iter()
            .any(|c| c.id == movement.recipient)
        {
            return Err(ContractError::new(
                IssueCode::InvalidReference,
                "/recipient",
                "unknown recipient",
            ));
        }
        let target = OwnerRef::Character(movement.recipient.clone());
        if owner == &target {
            return Ok(());
        }
        let blocked = if matches!(owner, OwnerRef::UnresolvedExternal(_)) {
            Some(IssueCode::UnresolvedOwner)
        } else if self.preferences.protected_items.contains(&movement.item) {
            Some(IssueCode::ProtectedItem)
        } else if lock == Some(true)
            && self.preferences.game_lock_policy == GameLockPolicy::ProtectFromReassignment
        {
            Some(IssueCode::LockedItem)
        } else if self.preferences.excluded_items.contains(&movement.item)
            || self
                .preferences
                .excluded_characters
                .contains(&movement.recipient)
            || self
                .preferences
                .pinned
                .iter()
                .any(|p| p.item == movement.item && p.owner != target)
        {
            Some(IssueCode::ProtectedItem)
        } else {
            None
        };
        if let Some(code) = blocked {
            return Err(ContractError::new(
                code,
                "/item",
                "movement policy blocks reassignment",
            ));
        }
        Ok(())
    }
}
fn owner_key(owner: &OwnerRef) -> String {
    match owner {
        OwnerRef::Unequipped => "none".into(),
        OwnerRef::Character(id) => format!("character:{}", id.as_str()),
        OwnerRef::UnresolvedExternal(key) => format!("external:{key}"),
    }
}
fn validate_owner(owner: &OwnerRef, characters: &BTreeSet<CharacterId>) -> ContractResult<()> {
    match owner {
        OwnerRef::Character(id) if !characters.contains(id) => Err(ContractError::new(
            IssueCode::InvalidReference,
            "/owner",
            "resolved owner not present",
        )),
        OwnerRef::UnresolvedExternal(key) if key.is_empty() || key.len() > 4096 => {
            Err(ContractError::new(
                IssueCode::InvalidReference,
                "/owner",
                "invalid external owner",
            ))
        }
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Evaluate,
    Compare,
    Sensitivity,
    ExportCanonical,
    ExportSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityBlocker {
    pub operation: Operation,
    pub issue: ImportIssue,
    pub affected_definitions: BTreeSet<DefinitionId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCapability {
    pub supported: BTreeSet<Operation>,
    pub blockers: Vec<CapabilityBlocker>,
    pub scope: EvaluationScope,
}
