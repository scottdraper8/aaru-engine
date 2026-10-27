use crate::*;
use std::collections::BTreeSet;

impl SensitivityResult {
    pub fn validate_for(&self, account: &ValidatedAccount) -> ContractResult<()> {
        self.baseline.validate_for(account)?;
        let invalid =
            |message| ContractError::new(IssueCode::InvalidAssignment, "/sensitivity", message);
        if self.scenarios.is_empty() || self.scenarios.len() > 16 {
            return Err(invalid("supply 1 to 16 scenarios"));
        }
        let mut ids = BTreeSet::new();
        for scenario in &self.scenarios {
            if !ids.insert(&scenario.scenario.id) {
                return Err(invalid("duplicate scenario identity"));
            }
            let mut expected = self.baseline.spec.clone();
            expected.assumptions = scenario.scenario.assumptions.clone();
            expected.encounters = scenario.scenario.encounters.clone();
            if scenario.result.spec != expected {
                return Err(invalid(
                    "scenario changed fields beyond assumptions and encounters",
                ));
            }
            scenario.result.validate_for(account)?;
            let before_ids: Vec<_> = self.baseline.allocations.iter().map(|a| &a.id).collect();
            let after_ids: Vec<_> = scenario.result.allocations.iter().map(|a| &a.id).collect();
            if scenario.ranking_changed != (before_ids != after_ids) {
                return Err(invalid("ranking summary differs from returned allocations"));
            }
            if let Metric::Available { value, .. } = &scenario.global_delta {
                let before = self
                    .baseline
                    .allocations
                    .first()
                    .ok_or_else(|| invalid("delta without baseline allocation"))?;
                let after = scenario
                    .result
                    .allocations
                    .first()
                    .ok_or_else(|| invalid("delta without scenario allocation"))?;
                if after
                    .presentation_score
                    .checked_sub(before.presentation_score)
                    .map_err(|_| invalid("delta overflow"))?
                    != *value
                {
                    return Err(invalid("scenario delta differs from allocation scores"));
                }
            }
        }
        Ok(())
    }
}
