//! All external JSON enters here, before a generic map can erase duplicate keys.
use crate::*;
use serde::de::{
    DeserializeOwned, DeserializeSeed, Error, IgnoredAny, MapAccess, SeqAccess, Visitor,
};
use std::{collections::BTreeSet, fmt};

pub const MAX_WIRE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_DEPTH: usize = 64;
pub const MAX_STRING_BYTES: usize = 4096;
pub const MAX_CONTAINER_ENTRIES: usize = 65536;

pub fn decode<T: DeserializeOwned>(text: &str) -> ContractResult<T> {
    if text.len() > MAX_WIRE_BYTES {
        return Err(ContractError::new(
            IssueCode::ResourceLimit,
            "",
            "wire message exceeds 128 MiB",
        ));
    }
    if text.starts_with('\u{feff}') {
        return Err(ContractError::new(
            IssueCode::InvalidInput,
            "",
            "UTF-8 BOM is not accepted",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_str(text);
    Guard {
        depth: 0,
        long_string: false,
    }
    .deserialize(&mut deserializer)
    .map_err(parse_error)?;
    deserializer.end().map_err(parse_error)?;
    serde_json::from_str(text).map_err(parse_error)
}
pub fn encode<T: serde::Serialize>(value: &T) -> ContractResult<String> {
    let text = serde_json::to_string(value).map_err(parse_error)?;
    // Apply identical limits to outbound and inbound data.
    let _: IgnoredAny = decode(&text)?;
    Ok(text)
}
pub fn decode_request(text: &str) -> ContractResult<WorkerRequest> {
    let request: WorkerRequest = decode(text)?;
    request.validate()?;
    Ok(request)
}
pub fn decode_account(text: &str) -> ContractResult<ValidatedAccount> {
    ValidatedAccount::imported(decode(text)?)
}
fn parse_error(error: serde_json::Error) -> ContractError {
    let message = error.to_string();
    let code = if message.contains("duplicate object property") {
        IssueCode::DuplicateProperty
    } else if message.contains("contract resource limit") {
        IssueCode::ResourceLimit
    } else if message.contains("unsupported ") {
        IssueCode::UnsupportedVersion
    } else {
        IssueCode::InvalidInput
    };
    ContractError::new(code, "", message)
}
struct Guard {
    depth: usize,
    long_string: bool,
}
impl<'de> DeserializeSeed<'de> for Guard {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        if self.depth > MAX_DEPTH {
            return Err(D::Error::custom("contract resource limit: depth"));
        }
        deserializer.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Guard {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded duplicate-free JSON")
    }
    fn visit_bool<E: Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<(), E> {
        let limit = if self.long_string {
            MAX_WIRE_BYTES
        } else {
            MAX_STRING_BYTES
        };
        if value.len() > limit {
            return Err(E::custom("contract resource limit: string"));
        }
        Ok(())
    }
    fn visit_string<E: Error>(self, value: String) -> Result<(), E> {
        self.visit_str(&value)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let mut count = 0;
        while seq
            .next_element_seed(Guard {
                depth: self.depth + 1,
                long_string: false,
            })?
            .is_some()
        {
            count += 1;
            if count > MAX_CONTAINER_ENTRIES {
                return Err(A::Error::custom("contract resource limit: array"));
            }
        }
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut seen = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if key.len() > MAX_STRING_BYTES || seen.len() >= MAX_CONTAINER_ENTRIES {
                return Err(A::Error::custom("contract resource limit: object"));
            }
            if !seen.insert(key.clone()) {
                return Err(A::Error::custom("duplicate object property"));
            }
            map.next_value_seed(Guard {
                depth: self.depth + 1,
                long_string: matches!(key.as_str(), "utf8" | "text"),
            })?;
        }
        Ok(())
    }
}

/// Hash exact normalized serialization for transaction acknowledgements (not the
/// semantic result fingerprint). Source bytes and all canonical IDs are included.
pub fn canonical_backup_digest(account: &AccountSnapshot) -> ContractResult<Digest> {
    account.validate()?;
    Ok(Digest::of(encode(account)?.as_bytes()))
}

/// Stable semantic inventory hash: sort physical records/sets; exclude source,
/// provenance, and capture metadata, but include section coverage and preferences.
pub fn account_semantic_digest(account: &AccountSnapshot) -> ContractResult<Digest> {
    account.validate()?;
    let mut normalized = account.clone();
    normalized.source_archive = None;
    normalized.provenance.clear();
    if let Section::Supplied { records, .. } = &mut normalized.characters {
        records.sort_by(|a, b| a.id.cmp(&b.id));
        for record in records {
            record.source = None;
        }
    }
    if let Section::Supplied { records, .. } = &mut normalized.weapons {
        records.sort_by(|a, b| a.id.cmp(&b.id));
        for record in records {
            record.source = None;
        }
    }
    if let Section::Supplied { records, .. } = &mut normalized.artifacts {
        records.sort_by(|a, b| a.id.cmp(&b.id));
        for record in records {
            record.source = None;
            if let Some(stats) = &mut record.substats {
                stats.sort_by(|a, b| a.key.cmp(&b.key));
            }
        }
    }
    normalized
        .preferences
        .pinned
        .sort_by(|a, b| a.item.cmp(&b.item));
    Ok(Digest::of(encode(&normalized)?.as_bytes()))
}

/// Hash a specification with BTreeMap/BTreeSet order and declared vector order.
pub fn decision_spec_digest(spec: &DecisionSpec) -> ContractResult<Digest> {
    Ok(Digest::of(encode(spec)?.as_bytes()))
}

/// SHA-256 over a JSON array with unambiguous component framing.
pub fn result_fingerprint(
    account: &AccountSnapshot,
    spec: &DecisionSpec,
    engine_version: &str,
    engine_revision: &RevisionId,
) -> ContractResult<Digest> {
    let account = account_semantic_digest(account)?;
    let spec_hash = decision_spec_digest(spec)?;
    Ok(Digest::of(
        encode(&(
            "aaru-result-v1",
            account,
            spec_hash,
            engine_version,
            engine_revision,
        ))?
        .as_bytes(),
    ))
}
