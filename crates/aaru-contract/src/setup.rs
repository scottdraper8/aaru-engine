use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_PUBLIC_DEFINITIONS: usize = 32768;
pub const MAX_PRESETS: usize = 16;
pub const MAX_EDITABLE_PARAMETERS: usize = 128;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionKind {
    Character,
    Weapon,
    ArtifactSet,
    Material,
    Stat,
    Encounter,
    Assumption,
    Archetype,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicDefinition {
    pub id: DefinitionId,
    pub kind: DefinitionKind,
    pub label: String,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParameterTarget {
    Assumption { key: DefinitionId },
    Encounter { index: u16, key: DefinitionId },
    TargetCount { index: u16 },
    ObjectiveWeight { dimension: Dimension },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParameterDomain {
    #[serde(deserialize_with = "crate::wire::deserialize_empty_variant")]
    Flag,
    Quantity {
        unit: QuantityUnit,
        minimum: Fixed,
        maximum: Fixed,
    },
    Choice {
        labels: BTreeMap<DefinitionId, String>,
    },
    Count {
        minimum: u16,
        maximum: u16,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditableParameter {
    pub target: ParameterTarget,
    pub label: String,
    pub explanation: String,
    pub fidelity: Fidelity,
    pub provenance: Vec<RevisionId>,
    pub domain: ParameterDomain,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionPreset {
    pub id: DefinitionId,
    pub revision: RevisionId,
    pub label: String,
    pub spec: DecisionSpec,
    pub parameters: Vec<EditableParameter>,
}
impl DecisionPreset {
    pub fn validate(
        &self,
        mechanics: &MechanicsRef,
        normalization: &Normalization,
    ) -> ContractResult<()> {
        label(&self.label)?;
        self.spec.validate_shape()?;
        ensure(
            &self.spec.mechanics == mechanics && &self.spec.normalization == normalization,
            "preset mechanics/normalization mismatch",
        )?;
        ensure(
            !self.parameters.is_empty() && self.parameters.len() <= MAX_EDITABLE_PARAMETERS,
            "parameter metadata bounds",
        )?;
        let mut expected = self
            .spec
            .assumptions
            .values
            .keys()
            .map(|key| ParameterTarget::Assumption { key: key.clone() })
            .collect::<BTreeSet<_>>();
        for (index, encounter) in self.spec.encounters.iter().enumerate() {
            expected.insert(ParameterTarget::TargetCount {
                index: index as u16,
            });
            expected.extend(
                encounter
                    .parameters
                    .keys()
                    .map(|key| ParameterTarget::Encounter {
                        index: index as u16,
                        key: key.clone(),
                    }),
            );
        }
        expected.extend(self.spec.objective.weights.keys().map(|dimension| {
            ParameterTarget::ObjectiveWeight {
                dimension: *dimension,
            }
        }));
        let mut seen = BTreeSet::new();
        for parameter in &self.parameters {
            label(&parameter.label)?;
            ensure(
                !parameter.explanation.is_empty()
                    && parameter.explanation.len() <= 1024
                    && parameter.provenance.len() <= MAX_DIAGNOSTICS,
                "explicit bounded parameter explanation required",
            )?;
            ensure(
                expected.contains(&parameter.target) && seen.insert(parameter.target.clone()),
                "unknown or duplicate parameter target",
            )?;
            let value = match &parameter.target {
                ParameterTarget::Assumption { key } => {
                    self.spec.assumptions.values.get(key).cloned()
                }
                ParameterTarget::Encounter { index, key } => self
                    .spec
                    .encounters
                    .get(usize::from(*index))
                    .and_then(|e| e.parameters.get(key))
                    .cloned(),
                ParameterTarget::ObjectiveWeight { dimension } => self
                    .spec
                    .objective
                    .weights
                    .get(dimension)
                    .map(|v| AssumptionValue::Quantity(*v)),
                ParameterTarget::TargetCount { .. } => None,
            };
            match (&parameter.domain, value) {
                (ParameterDomain::Flag, Some(AssumptionValue::Flag(_))) => {}
                (
                    ParameterDomain::Quantity {
                        minimum, maximum, ..
                    },
                    Some(AssumptionValue::Quantity(v)),
                ) => {
                    ensure(
                        minimum <= maximum && minimum <= &v && &v <= maximum,
                        "quantity preset outside domain",
                    )?;
                    if matches!(parameter.target, ParameterTarget::ObjectiveWeight { .. }) {
                        ensure(
                            *minimum >= Fixed::ZERO,
                            "objective domain permits negative weights",
                        )?;
                    }
                }
                (ParameterDomain::Choice { labels }, Some(AssumptionValue::Choice(v))) => {
                    ensure(
                        !labels.is_empty() && labels.len() <= 64 && labels.contains_key(&v),
                        "choice preset outside domain",
                    )?;
                    for text in labels.values() {
                        label(text)?;
                    }
                }
                (ParameterDomain::Count { minimum, maximum }, None) => {
                    let ParameterTarget::TargetCount { index } = parameter.target else {
                        return Err(invalid("count domain has wrong target"));
                    };
                    let count = self.spec.encounters[usize::from(index)].target_count;
                    ensure(
                        *minimum > 0
                            && minimum <= maximum
                            && minimum <= &count
                            && &count <= maximum,
                        "target count outside domain",
                    )?;
                }
                _ => return Err(invalid("parameter type differs from explicit preset value")),
            }
        }
        ensure(
            seen == expected,
            "all preset assumptions, encounter parameters, counts and weights need metadata",
        )
    }
}
impl Catalog {
    pub fn validate(&self) -> ContractResult<()> {
        ensure(
            self.samples.len() <= 4
                && self.mechanics.len() <= 32
                && self.definitions.len() <= MAX_PUBLIC_DEFINITIONS,
            "catalog bounds",
        )?;
        let mut definitions = BTreeMap::new();
        for definition in &self.definitions {
            label(&definition.label)?;
            ensure(
                definitions
                    .insert(&definition.id, &definition.kind)
                    .is_none(),
                "duplicate public definition",
            )?;
        }
        let mut revisions = BTreeSet::new();
        for entry in &self.mechanics {
            label(&entry.label)?;
            entry.normalization.validate()?;
            let expected_path = match &entry.mechanics {
                MechanicsRef::Synthetic { sample, .. } => {
                    ensure(
                        sample.is_synthetic(),
                        "preview cannot supply synthetic mechanics",
                    )?;
                    format!("data/mechanics/synthetic/{}.json", sample_name(*sample))
                }
                MechanicsRef::Live { revision, .. } => {
                    let component = revision.as_str();
                    ensure(
                        component
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                            && component
                                .as_bytes()
                                .first()
                                .is_some_and(u8::is_ascii_alphanumeric)
                            && component
                                .as_bytes()
                                .last()
                                .is_some_and(u8::is_ascii_alphanumeric)
                            && !component.contains(".."),
                        "mechanics asset revision is not a safe filename component",
                    )?;
                    format!("data/mechanics/live/{component}.json")
                }
            };
            ensure(entry.path == expected_path, "mechanics asset path mismatch")?;
            let revision = match &entry.mechanics {
                MechanicsRef::Synthetic { revision, .. } | MechanicsRef::Live { revision, .. } => {
                    revision
                }
            };
            ensure(
                revisions.insert(revision)
                    && !entry.presets.is_empty()
                    && entry.presets.len() <= MAX_PRESETS,
                "mechanics revision or preset bounds",
            )?;
            ensure(
                entry
                    .covered_definitions
                    .is_disjoint(&entry.unsupported_definitions)
                    && entry.provenance.len() <= MAX_DIAGNOSTICS,
                "conflicting definition coverage",
            )?;
            ensure(
                entry
                    .covered_definitions
                    .iter()
                    .chain(&entry.unsupported_definitions)
                    .all(|d| definitions.contains_key(d)),
                "missing public definition labels",
            )?;
            let mut ids = BTreeSet::new();
            for preset in &entry.presets {
                ensure(ids.insert(&preset.id), "duplicate preset")?;
                preset.validate(&entry.mechanics, &entry.normalization)?;
                for encounter in &preset.spec.encounters {
                    ensure(
                        definitions.get(&encounter.definition) == Some(&&DefinitionKind::Encounter),
                        "encounter label missing or wrong kind",
                    )?;
                }
                for key in preset.spec.assumptions.values.keys().chain(
                    preset
                        .spec
                        .encounters
                        .iter()
                        .flat_map(|e| e.parameters.keys()),
                ) {
                    ensure(
                        definitions.get(key) == Some(&&DefinitionKind::Assumption),
                        "assumption label missing or wrong kind",
                    )?;
                }
            }
        }
        let mut samples = BTreeSet::new();
        for sample in &self.samples {
            label(&sample.label)?;
            ensure(
                samples.insert(sample.id)
                    && sample.description.len() <= 1024
                    && sample.expected_diagnostics.len() <= MAX_DIAGNOSTICS
                    && sample.oracle_cases.len() <= MAX_DIAGNOSTICS,
                "sample metadata bounds",
            )?;
            let expected_path = if sample.id.is_synthetic() {
                format!("fixtures/canonical/{}.json", sample_name(sample.id))
            } else {
                "fixtures/good/good-unsupported-preview.json".into()
            };
            ensure(
                sample.account_path == expected_path
                    && sample.fabricated_inventory == sample.id.is_synthetic(),
                "sample path or synthetic disclosure mismatch",
            )?;
            if sample.supported_operations.iter().any(|op| {
                matches!(
                    op,
                    Operation::Evaluate | Operation::Compare | Operation::Sensitivity
                )
            }) {
                ensure(
                    sample.mechanics.is_some(),
                    "decision capability requires explicit mechanics",
                )?;
            }
            if let Some(mechanics) = &sample.mechanics {
                ensure(
                    matches!(mechanics, MechanicsRef::Synthetic { sample: id, .. } if id == &sample.id && id.is_synthetic()),
                    "bundled sample mechanics binding mismatch",
                )?;
                ensure(
                    self.mechanics.iter().any(|m| {
                        &m.mechanics == mechanics
                            && sample.supported_operations.is_subset(&m.capabilities)
                    }),
                    "sample references missing mechanics or capabilities",
                )?;
            }
        }
        wire::encode(self)?;
        Ok(())
    }
}
fn sample_name(sample: SampleId) -> &'static str {
    match sample {
        SampleId::SynWelcome => "syn-welcome",
        SampleId::SynScarcity => "syn-scarcity",
        SampleId::SynStress => "syn-stress",
        SampleId::GoodUnsupportedPreview => "good-unsupported-preview",
    }
}
fn label(text: &str) -> ContractResult<()> {
    ensure(
        !text.is_empty() && text.len() <= 160 && !text.chars().any(char::is_control),
        "bounded literal label required",
    )
}
fn ensure(ok: bool, message: &str) -> ContractResult<()> {
    if ok { Ok(()) } else { Err(invalid(message)) }
}
fn invalid(message: &str) -> ContractError {
    ContractError::new(IssueCode::InvalidInput, "/catalog", message)
}
