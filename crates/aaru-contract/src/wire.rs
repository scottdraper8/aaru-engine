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
    guard_numbers(text)?;
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
    let _: IgnoredAny = decode(&text)?;
    Ok(text)
}
pub fn decode_request(text: &str) -> ContractResult<WorkerRequest> {
    let request: WorkerRequest = decode(text)?;
    request.validate()?;
    Ok(request)
}
fn guard_numbers(text: &str) -> ContractResult<()> {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut in_string = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if in_string => {
                index += 2;
                continue;
            }
            b'"' => in_string = !in_string,
            b'-' | b'0'..=b'9' if !in_string => {
                let start = index;
                while index < bytes.len() && !b" \n\r\t,]}:".contains(&bytes[index]) {
                    index += 1;
                }
                let token = &text[start..index];
                let excessive_exponent = token.split_once(['e', 'E']).is_some_and(|(_, e)| {
                    e.parse::<i32>()
                        .map_or(true, |e| e.unsigned_abs() > numeric::MAX_EXPONENT as u32)
                });
                if token.len() > numeric::MAX_DECIMAL_BYTES || excessive_exponent {
                    return Err(ContractError::new(
                        IssueCode::ResourceLimit,
                        "",
                        "numeric lexeme/exponent budget exceeded",
                    ));
                }
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    Ok(())
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

pub fn snapshot_digest(account: &AccountSnapshot) -> ContractResult<Digest> {
    account.validate()?;
    Ok(Digest::of(encode(account)?.as_bytes()))
}

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

pub fn decision_spec_digest(spec: &DecisionSpec) -> ContractResult<Digest> {
    Ok(Digest::of(encode(spec)?.as_bytes()))
}

pub fn result_fingerprint(
    account: &AccountSnapshot,
    spec: &DecisionSpec,
    engine_version: &str,
    engine_revision: &RevisionId,
) -> ContractResult<Digest> {
    let account = account_semantic_digest(account)?;
    let spec_hash = decision_spec_digest(spec)?;
    fingerprint_from_digests(
        &account,
        &spec_hash,
        &EngineIdentity {
            version: engine_version.into(),
            revision: engine_revision.clone(),
        },
    )
}
pub fn fingerprint_from_digests(
    account: &Digest,
    spec: &Digest,
    engine: &EngineIdentity,
) -> ContractResult<Digest> {
    Ok(Digest::of(
        encode(&(
            "aaru-result-v1",
            account,
            spec,
            &engine.version,
            &engine.revision,
        ))?
        .as_bytes(),
    ))
}
pub fn operation_fingerprint(
    context: &RequestContext,
    generation: Counter,
    payload: &Request,
) -> ContractResult<Digest> {
    Ok(Digest::of(
        encode(&("aaru-operation-v1", context, generation, payload))?.as_bytes(),
    ))
}
