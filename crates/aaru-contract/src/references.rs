use crate::*;
use std::collections::{BTreeMap, BTreeSet};

pub struct SubjectContext<'a> {
    account: &'a AccountSnapshot,
    spec: &'a DecisionSpec,
    allocations: &'a [Allocation],
}
pub enum ResolvedSubject<'a> {
    Account(&'a AccountSnapshot),
    Character(&'a OwnedCharacter),
    Weapon(&'a WeaponInstance),
    Artifact(&'a ArtifactInstance),
    Team {
        allocation: &'a Allocation,
        team: &'a TeamAssignment,
    },
    Allocation(&'a Allocation),
    Dimension {
        dimension: Dimension,
        range: &'a MetricRange,
    },
    Assumption(&'a AssumptionValue),
}
impl DecisionResult {
    pub fn subject_context<'a>(&'a self, account: &'a ValidatedAccount) -> SubjectContext<'a> {
        SubjectContext {
            account: account.snapshot(),
            spec: &self.spec,
            allocations: &self.allocations,
        }
    }
}
impl SubjectContext<'_> {
    pub fn resolve<'a>(&'a self, subject: &Subject) -> ContractResult<ResolvedSubject<'a>> {
        let resolved = match subject {
            Subject::Account(id) if id == &self.account.id => {
                ResolvedSubject::Account(self.account)
            }
            Subject::Character(id) => ResolvedSubject::Character(
                self.account
                    .characters
                    .records()
                    .iter()
                    .find(|c| &c.id == id)
                    .ok_or_else(dangling)?,
            ),
            Subject::Item(id) => {
                if let Some(weapon) = self.account.weapons.records().iter().find(|w| &w.id == id) {
                    ResolvedSubject::Weapon(weapon)
                } else {
                    ResolvedSubject::Artifact(
                        self.account
                            .artifacts
                            .records()
                            .iter()
                            .find(|a| &a.id == id)
                            .ok_or_else(dangling)?,
                    )
                }
            }
            Subject::Team { allocation, team } => {
                let allocation = self.allocation(allocation)?;
                let team = allocation
                    .teams
                    .iter()
                    .find(|t| &t.id == team)
                    .ok_or_else(dangling)?;
                ResolvedSubject::Team { allocation, team }
            }
            Subject::Allocation(id) => ResolvedSubject::Allocation(self.allocation(id)?),
            Subject::Dimension(dimension) => ResolvedSubject::Dimension {
                dimension: *dimension,
                range: self
                    .spec
                    .normalization
                    .dimensions
                    .get(dimension)
                    .ok_or_else(dangling)?,
            },
            Subject::Assumption(reference) => {
                ResolvedSubject::Assumption(self.assumption(reference)?)
            }
            _ => return Err(dangling()),
        };
        Ok(resolved)
    }
    fn allocation(&self, id: &AllocationId) -> ContractResult<&Allocation> {
        self.allocations
            .iter()
            .find(|a| &a.id == id)
            .ok_or_else(dangling)
    }
    fn assumption(&self, reference: &AssumptionRef) -> ContractResult<&AssumptionValue> {
        match reference {
            AssumptionRef::Global { key } => self.spec.assumptions.values.get(key),
            AssumptionRef::Encounter { index, key } => self
                .spec
                .encounters
                .get(usize::from(*index))
                .and_then(|e| e.parameters.get(key)),
        }
        .ok_or_else(dangling)
    }
    pub(crate) fn validate_uncertainty(
        &self,
        uncertainty: &LocalizedUncertainty,
    ) -> ContractResult<()> {
        self.resolve(&uncertainty.subject)?;
        for assumption in &uncertainty.assumptions {
            self.assumption(assumption)?;
        }
        if let Some(metric) = &uncertainty.metric
            && !self.spec.constraints.metric_minima.contains_key(metric)
            && !self
                .allocations
                .iter()
                .filter(|a| match &uncertainty.subject {
                    Subject::Allocation(id) | Subject::Team { allocation: id, .. } => &a.id == id,
                    _ => true,
                })
                .flat_map(|a| &a.teams)
                .filter(|t| match &uncertainty.subject {
                    Subject::Team { team, .. } => &t.id == team,
                    _ => true,
                })
                .flat_map(|t| &t.builds)
                .filter(|b| match &uncertainty.subject {
                    Subject::Character(character) => &b.character == character,
                    Subject::Item(item) => {
                        b.weapon.as_ref() == Some(item) || b.artifacts.values().any(|i| i == item)
                    }
                    _ => true,
                })
                .any(|b| b.metrics.contains_key(metric))
        {
            return Err(dangling());
        }
        Ok(())
    }
}
pub(crate) fn validate_parameters(
    parameters: &BTreeMap<String, Parameter>,
    account: &AccountSnapshot,
) -> ContractResult<()> {
    for parameter in parameters.values() {
        let exists = match parameter {
            Parameter::Character(id) => account.characters.records().iter().any(|c| &c.id == id),
            Parameter::Item(id) => account.item_owner(id).is_some(),
            _ => true,
        };
        if !exists {
            return Err(dangling());
        }
    }
    Ok(())
}
pub(crate) fn validate_reasons<S>(
    reasons: &[CausalReason<S>],
    account: &AccountSnapshot,
    mut resolve: impl FnMut(&S) -> ContractResult<()>,
) -> ContractResult<BTreeSet<ReasonId>> {
    if reasons.len() > MAX_RESULT_REASONS {
        return Err(ContractError::new(
            IssueCode::ResourceLimit,
            "/reasons",
            "reason budget exceeded",
        ));
    }
    let mut seen = BTreeSet::new();
    for reason in reasons {
        if reason.causes.iter().any(|c| !seen.contains(c)) || !seen.insert(reason.id.clone()) {
            return Err(ContractError::new(
                IssueCode::InvalidReference,
                "/reasons",
                "duplicate reason or cause does not precede effect",
            ));
        }
        for subject in &reason.subjects {
            resolve(subject)?;
        }
        validate_parameters(&reason.parameters, account)?;
    }
    Ok(seen)
}
fn dangling() -> ContractError {
    ContractError::new(
        IssueCode::InvalidReference,
        "/explanations",
        "reference is absent from its declared account, specification or result context",
    )
}
