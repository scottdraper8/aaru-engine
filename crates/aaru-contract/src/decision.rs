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
    /// Dot product; a single ties-to-even rounding after all products are summed.
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
        Ok(())
    }
    /// All dimensions maximize. Unknowns make vectors incomparable, never zero.
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

/// Semantic keys are resolved by the engine's mechanics catalog, not an ontology here.
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
    pub mechanics: MechanicsRef,
    pub scope: EvaluationScope,
    pub objective: Objective,
    /// One explicit encounter per simultaneous team; length defines team count.
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
#[serde(deny_unknown_fields)]
pub struct ResourceFootprint {
    pub characters: BTreeSet<CharacterId>,
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
        let mut characters = BTreeSet::new();
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
                    pin.owner == OwnerRef::Character(b.character.clone())
                        && (b.weapon.as_ref() == Some(&pin.item)
                            || b.artifacts.values().any(|i| i == &pin.item))
                })
            {
                return Err(invalid("allocation violates pinned assignment"));
            }
        }
        if self.footprint.characters != characters || self.footprint.items != items {
            return Err(invalid("footprint differs from exact assignments"));
        }
        if self.presentation_score != spec.objective.score(&self.score_vector)? {
            return Err(invalid("presentation score does not match objective"));
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchLimit {
    BuildNodes,
    TeamNodes,
    AllocationNodes,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchReport {
    pub status: SearchStatus,
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
        let mut reasons = BTreeSet::new();
        // Causes must precede effects, simultaneously forbidding cycles and dangling causes.
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
pub enum ImprovementClaim {
    ProvenWithinDeclaredDomain,
    BestFoundDifference,
    NotComparable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonResult {
    pub movement: ItemMove,
    pub original_owner: OwnerRef,
    pub before_ownership: BTreeMap<ItemId, OwnerRef>,
    pub after_ownership: BTreeMap<ItemId, OwnerRef>,
    pub before: DecisionResult,
    pub after: DecisionResult,
    pub recipient: CharacterChange,
    pub donor: Option<CharacterChange>,
    pub other_changes: Vec<CharacterChange>,
    pub global_delta: Metric,
    pub claim: ImprovementClaim,
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
