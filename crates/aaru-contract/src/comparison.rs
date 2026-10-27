use crate::*;
use std::collections::BTreeMap;

pub fn ownership(account: &AccountSnapshot) -> BTreeMap<ItemId, OwnerRef> {
    account
        .weapons
        .records()
        .iter()
        .map(|w| (w.id.clone(), w.owner.clone()))
        .chain(
            account
                .artifacts
                .records()
                .iter()
                .map(|a| (a.id.clone(), a.owner.clone())),
        )
        .collect()
}
impl ComparisonResult {
    pub fn validate_for(&self, account: &ValidatedAccount) -> ContractResult<()> {
        let invalid =
            |message| ContractError::new(IssueCode::InvalidAssignment, "/comparison", message);
        let snapshot = account.snapshot();
        snapshot.validate_move(&self.movement)?;
        if self.before_ownership != ownership(snapshot)
            || self.before.spec != self.after.spec
            || self.original_owner
                != *snapshot
                    .item_owner(&self.movement.item)
                    .ok_or_else(|| invalid("unknown moved item"))?
                    .0
            || self.recipient.character != self.movement.recipient
        {
            return Err(invalid("baseline, owner, or recipient mismatch"));
        }
        if self.before_ownership.keys().ne(self.after_ownership.keys()) {
            return Err(invalid(
                "ownership maps must contain every physical item exactly once",
            ));
        }
        if self.after_ownership.get(&self.movement.item)
            != Some(&OwnerRef::Character(self.movement.recipient.clone()))
        {
            return Err(invalid("hypothetical move not applied"));
        }
        match &self.original_owner {
            OwnerRef::Character(donor) if donor != &self.movement.recipient => {
                if self.donor.as_ref().map(|d| &d.character) != Some(donor) {
                    return Err(invalid("donor loss/replacement must be explicit"));
                }
            }
            _ if self.donor.is_some() => {
                return Err(invalid("donor supplied for unequipped or unchanged owner"));
            }
            _ => {}
        }
        let mut after = snapshot.clone();
        for (item, target) in &self.after_ownership {
            let (previous, lock) = snapshot
                .item_owner(item)
                .ok_or_else(|| invalid("unknown item"))?;
            if previous != target {
                if matches!(previous, OwnerRef::UnresolvedExternal(_))
                    || snapshot.preferences.protected_items.contains(item)
                    || (lock == Some(true)
                        && snapshot.preferences.game_lock_policy
                            == GameLockPolicy::ProtectFromReassignment)
                    || snapshot
                        .preferences
                        .pinned
                        .iter()
                        .any(|p| &p.item == item && &p.owner != target)
                {
                    return Err(invalid(
                        "hypothetical ownership violates reservation/protection",
                    ));
                }
                if let OwnerRef::Character(recipient) = target {
                    snapshot.validate_move(&ItemMove {
                        item: item.clone(),
                        recipient: recipient.clone(),
                    })?;
                }
            }
        }
        if let Section::Supplied { records, .. } = &mut after.weapons {
            for item in records {
                item.owner = self.after_ownership[&item.id].clone();
            }
        }
        if let Section::Supplied { records, .. } = &mut after.artifacts {
            for item in records {
                item.owner = self.after_ownership[&item.id].clone();
            }
        }
        let hypothetical = match account.sample() {
            Some(sample) => ValidatedAccount::bundled(after, sample)?,
            None => ValidatedAccount::imported(after)?,
        };
        self.before.validate_for(account)?;
        self.after.validate_for(&hypothetical)?;
        for build in self
            .after
            .allocations
            .iter()
            .flat_map(|a| &a.teams)
            .flat_map(|t| &t.builds)
        {
            if (build.weapon.as_ref() == Some(&self.movement.item)
                || build
                    .artifacts
                    .values()
                    .any(|item| item == &self.movement.item))
                && build.character != self.movement.recipient
            {
                return Err(invalid(
                    "requested item must remain reserved to its hypothetical recipient",
                ));
            }
        }
        if self.claim == ImprovementClaim::ProvenWithinDeclaredDomain
            && (self.before.search.status != SearchStatus::Complete
                || self.after.search.status != SearchStatus::Complete)
        {
            return Err(invalid("bounded results cannot prove an improvement"));
        }
        if let Metric::Available { value, .. } = &self.global_delta {
            let before = self
                .before
                .allocations
                .first()
                .ok_or_else(|| invalid("delta without baseline allocation"))?;
            let after = self
                .after
                .allocations
                .first()
                .ok_or_else(|| invalid("delta without hypothetical allocation"))?;
            if after
                .presentation_score
                .checked_sub(before.presentation_score)
                .map_err(|_| invalid("global delta overflow"))?
                != *value
            {
                return Err(invalid(
                    "global delta differs from complete allocation scores",
                ));
            }
        }
        Ok(())
    }
}
