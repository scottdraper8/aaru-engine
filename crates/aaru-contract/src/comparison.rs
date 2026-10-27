use crate::*;
use std::collections::BTreeSet;
impl ComparisonSpec {
    pub fn validate_for(&self, account: &ValidatedAccount) -> ContractResult<()> {
        self.baseline.validate_for(account)?;
        account.snapshot().validate_move(&self.movement)?;
        if let Some((OwnerRef::Character(owner), _)) =
            account.snapshot().item_owner(&self.movement.item)
            && account.snapshot().availability(owner)?
                == account.snapshot().availability(&self.movement.recipient)?
        {
            return Err(invalid(
                "selecting another form is not an equipment transfer between owned characters",
            ));
        }
        let expected = match &self.mode {
            ComparisonMode::CurrentLoadout {
                allocation,
                donor_fallback,
            } => {
                allocation.validate(account.snapshot(), &self.baseline)?;
                if !allocation
                    .footprint
                    .characters
                    .contains(&self.movement.recipient)
                {
                    return Err(invalid(
                        "recipient must participate in the fixed current party allocation",
                    ));
                }
                validate_fallback(
                    allocation,
                    account.snapshot(),
                    &self.movement,
                    donor_fallback.as_ref(),
                )?;
                AssignmentDomain::CurrentLoadout
            }
            ComparisonMode::UnrestrictedOptimum => AssignmentDomain::Unrestricted,
        };
        if self.baseline.assignments != expected {
            return Err(invalid(
                "comparison mode differs from baseline assignment domain",
            ));
        }
        Ok(())
    }
    pub fn intervention_spec(&self) -> DecisionSpec {
        let mut after = self.baseline.clone();
        after.assignments = match &self.mode {
            ComparisonMode::CurrentLoadout {
                allocation,
                donor_fallback,
            } => AssignmentDomain::MoveOnly {
                allocation: allocation.clone(),
                movement: self.movement.clone(),
                donor_fallback: donor_fallback.clone(),
            },
            ComparisonMode::UnrestrictedOptimum => AssignmentDomain::ReservedMove {
                movement: self.movement.clone(),
            },
        };
        after
    }
}
fn invalid(message: &str) -> ContractError {
    ContractError::new(IssueCode::InvalidAssignment, "/comparison", message)
}
impl ComparisonResult {
    pub fn validate_for(&self, account: &ValidatedAccount) -> ContractResult<()> {
        self.comparison.validate_for(account)?;
        let movement = &self.comparison.movement;
        let snapshot = account.snapshot();
        if self.before.spec != self.comparison.baseline
            || self.after.spec != self.comparison.intervention_spec()
            || self.before.reproducibility.engine_version
                != self.after.reproducibility.engine_version
            || self.before.reproducibility.engine_revision
                != self.after.reproducibility.engine_revision
            || self.original_owner
                != *snapshot
                    .item_owner(&movement.item)
                    .ok_or_else(|| invalid("unknown moved item"))?
                    .0
            || self.recipient.character != movement.recipient
        {
            return Err(invalid("comparison provenance, owner or recipient differs"));
        }
        self.before.validate_for(account)?;
        self.after.validate_for(account)?;
        match &self.original_owner {
            OwnerRef::Character(owner) => {
                let availability = snapshot.availability(owner)?;
                let mut forms = self
                    .before
                    .allocations
                    .first()
                    .into_iter()
                    .chain(self.after.allocations.first())
                    .flat_map(|a| &a.teams)
                    .flat_map(|t| &t.builds)
                    .filter(|b| snapshot.availability(&b.character).ok() == Some(availability))
                    .map(|b| b.character.clone())
                    .collect::<BTreeSet<_>>();
                if forms.is_empty() {
                    forms.insert(owner.clone());
                }
                if self.donor.as_ref().is_none_or(|donor| {
                    &donor.availability != availability
                        || donor
                            .forms
                            .iter()
                            .map(|c| c.character.clone())
                            .collect::<BTreeSet<_>>()
                            != forms
                        || donor.forms.len() != forms.len()
                }) {
                    return Err(invalid(
                        "donor loss must identify every participating physical-owner form",
                    ));
                }
            }
            _ if self.donor.is_some() => return Err(invalid("unexpected donor")),
            _ => {}
        }
        if let ComparisonMode::CurrentLoadout {
            allocation,
            donor_fallback,
        } = &self.comparison.mode
        {
            if self.before.allocations.as_slice() != [allocation.as_ref().clone()]
                || self.after.allocations.len() > 1
            {
                return Err(invalid(
                    "current comparison must evaluate only the fixed allocation",
                ));
            }
            for after in &self.after.allocations {
                validate_move_only(
                    allocation,
                    after,
                    snapshot,
                    movement,
                    donor_fallback.as_ref(),
                )?;
            }
        }
        let exact = self.before.search.status == SearchStatus::Complete
            && self.after.search.status == SearchStatus::Complete;
        let feasible = self
            .before
            .allocations
            .first()
            .zip(self.after.allocations.first());
        let expected_claim = if feasible.is_none() {
            ComparisonClaim::NotComparable
        } else if exact {
            ComparisonClaim::ExactDifference
        } else {
            ComparisonClaim::BestFoundDifference
        };
        if self.claim != expected_claim {
            return Err(invalid("comparison claim differs from search evidence"));
        }
        match (feasible, &self.global_delta) {
            (Some((before, after)), Metric::Available { value, .. }) => {
                if after
                    .presentation_score
                    .checked_sub(before.presentation_score)
                    .map_err(|_| invalid("delta overflow"))?
                    != *value
                {
                    return Err(invalid("delta differs from allocation scores"));
                }
                if matches!(self.comparison.mode, ComparisonMode::UnrestrictedOptimum)
                    && self.before.search.status == SearchStatus::Complete
                    && *value > Fixed::ZERO
                {
                    return Err(invalid(
                        "assignment restriction cannot improve a complete unrestricted optimum",
                    ));
                }
            }
            (None, Metric::Unavailable { .. }) => {}
            _ => {
                return Err(invalid(
                    "delta availability differs from comparable feasible results",
                ));
            }
        }
        let selected_characters = |result: &DecisionResult| {
            result
                .allocations
                .first()
                .into_iter()
                .flat_map(|a| &a.teams)
                .flat_map(|t| &t.builds)
                .map(|b| b.character.clone())
                .collect::<BTreeSet<_>>()
        };
        let mut changes = BTreeSet::new();
        for change in std::iter::once(&self.recipient)
            .chain(self.donor.iter().flat_map(|d| &d.forms))
            .chain(&self.other_changes)
        {
            if !snapshot
                .characters
                .records()
                .iter()
                .any(|c| c.id == change.character)
                || !changes.insert(&change.character)
            {
                return Err(invalid("unknown or duplicate character change"));
            }
            let builds = |result: &DecisionResult| {
                result
                    .allocations
                    .first()
                    .into_iter()
                    .flat_map(|a| &a.teams)
                    .flat_map(|t| &t.builds)
                    .filter(|b| b.character == change.character)
                    .map(|b| b.id.clone())
                    .collect::<Vec<_>>()
            };
            if change.before_builds != builds(&self.before)
                || change.after_builds != builds(&self.after)
            {
                return Err(invalid(
                    "character change does not identify selected builds",
                ));
            }
        }
        if selected_characters(&self.before)
            .union(&selected_characters(&self.after))
            .any(|c| !changes.contains(c))
        {
            return Err(invalid(
                "selected character changes must be exhaustive, including unchanged controls",
            ));
        }
        let mut reasons = BTreeSet::new();
        if self.reasons.len() > MAX_RESULT_REASONS {
            return Err(invalid("comparison reason budget exceeded"));
        }
        for reason in &self.reasons {
            if reason.causes.iter().any(|c| !reasons.contains(c))
                || !reasons.insert(reason.id.clone())
            {
                return Err(invalid("comparison causes must precede effects"));
            }
        }
        for change in std::iter::once(&self.recipient)
            .chain(self.donor.iter().flat_map(|d| &d.forms))
            .chain(&self.other_changes)
        {
            if change.reasons.iter().any(|r| !reasons.contains(r)) {
                return Err(invalid("dangling character change reason"));
            }
        }
        Ok(())
    }
}
fn validate_fallback(
    before: &Allocation,
    account: &AccountSnapshot,
    movement: &ItemMove,
    fallback: Option<&ItemId>,
) -> ContractResult<()> {
    account.validate_move(movement)?;
    if !before.footprint.characters.contains(&movement.recipient) {
        return Err(invalid("move-only recipient must participate"));
    }
    if let Some(item) = fallback {
        let donor = account
            .item_owner(&movement.item)
            .ok_or_else(|| invalid("unknown moved item"))?
            .0;
        let OwnerRef::Character(donor) = donor else {
            return Err(invalid("fallback requires a resolved donor"));
        };
        let availability = account.availability(donor)?;
        let participating = before
            .teams
            .iter()
            .flat_map(|t| &t.builds)
            .find(|b| account.availability(&b.character).ok() == Some(availability))
            .ok_or_else(|| invalid("fallback requires a participating donor form"))?;
        if availability == account.availability(&movement.recipient)?
            || account.item_owner(item).map(|(owner, _)| owner) != Some(&OwnerRef::Unequipped)
        {
            return Err(invalid(
                "fallback must be unequipped and used by a participating distinct donor",
            ));
        }
        account.validate_move(&ItemMove {
            item: item.clone(),
            recipient: participating.character.clone(),
        })?;
        let slot = |id: &ItemId| {
            account
                .artifacts
                .records()
                .iter()
                .find(|a| &a.id == id)
                .map(|a| a.slot)
        };
        if slot(item) != slot(&movement.item) {
            return Err(invalid(
                "fallback must have the moved equipment's slot/type",
            ));
        }
    }
    Ok(())
}
pub(crate) fn validate_move_only(
    before: &Allocation,
    after: &Allocation,
    account: &AccountSnapshot,
    movement: &ItemMove,
    fallback: Option<&ItemId>,
) -> ContractResult<()> {
    validate_fallback(before, account, movement, fallback)?;
    if before.teams.len() != after.teams.len() {
        return Err(invalid("unrelated party reoptimization"));
    }
    for (before_team, after_team) in before.teams.iter().zip(&after.teams) {
        if before_team.encounter_index != after_team.encounter_index
            || before_team.builds.len() != after_team.builds.len()
        {
            return Err(invalid("unrelated encounter reoptimization"));
        }
        for (before_build, after_build) in before_team.builds.iter().zip(&after_team.builds) {
            let mut expected = before_build.clone();
            if expected.weapon.as_ref() == Some(&movement.item) {
                expected.weapon = fallback.cloned();
            }
            for item in expected.artifacts.values_mut() {
                if item == &movement.item
                    && let Some(fallback) = fallback
                {
                    *item = fallback.clone();
                }
            }
            expected.artifacts.retain(|_, i| i != &movement.item);
            if expected.character == movement.recipient {
                if account
                    .weapons
                    .records()
                    .iter()
                    .any(|w| w.id == movement.item)
                {
                    expected.weapon = Some(movement.item.clone());
                } else if let Some(a) = account
                    .artifacts
                    .records()
                    .iter()
                    .find(|a| a.id == movement.item)
                {
                    expected.artifacts.insert(a.slot, movement.item.clone());
                }
            }
            if expected.character != after_build.character
                || expected.weapon != after_build.weapon
                || expected.artifacts != after_build.artifacts
            {
                return Err(invalid(
                    "move-only comparison changed unrelated equipment or party membership",
                ));
            }
        }
    }
    Ok(())
}
