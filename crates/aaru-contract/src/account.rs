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
    DeclaredComplete { asserted_by: CompletenessAuthority },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompletenessAuthority {
    User { statement: String },
    Exporter { name: String, source: SourceRef },
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventorySection {
    Characters,
    Weapons,
    Artifacts,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportDeclaration {
    pub section: InventorySection,
    pub completeness: Completeness,
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
    pub fn is_declared_complete(&self) -> bool {
        matches!(
            self,
            Self::Supplied {
                completeness: Completeness::DeclaredComplete { .. },
                ..
            }
        )
    }
    fn declare(&mut self, value: Completeness) -> ContractResult<()> {
        let Self::Supplied { completeness, .. } = self else {
            return Err(ContractError::new(
                IssueCode::MissingSection,
                "/declarations",
                "an omitted section cannot be declared empty or complete",
            ));
        };
        *completeness = value;
        Ok(())
    }
}

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
    pub availability: CharacterAvailability,
    pub level: Option<u16>,
    pub ascension: Option<u16>,
    pub constellation: Option<u16>,
    pub talents: Option<Talents>,
    pub source: Option<SourceRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CharacterAvailability {
    Resolved {
        owned: AvailabilityId,
        identity: DefinitionId,
    },
    Unknown,
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
    pub source_archive: Option<std::sync::Arc<SourceArchive>>,
    pub provenance: Vec<Provenance>,
}
#[derive(Debug, Clone)]
pub struct ValidatedAccount {
    snapshot: std::sync::Arc<AccountSnapshot>,
    sample: Option<SampleId>,
}
impl ValidatedAccount {
    pub fn imported(snapshot: impl Into<std::sync::Arc<AccountSnapshot>>) -> ContractResult<Self> {
        let snapshot = snapshot.into();
        snapshot.validate()?;
        Ok(Self {
            snapshot,
            sample: None,
        })
    }
    pub fn bundled(
        snapshot: impl Into<std::sync::Arc<AccountSnapshot>>,
        sample: SampleId,
    ) -> ContractResult<Self> {
        let snapshot = snapshot.into();
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
    pub fn with_preferences(&self, preferences: AllocationPreferences) -> ContractResult<Self> {
        let mut snapshot = self.snapshot.clone();
        std::sync::Arc::make_mut(&mut snapshot).preferences = preferences;
        snapshot.validate()?;
        Ok(Self {
            snapshot,
            sample: self.sample,
        })
    }
    pub fn snapshot(&self) -> &AccountSnapshot {
        &self.snapshot
    }
    pub fn shared_snapshot(&self) -> std::sync::Arc<AccountSnapshot> {
        self.snapshot.clone()
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
    pub fn with_import_declarations(
        mut self,
        declarations: Vec<ImportDeclaration>,
    ) -> ContractResult<Self> {
        let mut sections = BTreeSet::new();
        for declaration in declarations {
            if !sections.insert(declaration.section.clone()) {
                return Err(ContractError::new(
                    IssueCode::InvalidInput,
                    "/declarations",
                    "duplicate section declaration",
                ));
            }
            match declaration.section {
                InventorySection::Characters => {
                    self.characters.declare(declaration.completeness)?
                }
                InventorySection::Weapons => self.weapons.declare(declaration.completeness)?,
                InventorySection::Artifacts => self.artifacts.declare(declaration.completeness)?,
            }
        }
        self.validate()?;
        Ok(self)
    }
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
        for completeness in [
            section_completeness(&self.characters),
            section_completeness(&self.weapons),
            section_completeness(&self.artifacts),
        ]
        .into_iter()
        .flatten()
        {
            if let Completeness::DeclaredComplete { asserted_by } = completeness {
                match asserted_by {
                    CompletenessAuthority::User { statement }
                        if statement.is_empty() || statement.len() > 1024 =>
                    {
                        return fail(
                            IssueCode::InvalidInput,
                            "/declarations",
                            "completeness needs an explicit bounded user statement",
                        );
                    }
                    CompletenessAuthority::Exporter { name, source } => {
                        if name.is_empty() || name.len() > 128 {
                            return fail(
                                IssueCode::InvalidInput,
                                "/declarations",
                                "exporter name required",
                            );
                        }
                        self.validate_source(&Some(source.clone()))?;
                    }
                    _ => {}
                }
            }
        }
        let mut identities = BTreeSet::new();
        let mut characters = BTreeSet::new();
        let mut definitions = BTreeSet::new();
        let mut owned_identities = BTreeMap::new();
        let mut identity_owned = BTreeMap::new();
        for c in self.characters.records() {
            if !identities.insert(c.id.as_str()) {
                return fail(
                    IssueCode::DuplicateIdentity,
                    "/characters",
                    "duplicate canonical identity",
                );
            }
            if !definitions.insert(&c.definition) {
                return fail(
                    IssueCode::DuplicateIdentity,
                    "/characters",
                    "duplicate selectable definition",
                );
            }
            if let CharacterAvailability::Resolved { owned, identity } = &c.availability
                && (owned_identities
                    .insert(owned, identity)
                    .is_some_and(|old| old != identity)
                    || identity_owned
                        .insert(identity, owned)
                        .is_some_and(|old| old != owned))
            {
                return fail(
                    IssueCode::DuplicateIdentity,
                    "/characters",
                    "one real identity must map to one owned availability",
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
                && !occupied.insert((self.owner_resource_key(&w.owner), "weapon".to_owned()))
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
                && !occupied.insert((self.owner_resource_key(&a.owner), format!("{:?}", a.slot)))
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
    fn owner_resource_key(&self, owner: &OwnerRef) -> String {
        if let OwnerRef::Character(id) = owner
            && let Some(OwnedCharacter {
                availability: CharacterAvailability::Resolved { owned, .. },
                ..
            }) = self.characters.records().iter().find(|c| &c.id == id)
        {
            return format!("availability:{}", owned.as_str());
        }
        owner_key(owner)
    }
    pub fn owners_share_resource(&self, left: &OwnerRef, right: &OwnerRef) -> bool {
        self.owner_resource_key(left) == self.owner_resource_key(right)
    }
    pub fn availability(&self, id: &CharacterId) -> ContractResult<&AvailabilityId> {
        match self
            .characters
            .records()
            .iter()
            .find(|c| &c.id == id)
            .map(|c| &c.availability)
        {
            Some(CharacterAvailability::Resolved { owned, .. }) => Ok(owned),
            _ => Err(ContractError::new(
                IssueCode::UnknownAvailability,
                "/characters",
                "unknown ownership/form relationship blocks affected evaluation",
            )),
        }
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
        self.availability(&movement.recipient)?;
        if let OwnerRef::Character(character) = owner {
            self.availability(character)?;
        }
        let target = OwnerRef::Character(movement.recipient.clone());
        if self.owners_share_resource(owner, &target) {
            return Ok(());
        }
        let blocked =
            if matches!(owner, OwnerRef::UnresolvedExternal(_)) {
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
                || self.preferences.pinned.iter().any(|p| {
                    p.item == movement.item && !self.owners_share_resource(&p.owner, &target)
                })
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
fn section_completeness<T>(section: &Section<T>) -> Option<&Completeness> {
    match section {
        Section::Omitted => None,
        Section::Supplied { completeness, .. } => Some(completeness),
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
