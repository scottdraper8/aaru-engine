use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "origin", rename_all = "snake_case", deny_unknown_fields)]
pub enum MechanicsRef {
    Synthetic {
        sample: SampleId,
        revision: RevisionId,
        sha256: Digest,
    },
    Live {
        game_version: String,
        revision: RevisionId,
        sha256: Digest,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvaluationScope {
    WholeSuppliedInventory,
    Subset {
        characters: BTreeSet<CharacterId>,
        items: BTreeSet<ItemId>,
        exclusions: Vec<ImportIssue>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    OutputPotential,
    ReactionCoherence,
    BuffCompatibility,
    EnergyStability,
    RotationFit,
    DefensiveSufficiency,
    EncounterGenerality,
    SpecialSystemCoherence,
    ResourceEfficiency,
}
pub const MAX_WORK_UNITS_PER_SLICE: u32 = 256;
pub const MAX_RESULT_REASONS: usize = 1024;
pub const MAX_RESULT_BYTES: usize = 16 * 1024 * 1024;

pub const DIMENSIONS: [Dimension; 9] = [
    Dimension::OutputPotential,
    Dimension::ReactionCoherence,
    Dimension::BuffCompatibility,
    Dimension::EnergyStability,
    Dimension::RotationFit,
    Dimension::DefensiveSufficiency,
    Dimension::EncounterGenerality,
    Dimension::SpecialSystemCoherence,
    Dimension::ResourceEfficiency,
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Metric {
    Available {
        value: Fixed,
        fidelity: Fidelity,
        provenance: Vec<RevisionId>,
    },
    Unavailable {
        issue: IssueCode,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScoreVector {
    pub dimensions: BTreeMap<Dimension, Metric>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    pub revision: RevisionId,
    pub label: String,
    pub weights: BTreeMap<Dimension, Fixed>,
}
impl Objective {
    pub fn validate(&self) -> ContractResult<()> {
        if self.weights.is_empty()
            || self.weights.values().any(|w| *w < Fixed::ZERO)
            || !self.weights.values().any(|w| *w > Fixed::ZERO)
        {
            return Err(ContractError::new(
                IssueCode::InvalidObjective,
                "/objective/weights",
                "weights must be nonnegative with at least one positive weight",
            ));
        }
        Ok(())
    }
    pub fn score(&self, vector: &ScoreVector) -> ContractResult<Fixed> {
        self.validate()?;
        vector.validate()?;
        let mut total = 0_i128;
        for (dimension, weight) in &self.weights {
            if *weight == Fixed::ZERO {
                continue;
            }
            let Some(Metric::Available { value, .. }) = vector.dimensions.get(dimension) else {
                return Err(ContractError::new(
                    IssueCode::UnavailableDimension,
                    "/score_vector",
                    "objective requires unavailable dimension",
                ));
            };
            total = i128::from(value.scaled())
                .checked_mul(i128::from(weight.scaled()))
                .and_then(|product| total.checked_add(product))
                .ok_or_else(numeric_overflow)?;
        }
        let rounded =
            numeric::round_ratio(total, numeric::SCALE).map_err(|_| numeric_overflow())?;
        Ok(Fixed::from_scaled(
            i64::try_from(rounded).map_err(|_| numeric_overflow())?,
        ))
    }
}
fn numeric_overflow() -> ContractError {
    ContractError::new(
        IssueCode::NumericOverflow,
        "/score",
        "checked fixed-point operation overflowed",
    )
}
impl ScoreVector {
    pub fn validate(&self) -> ContractResult<()> {
        if self.dimensions.len() != DIMENSIONS.len()
            || DIMENSIONS.iter().any(|d| !self.dimensions.contains_key(d))
        {
            return Err(ContractError::new(
                IssueCode::UnavailableDimension,
                "/score_vector",
                "all nine dimensions must be explicit",
            ));
        }
        if self.dimensions.values().any(|m| matches!(m, Metric::Available { value, .. } if *value < Fixed::ZERO || *value > Fixed::ONE)) {
            return Err(ContractError::new(IssueCode::InvalidObjective, "/score_vector", "normalized utility must be within zero and one"));
        }
        Ok(())
    }
    pub fn aggregate(vectors: &[Self]) -> ContractResult<Self> {
        if vectors.is_empty() || vectors.len() > 3 {
            return Err(ContractError::new(
                IssueCode::InvalidEncounter,
                "/score_vector",
                "one to three encounter vectors required",
            ));
        }
        for vector in vectors {
            vector.validate()?;
        }
        let mut dimensions = BTreeMap::new();
        for dimension in DIMENSIONS {
            let mut sum = 0_i128;
            let mut fidelity = Fidelity::ExactUnderModel;
            let mut provenance = BTreeSet::new();
            let mut unavailable = None;
            for vector in vectors {
                match &vector.dimensions[&dimension] {
                    Metric::Available {
                        value,
                        fidelity: f,
                        provenance: p,
                    } => {
                        sum += i128::from(value.scaled());
                        fidelity = std::cmp::max(fidelity, f.clone());
                        provenance.extend(p.iter().cloned());
                    }
                    Metric::Unavailable { issue } => {
                        unavailable.get_or_insert(issue.clone());
                    }
                }
            }
            dimensions.insert(
                dimension,
                match unavailable {
                    Some(issue) => Metric::Unavailable { issue },
                    None => Metric::Available {
                        value: Fixed::from_scaled(
                            i64::try_from(
                                numeric::round_ratio(sum, vectors.len() as i128)
                                    .map_err(|_| numeric_overflow())?,
                            )
                            .map_err(|_| numeric_overflow())?,
                        ),
                        fidelity,
                        provenance: provenance.into_iter().collect(),
                    },
                },
            );
        }
        Ok(Self { dimensions })
    }
    pub fn pareto_against(&self, other: &Self) -> ContractResult<ParetoRelation> {
        self.validate()?;
        other.validate()?;
        let mut better = false;
        let mut worse = false;
        for d in DIMENSIONS {
            match (&self.dimensions[&d], &other.dimensions[&d]) {
                (Metric::Available { value: a, .. }, Metric::Available { value: b, .. }) => {
                    better |= a > b;
                    worse |= a < b;
                }
                _ => return Ok(ParetoRelation::Incomparable),
            }
        }
        Ok(match (better, worse) {
            (true, false) => ParetoRelation::Dominates,
            (false, true) => ParetoRelation::Dominated,
            (false, false) => ParetoRelation::Equal,
            (true, true) => ParetoRelation::Incomparable,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParetoRelation {
    Dominates,
    Dominated,
    Equal,
    Incomparable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParetoStatus {
    Frontier,
    Dominated { by: Vec<AllocationId> },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AssumptionValue {
    Flag(bool),
    Quantity(Fixed),
    Choice(DefinitionId),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assumptions {
    pub revision: RevisionId,
    pub values: BTreeMap<DefinitionId, AssumptionValue>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub revision: RevisionId,
    pub definition: DefinitionId,
    pub label: String,
    pub target_count: u16,
    pub parameters: BTreeMap<DefinitionId, AssumptionValue>,
    pub provenance: Vec<Provenance>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildConstraints {
    pub required_characters: BTreeSet<CharacterId>,
    pub excluded_characters: BTreeSet<CharacterId>,
    pub excluded_items: BTreeSet<ItemId>,
    pub pinned_assignments: Vec<PinnedAssignment>,
    pub allowed_sets: BTreeMap<CharacterId, BTreeSet<DefinitionId>>,
    pub allowed_main_stats: BTreeMap<CharacterId, BTreeMap<ArtifactSlot, BTreeSet<DefinitionId>>>,
    pub metric_minima: BTreeMap<DefinitionId, Fixed>,
    pub required_rules: BTreeSet<RuleId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchPolicy {
    pub revision: RevisionId,
    pub max_build_nodes: Counter,
    pub max_team_nodes: Counter,
    pub max_allocation_nodes: Counter,
    pub max_results: u16,
    pub work_units_per_slice: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionSpec {
    pub assignments: AssignmentDomain,
    pub mechanics: MechanicsRef,
    pub normalization: Normalization,
    pub scope: EvaluationScope,
    pub objective: Objective,
    pub encounters: Vec<Encounter>,
    pub assumptions: Assumptions,
    pub constraints: BuildConstraints,
    pub search: SearchPolicy,
}
impl DecisionSpec {
    pub fn validate_for(&self, account: &ValidatedAccount) -> ContractResult<()> {
        if !account.permits(&self.mechanics) {
            return Err(ContractError::new(
                IssueCode::SyntheticOnImportedAccount,
                "/mechanics",
                "mechanics origin does not match trusted account origin",
            ));
        }
        self.objective.validate()?;
        self.normalization.validate()?;
        if self.encounters.is_empty()
            || self.encounters.len() > 3
            || self.encounters.iter().any(|e| e.target_count == 0)
        {
            return Err(ContractError::new(
                IssueCode::InvalidEncounter,
                "/encounters",
                "supply one to three nonempty encounters",
            ));
        }
        if self.search.max_results == 0
            || self.search.max_results > 128
            || self.search.work_units_per_slice == 0
            || self.search.work_units_per_slice > MAX_WORK_UNITS_PER_SLICE
            || self.search.max_build_nodes.0 == 0
            || self.search.max_team_nodes.0 == 0
            || self.search.max_allocation_nodes.0 == 0
        {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/search",
                "search limits must be positive; at most 128 results",
            ));
        }
        let snapshot = account.snapshot();
        for character in snapshot.characters.records() {
            let affected = match &self.scope {
                EvaluationScope::WholeSuppliedInventory => true,
                EvaluationScope::Subset { characters, .. } => characters.contains(&character.id),
            };
            if affected {
                snapshot.availability(&character.id)?;
            }
        }
        for item in snapshot
            .weapons
            .records()
            .iter()
            .map(|w| (&w.id, &w.owner))
            .chain(
                snapshot
                    .artifacts
                    .records()
                    .iter()
                    .map(|a| (&a.id, &a.owner)),
            )
        {
            let affected = match &self.scope {
                EvaluationScope::WholeSuppliedInventory => true,
                EvaluationScope::Subset { items, .. } => items.contains(item.0),
            };
            if affected && let OwnerRef::Character(owner) = item.1 {
                snapshot.availability(owner)?;
            }
        }
        let chars: BTreeSet<_> = snapshot
            .characters
            .records()
            .iter()
            .map(|c| &c.id)
            .collect();
        for id in self
            .constraints
            .required_characters
            .iter()
            .chain(&self.constraints.excluded_characters)
            .chain(self.constraints.allowed_sets.keys())
            .chain(self.constraints.allowed_main_stats.keys())
        {
            if !chars.contains(id) {
                return Err(ContractError::new(
                    IssueCode::InvalidReference,
                    "/constraints",
                    "unknown character",
                ));
            }
        }
        if !self
            .constraints
            .required_characters
            .is_disjoint(&self.constraints.excluded_characters)
        {
            return Err(ContractError::new(
                IssueCode::InvalidInput,
                "/constraints",
                "required character excluded",
            ));
        }
        for id in &self.constraints.excluded_items {
            if snapshot.item_owner(id).is_none() {
                return Err(ContractError::new(
                    IssueCode::InvalidReference,
                    "/constraints",
                    "unknown item",
                ));
            }
        }
        let mut pinned = BTreeSet::new();
        for pin in &self.constraints.pinned_assignments {
            if snapshot.item_owner(&pin.item).is_none()
                || !pinned.insert(&pin.item)
                || matches!(&pin.owner, OwnerRef::Character(c) if !chars.contains(c))
            {
                return Err(ContractError::new(
                    IssueCode::InvalidReference,
                    "/constraints/pinned_assignments",
                    "unknown or repeated pin resource",
                ));
            }
        }
        if let EvaluationScope::Subset {
            characters, items, ..
        } = &self.scope
            && (characters.iter().any(|c| !chars.contains(c))
                || items.iter().any(|i| snapshot.item_owner(i).is_none()))
        {
            return Err(ContractError::new(
                IssueCode::InvalidReference,
                "/scope",
                "unknown subset resource",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemMove {
    pub item: ItemId,
    pub recipient: CharacterId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssignmentDomain {
    Unrestricted,
    CurrentLoadout,
    ReservedMove {
        movement: ItemMove,
    },
    MoveOnly {
        allocation: Box<Allocation>,
        movement: ItemMove,
        donor_fallback: Option<ItemId>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComparisonMode {
    CurrentLoadout {
        allocation: Box<Allocation>,
        donor_fallback: Option<ItemId>,
    },
    UnrestrictedOptimum,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonSpec {
    pub baseline: DecisionSpec,
    pub mode: ComparisonMode,
    pub movement: ItemMove,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceFootprint {
    pub characters: BTreeSet<CharacterId>,
    pub availability: BTreeSet<AvailabilityId>,
    pub items: BTreeSet<ItemId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildAssignment {
    pub id: BuildId,
    pub character: CharacterId,
    pub weapon: Option<ItemId>,
    pub artifacts: BTreeMap<ArtifactSlot, ItemId>,
    pub metrics: BTreeMap<DefinitionId, Metric>,
    pub reasons: Vec<ReasonId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamAssignment {
    pub id: TeamId,
    pub encounter_index: u16,
    pub builds: Vec<BuildAssignment>,
    pub archetypes: BTreeSet<DefinitionId>,
    pub score_vector: ScoreVector,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allocation {
    pub id: AllocationId,
    pub candidate: CandidateId,
    pub teams: Vec<TeamAssignment>,
    pub footprint: ResourceFootprint,
    pub score_vector: ScoreVector,
    pub presentation_score: Fixed,
    pub pareto: ParetoStatus,
    pub constraints: Vec<ConstraintOutcome>,
    pub reasons: Vec<ReasonId>,
}
impl Allocation {
    pub fn validate(&self, account: &AccountSnapshot, spec: &DecisionSpec) -> ContractResult<()> {
        let invalid =
            |message| ContractError::new(IssueCode::InvalidAssignment, "/allocations", message);
        if self.teams.len() != spec.encounters.len() {
            return Err(invalid("team/encounter count mismatch"));
        }
        account.validate()?;
        let mut characters = BTreeSet::new();
        let mut availability = BTreeSet::new();
        let mut items = BTreeSet::new();
        let mut encounters = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for team in &self.teams {
            if !ids.insert(team.id.as_str())
                || usize::from(team.encounter_index) >= spec.encounters.len()
                || !encounters.insert(team.encounter_index)
                || team.builds.len() != 4
            {
                return Err(invalid(
                    "duplicate team/encounter or party is not four characters",
                ));
            }
            for build in &team.builds {
                if !ids.insert(build.id.as_str())
                    || !characters.insert(build.character.clone())
                    || !account
                        .characters
                        .records()
                        .iter()
                        .any(|c| c.id == build.character)
                {
                    return Err(invalid("unknown/repeated character or build"));
                }
                for (key, minimum) in &spec.constraints.metric_minima {
                    if !matches!(build.metrics.get(key), Some(Metric::Available { value, .. }) if value >= minimum)
                    {
                        return Err(invalid("required build metric minimum is absent or unmet"));
                    }
                }
                if !availability.insert(account.availability(&build.character)?.clone()) {
                    return Err(invalid("mutually exclusive forms share owned availability"));
                }
                if let Some(weapon) = &build.weapon {
                    if !account.weapons.records().iter().any(|w| &w.id == weapon)
                        || !items.insert(weapon.clone())
                    {
                        return Err(invalid("unknown/repeated weapon copy"));
                    }
                    account.validate_move(&ItemMove {
                        item: weapon.clone(),
                        recipient: build.character.clone(),
                    })?;
                }
                for (slot, item) in &build.artifacts {
                    if account
                        .artifacts
                        .records()
                        .iter()
                        .find(|a| &a.id == item)
                        .is_some_and(|a| {
                            spec.constraints
                                .allowed_sets
                                .get(&build.character)
                                .is_some_and(|sets| !sets.contains(&a.set))
                                || spec
                                    .constraints
                                    .allowed_main_stats
                                    .get(&build.character)
                                    .and_then(|slots| slots.get(slot))
                                    .is_some_and(|stats| !stats.contains(&a.main_stat))
                        })
                    {
                        return Err(invalid("artifact violates allowed set or main stat"));
                    }
                    if !account
                        .artifacts
                        .records()
                        .iter()
                        .any(|a| &a.id == item && &a.slot == slot)
                        || !items.insert(item.clone())
                    {
                        return Err(invalid("unknown/repeated artifact or wrong slot"));
                    }
                    account.validate_move(&ItemMove {
                        item: item.clone(),
                        recipient: build.character.clone(),
                    })?;
                }
            }
        }
        if !spec.constraints.required_characters.is_subset(&characters)
            || !spec
                .constraints
                .excluded_characters
                .is_disjoint(&characters)
            || !account
                .preferences
                .excluded_characters
                .is_disjoint(&characters)
            || !spec.constraints.excluded_items.is_disjoint(&items)
            || !account.preferences.excluded_items.is_disjoint(&items)
        {
            return Err(invalid("allocation violates required/excluded resources"));
        }
        if let EvaluationScope::Subset {
            characters: allowed_characters,
            items: allowed_items,
            ..
        } = &spec.scope
            && (!characters.is_subset(allowed_characters) || !items.is_subset(allowed_items))
        {
            return Err(invalid("allocation uses resources outside declared subset"));
        }
        for pin in spec
            .constraints
            .pinned_assignments
            .iter()
            .chain(&account.preferences.pinned)
        {
            if items.contains(&pin.item)
                && !self.teams.iter().flat_map(|t| &t.builds).any(|b| {
                    account.owners_share_resource(
                        &pin.owner,
                        &OwnerRef::Character(b.character.clone()),
                    ) && (b.weapon.as_ref() == Some(&pin.item)
                        || b.artifacts.values().any(|i| i == &pin.item))
                })
            {
                return Err(invalid("allocation violates pinned assignment"));
            }
        }
        if self.footprint.characters != characters
            || self.footprint.items != items
            || self.footprint.availability != availability
        {
            return Err(invalid("footprint differs from exact assignments"));
        }
        match &spec.assignments {
            AssignmentDomain::Unrestricted => {}
            AssignmentDomain::CurrentLoadout => {
                for build in self.teams.iter().flat_map(|t| &t.builds) {
                    let owner = OwnerRef::Character(build.character.clone());
                    let weapon = account
                        .weapons
                        .records()
                        .iter()
                        .find(|w| account.owners_share_resource(&w.owner, &owner))
                        .map(|w| w.id.clone());
                    let artifacts = account
                        .artifacts
                        .records()
                        .iter()
                        .filter(|a| account.owners_share_resource(&a.owner, &owner))
                        .map(|a| (a.slot, a.id.clone()))
                        .collect::<BTreeMap<_, _>>();
                    if build.weapon != weapon || build.artifacts != artifacts {
                        return Err(invalid(
                            "current-loadout evaluation cannot reassign equipment",
                        ));
                    }
                }
            }
            AssignmentDomain::MoveOnly {
                allocation,
                movement,
                donor_fallback,
            } => {
                let mut baseline = spec.clone();
                baseline.assignments = AssignmentDomain::CurrentLoadout;
                allocation.validate(account, &baseline)?;
                crate::comparison::validate_move_only(
                    allocation,
                    self,
                    account,
                    movement,
                    donor_fallback.as_ref(),
                )?;
            }
            AssignmentDomain::ReservedMove { movement } => {
                account.validate_move(movement)?;
                if !self.teams.iter().flat_map(|t| &t.builds).any(|b| {
                    b.character == movement.recipient
                        && (b.weapon.as_ref() == Some(&movement.item)
                            || b.artifacts.values().any(|i| i == &movement.item))
                }) {
                    return Err(invalid(
                        "recipient must participate and use the reserved item",
                    ));
                }
            }
        }
        if self.score_vector
            != ScoreVector::aggregate(
                &self
                    .teams
                    .iter()
                    .map(|t| t.score_vector.clone())
                    .collect::<Vec<_>>(),
            )?
        {
            return Err(invalid(
                "allocation vector must be the equal-weight encounter mean",
            ));
        }
        if self.presentation_score != spec.objective.score(&self.score_vector)? {
            return Err(invalid("presentation score does not match objective"));
        }
        if spec.constraints.required_rules.iter().any(|rule| {
            !self
                .constraints
                .iter()
                .any(|c| &c.rule == rule && c.hard && c.satisfied)
        }) {
            return Err(invalid("required hard rule has no satisfied outcome"));
        }
        if self.constraints.iter().any(|c| c.hard && !c.satisfied) {
            return Err(invalid("ranked allocation violates hard constraint"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstraintOutcome {
    pub rule: RuleId,
    pub hard: bool,
    pub satisfied: bool,
    pub reasons: Vec<ReasonId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReasonCode {
    BuffCompatible,
    BuffInapplicable,
    AuraRequirementSatisfied,
    FieldTimeConflict,
    EnergyDeficit,
    ReactionInterference,
    ResonanceActive,
    SpecialSystemActive,
    SustainSufficient,
    ResourceConflict,
    EnemyDependent,
    DonorLoss,
    RecipientGain,
    ReplacementRequired,
    ArtifactRedistributed,
    GlobalAllocationChanged,
    ConstraintSatisfied,
    ConstraintFailed,
    UnsupportedExcluded,
    SearchLimitReached,
    AssumptionChanged,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Subject {
    Account(SnapshotId),
    Character(CharacterId),
    Item(ItemId),
    Team(TeamId),
    Allocation(AllocationId),
    Dimension(Dimension),
    Assumption(DefinitionId),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CausalReason {
    pub id: ReasonId,
    pub code: ReasonCode,
    pub rule: RuleId,
    pub subjects: Vec<Subject>,
    pub causes: Vec<ReasonId>,
    pub parameters: BTreeMap<String, Parameter>,
    pub provenance: Vec<RevisionId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizedUncertainty {
    pub subject: Subject,
    pub metric: Option<DefinitionId>,
    pub fidelity: Fidelity,
    pub assumptions: Vec<DefinitionId>,
    pub explanation: String,
    pub provenance: Vec<RevisionId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reproducibility {
    pub account_semantics: Digest,
    pub mechanics: MechanicsRef,
    pub engine_version: String,
    pub engine_revision: RevisionId,
    pub decision_spec: Digest,
    pub fingerprint: Digest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStatus {
    Complete,
    Bounded,
    Cancelled,
    Infeasible,
    BlockedByUnsupportedMechanics,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchCounters {
    pub build_nodes: Counter,
    pub team_nodes: Counter,
    pub allocation_nodes: Counter,
    pub pruned_nodes: Counter,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchLimit {
    BuildNodes,
    TeamNodes,
    AllocationNodes,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum FrontierPresentation {
    Exhaustive,
    Truncated { omitted: Counter },
    AmongReturned,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchReport {
    pub status: SearchStatus,
    pub frontier: FrontierPresentation,
    pub counters: SearchCounters,
    pub domain: String,
    pub limits_reached: Vec<SearchLimit>,
    pub justified_upper_bound: Option<Fixed>,
    pub bound_provenance: Vec<RuleId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionResult {
    pub reproducibility: Reproducibility,
    pub spec: DecisionSpec,
    pub scope: EvaluationScope,
    pub exclusions: Vec<ImportIssue>,
    pub allocations: Vec<Allocation>,
    pub search: SearchReport,
    pub reasons: Vec<CausalReason>,
    pub uncertainty: Vec<LocalizedUncertainty>,
    pub provenance: Vec<Provenance>,
}
impl DecisionResult {
    pub fn validate_for(&self, account: &ValidatedAccount) -> ContractResult<()> {
        self.spec.validate_for(account)?;
        let reproduction = &self.reproducibility;
        if reproduction.account_semantics != wire::account_semantic_digest(account.snapshot())?
            || reproduction.decision_spec != wire::decision_spec_digest(&self.spec)?
            || reproduction.fingerprint
                != wire::result_fingerprint(
                    account.snapshot(),
                    &self.spec,
                    &reproduction.engine_version,
                    &reproduction.engine_revision,
                )?
            || reproduction.mechanics != self.spec.mechanics
            || self.scope != self.spec.scope
        {
            return Err(ContractError::new(
                IssueCode::InvalidAssignment,
                "/reproducibility",
                "result context differs from specification",
            ));
        }
        if (matches!(
            self.search.status,
            SearchStatus::Infeasible | SearchStatus::BlockedByUnsupportedMechanics
        ) && !self.allocations.is_empty())
            || (matches!(
                self.search.status,
                SearchStatus::Complete | SearchStatus::Infeasible
            ) && !self.search.limits_reached.is_empty())
            || (self.search.status == SearchStatus::Bounded
                && self.search.limits_reached.is_empty())
            || (self.search.justified_upper_bound.is_some()
                && self.search.bound_provenance.is_empty())
        {
            return Err(ContractError::new(
                IssueCode::InvalidSearchReport,
                "/search",
                "inconsistent completeness or bound evidence",
            ));
        }
        if self.search.status == SearchStatus::Infeasible
            && matches!(self.search.frontier, FrontierPresentation::Truncated { .. })
        {
            return Err(ContractError::new(
                IssueCode::InvalidSearchReport,
                "/search/frontier",
                "empty feasible domain cannot omit frontier allocations",
            ));
        }
        if self.search.status == SearchStatus::Complete && self.allocations.is_empty() {
            return Err(ContractError::new(
                IssueCode::InvalidSearchReport,
                "/search",
                "empty exhaustive domain must report infeasible",
            ));
        }
        if self.allocations.windows(2).any(|pair| {
            pair[0].presentation_score < pair[1].presentation_score
                || (pair[0].presentation_score == pair[1].presentation_score
                    && pair[0].id >= pair[1].id)
        }) {
            return Err(ContractError::new(
                IssueCode::InvalidAssignment,
                "/allocations",
                "rank by descending score, then ascending allocation id",
            ));
        }
        if self.allocations.len() > usize::from(self.spec.search.max_results) {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/allocations",
                "too many returned allocations",
            ));
        }
        if self.reasons.len() > MAX_RESULT_REASONS
            || self.exclusions.len() > MAX_DIAGNOSTICS
            || self.uncertainty.len() > MAX_DIAGNOSTICS
            || self.provenance.len() > MAX_DIAGNOSTICS
        {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/result",
                "result metadata budget exceeded",
            ));
        }
        if matches!(
            self.search.frontier,
            FrontierPresentation::Exhaustive | FrontierPresentation::Truncated { .. }
        ) && !matches!(
            self.search.status,
            SearchStatus::Complete | SearchStatus::Infeasible
        ) {
            return Err(ContractError::new(
                IssueCode::InvalidSearchReport,
                "/search/frontier",
                "unsearched space cannot have an exhaustive frontier",
            ));
        }
        if self.search.counters.build_nodes > self.spec.search.max_build_nodes
            || self.search.counters.team_nodes > self.spec.search.max_team_nodes
            || self.search.counters.allocation_nodes > self.spec.search.max_allocation_nodes
            || self.search.justified_upper_bound.is_some_and(|bound| {
                self.allocations
                    .first()
                    .is_some_and(|a| bound < a.presentation_score)
            })
        {
            return Err(ContractError::new(
                IssueCode::InvalidSearchReport,
                "/search",
                "work counters or upper bound contradict results",
            ));
        }
        let mut reached = BTreeSet::new();
        for limit in &self.search.limits_reached {
            let (count, maximum) = match limit {
                SearchLimit::BuildNodes => (
                    self.search.counters.build_nodes,
                    self.spec.search.max_build_nodes,
                ),
                SearchLimit::TeamNodes => (
                    self.search.counters.team_nodes,
                    self.spec.search.max_team_nodes,
                ),
                SearchLimit::AllocationNodes => (
                    self.search.counters.allocation_nodes,
                    self.spec.search.max_allocation_nodes,
                ),
            };
            if count != maximum || !reached.insert(limit) {
                return Err(ContractError::new(
                    IssueCode::InvalidSearchReport,
                    "/search/limits_reached",
                    "each reached limit must uniquely identify an exhausted node budget",
                ));
            }
        }
        let mut ids = BTreeSet::new();
        for allocation in &self.allocations {
            if !ids.insert(&allocation.id) {
                return Err(ContractError::new(
                    IssueCode::DuplicateIdentity,
                    "/allocations",
                    "duplicate allocation id",
                ));
            }
            allocation.validate(account.snapshot(), &self.spec)?;
        }
        if matches!(
            self.search.frontier,
            FrontierPresentation::Truncated {
                omitted: Counter(0)
            }
        ) {
            return Err(ContractError::new(
                IssueCode::InvalidSearchReport,
                "/search/frontier",
                "truncation must omit at least one frontier allocation",
            ));
        }
        for allocation in &self.allocations {
            if !matches!(self.search.frontier, FrontierPresentation::AmongReturned)
                && (matches!(allocation.pareto, ParetoStatus::Unknown)
                    || allocation
                        .score_vector
                        .dimensions
                        .values()
                        .any(|m| matches!(m, Metric::Unavailable { .. })))
            {
                return Err(ContractError::new(
                    IssueCode::InvalidSearchReport,
                    "/search/frontier",
                    "global frontier presentation needs comparable classified vectors",
                ));
            }
            let dominators = self
                .allocations
                .iter()
                .filter(|other| other.id != allocation.id)
                .filter_map(|other| {
                    match other.score_vector.pareto_against(&allocation.score_vector) {
                        Ok(ParetoRelation::Dominates) => Some(other.id.clone()),
                        _ => None,
                    }
                })
                .collect::<BTreeSet<_>>();
            match &allocation.pareto {
                ParetoStatus::Frontier
                    if !dominators.is_empty()
                        || allocation
                            .score_vector
                            .dimensions
                            .values()
                            .any(|m| matches!(m, Metric::Unavailable { .. })) =>
                {
                    return Err(ContractError::new(
                        IssueCode::InvalidSearchReport,
                        "/pareto",
                        "unsupported frontier claim",
                    ));
                }
                ParetoStatus::Dominated { by }
                    if by.is_empty() || by.iter().any(|id| !dominators.contains(id)) =>
                {
                    return Err(ContractError::new(
                        IssueCode::InvalidSearchReport,
                        "/pareto",
                        "dominance needs returned witnesses",
                    ));
                }
                _ => {}
            }
        }
        let mut reasons = BTreeSet::new();
        for reason in &self.reasons {
            if reason.causes.iter().any(|c| !reasons.contains(c))
                || !reasons.insert(reason.id.clone())
            {
                return Err(ContractError::new(
                    IssueCode::InvalidReference,
                    "/reasons",
                    "duplicate reason or cause does not precede effect",
                ));
            }
        }
        for allocation in &self.allocations {
            for reason in allocation
                .reasons
                .iter()
                .chain(allocation.constraints.iter().flat_map(|c| &c.reasons))
                .chain(
                    allocation
                        .teams
                        .iter()
                        .flat_map(|t| &t.builds)
                        .flat_map(|b| &b.reasons),
                )
            {
                if !reasons.contains(reason) {
                    return Err(ContractError::new(
                        IssueCode::InvalidReference,
                        "/reasons",
                        "dangling reason reference",
                    ));
                }
            }
        }
        if wire::encode(self)?.len() > MAX_RESULT_BYTES {
            return Err(ContractError::new(
                IssueCode::ResourceLimit,
                "/result",
                "result exceeds byte budget",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterChange {
    pub character: CharacterId,
    pub before_builds: Vec<BuildId>,
    pub after_builds: Vec<BuildId>,
    pub metric_deltas: BTreeMap<DefinitionId, Metric>,
    pub reasons: Vec<ReasonId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonClaim {
    ExactDifference,
    BestFoundDifference,
    NotComparable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DonorChange {
    pub availability: AvailabilityId,
    pub forms: Vec<CharacterChange>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonResult {
    pub comparison: ComparisonSpec,
    pub original_owner: OwnerRef,
    pub before: DecisionResult,
    pub after: DecisionResult,
    pub recipient: CharacterChange,
    pub donor: Option<DonorChange>,
    pub other_changes: Vec<CharacterChange>,
    pub global_delta: Metric,
    pub claim: ComparisonClaim,
    pub reasons: Vec<CausalReason>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityScenario {
    pub id: ScenarioId,
    pub label: String,
    pub assumptions: Assumptions,
    pub encounters: Vec<Encounter>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioResult {
    pub scenario: SensitivityScenario,
    pub result: DecisionResult,
    pub ranking_changed: bool,
    pub global_delta: Metric,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityResult {
    pub baseline: DecisionResult,
    pub scenarios: Vec<ScenarioResult>,
}
