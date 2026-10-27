use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    HigherIsBetter,
    LowerIsBetter,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricRange {
    pub unit: QuantityUnit,
    pub direction: Direction,
    pub low: Fixed,
    pub high: Fixed,
}
impl MetricRange {
    pub fn normalize(&self, raw: Fixed) -> ContractResult<Fixed> {
        if self.low >= self.high || raw < self.low || raw > self.high {
            return Err(ContractError::new(
                IssueCode::InvalidObjective,
                "/normalization",
                "invalid range or raw value outside declared range",
            ));
        }
        let span = i128::from(self.high.scaled()) - i128::from(self.low.scaled());
        let offset = match self.direction {
            Direction::HigherIsBetter => i128::from(raw.scaled()) - i128::from(self.low.scaled()),
            Direction::LowerIsBetter => i128::from(self.high.scaled()) - i128::from(raw.scaled()),
        };
        let normalized = numeric::round_ratio(offset * numeric::SCALE, span).map_err(|_| {
            ContractError::new(
                IssueCode::NumericOverflow,
                "/normalization",
                "normalization overflow",
            )
        })?;
        Ok(Fixed::from_scaled(i64::try_from(normalized).map_err(
            |_| {
                ContractError::new(
                    IssueCode::NumericOverflow,
                    "/normalization",
                    "normalization overflow",
                )
            },
        )?))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Normalization {
    pub revision: RevisionId,
    pub dimensions: BTreeMap<Dimension, MetricRange>,
}
impl Normalization {
    pub fn validate(&self) -> ContractResult<()> {
        if self.dimensions.len() != DIMENSIONS.len()
            || DIMENSIONS.iter().any(|d| !self.dimensions.contains_key(d))
            || self.dimensions.values().any(|r| r.low >= r.high)
        {
            return Err(ContractError::new(
                IssueCode::InvalidObjective,
                "/normalization",
                "all nine valid metric ranges are required",
            ));
        }
        Ok(())
    }
}
