use super::*;
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;

fn strict_empty<T: Serialize + DeserializeOwned + Debug + PartialEq>(value: T, canonical: &str) {
    assert_eq!(wire::encode(&value).unwrap(), canonical);
    assert_eq!(wire::decode::<T>(canonical).unwrap(), value);
    let mut forged: serde_json::Value = serde_json::from_str(canonical).unwrap();
    forged["verified"] = true.into();
    forged["receipt"] = serde_json::json!({"revision": "7", "material_kinds": 2});
    assert!(
        wire::decode::<T>(&forged.to_string()).is_err(),
        "unexpected claims accepted for {canonical}"
    );
}

#[test]
fn empty_inventory_recovery_and_request_variants_reject_payload_claims() {
    strict_empty(CharacterAvailability::Unknown, r#"{"status":"unknown"}"#);
    strict_empty(
        ActiveSelection::ObservedDurable,
        r#"{"type":"observed_durable"}"#,
    );
    strict_empty(ActiveSelection::EmptySession, r#"{"type":"empty_session"}"#);
    strict_empty(
        StorageDisposition::SessionOnly,
        r#"{"type":"session_only"}"#,
    );
    strict_empty(Request::GetCatalog, r#"{"type":"get_catalog"}"#);
    strict_empty(Request::DiscardSession, r#"{"type":"discard_session"}"#);
    strict_empty(MaterialQuantity::Unknown {}, r#"{"status":"unknown"}"#);
    strict_empty(
        Section::<MaterialBalance>::Omitted {},
        r#"{"coverage":"omitted"}"#,
    );
    strict_empty(OwnerRef::Unequipped, r#"{"type":"unequipped"}"#);
}

#[test]
fn empty_decision_and_parameter_variants_reject_payload_claims() {
    strict_empty(
        EvaluationScope::WholeSuppliedInventory,
        r#"{"type":"whole_supplied_inventory"}"#,
    );
    strict_empty(ParetoStatus::Frontier, r#"{"status":"frontier"}"#);
    strict_empty(ParetoStatus::Unknown, r#"{"status":"unknown"}"#);
    strict_empty(AssignmentDomain::Unrestricted, r#"{"type":"unrestricted"}"#);
    strict_empty(
        AssignmentDomain::CurrentLoadout,
        r#"{"type":"current_loadout"}"#,
    );
    strict_empty(
        ComparisonMode::UnrestrictedOptimum,
        r#"{"type":"unrestricted_optimum"}"#,
    );
    strict_empty(FrontierPresentation::Exhaustive, r#"{"type":"exhaustive"}"#);
    strict_empty(
        FrontierPresentation::AmongReturned,
        r#"{"type":"among_returned"}"#,
    );
    strict_empty(setup::ParameterDomain::Flag, r#"{"type":"flag"}"#);
}

#[test]
fn empty_transport_variants_reject_payload_claims() {
    strict_empty(DurableContent::Tombstone, r#"{"type":"tombstone"}"#);
    strict_empty(
        CheckpointPolicy::PreserveAccepted,
        r#"{"type":"preserve_accepted"}"#,
    );
    strict_empty(ActionIntent::Evaluate, r#"{"type":"evaluate"}"#);
    strict_empty(ActionIntent::Compare, r#"{"type":"compare"}"#);
    strict_empty(ActionIntent::Sensitivity, r#"{"type":"sensitivity"}"#);
    strict_empty(ActionIntent::Catalog, r#"{"type":"catalog"}"#);
    strict_empty(
        Control::ClearReservationAccepted,
        r#"{"type":"clear_reservation_accepted"}"#,
    );
    strict_empty(Control::ClearAborted, r#"{"type":"clear_aborted"}"#);
    strict_empty(Control::Cancel, r#"{"type":"cancel"}"#);
    strict_empty(Control::Retry, r#"{"type":"retry"}"#);
    strict_empty(Control::ClearLocalData, r#"{"type":"clear_local_data"}"#);
    strict_empty(Control::DiscardSession, r#"{"type":"discard_session"}"#);
    strict_empty(Control::Shutdown, r#"{"type":"shutdown"}"#);
}

#[test]
fn complete_material_assertion_rejects_unrecognized_verification_fields() {
    let valid = r#"{"coverage":"supplied","completeness":{"declared_complete":{"asserted_by":{"type":"user","statement":"Explicit fabricated completeness assertion"}}},"records":[]}"#;
    let section: Section<MaterialBalance> = wire::decode(valid).unwrap();
    let mut a = account();
    a.materials = section;
    assert_eq!(
        ValidatedAccount::imported(a)
            .unwrap()
            .material_balance(&id("missing")),
        MaterialQuantity::Known { amount: Counter(0) }
    );
    for field in ["verified", "audited", "source"] {
        let mut forged: serde_json::Value = serde_json::from_str(valid).unwrap();
        forged["completeness"]["declared_complete"][field] = true.into();
        assert!(wire::decode::<Section<MaterialBalance>>(&forged.to_string()).is_err());
    }
}

#[test]
fn real_request_recovery_and_control_decoders_do_not_discard_supplied_data() {
    let request = request("strict-catalog", Request::GetCatalog, 0);
    let mut forged = serde_json::to_value(&request).unwrap();
    forged["payload"]["spec"] = serde_json::to_value(spec()).unwrap();
    assert!(wire::decode_request(&forged.to_string()).is_err());
    assert_eq!(
        wire::decode_request(&wire::encode(&request).unwrap()).unwrap(),
        request
    );

    let frame = ControlFrame {
        version: ProtocolVersion,
        from: Endpoint::Dom,
        to: Endpoint::UiSupervisor,
        authority: TransportAuthority {
            epochs: context().epochs,
            generation: Counter(0),
            action: id("strict-cancel"),
        },
        step: Counter(0),
        payload: Control::Cancel,
    };
    assert_eq!(
        decode_control(&encode_control(&frame).unwrap()).unwrap(),
        frame
    );
    let mut forged = serde_json::to_value(&frame).unwrap();
    forged["payload"]["receipt"] = serde_json::json!({"revision": "7"});
    assert!(decode_control(&forged.to_string()).is_err());

    let mut a = account();
    a.materials = supplied(vec![material_cases::balance(
        "strict:ore",
        MaterialQuantity::Known {
            amount: Counter(u64::MAX),
        },
    )]);
    let backup = Arc::new(Backup::from_validated(&ValidatedAccount::imported(a).unwrap()).unwrap());
    let mut forged = serde_json::json!({"type": "empty_session"});
    forged["backup"] = serde_json::to_value(&backup).unwrap();
    assert!(wire::decode::<ActiveSelection>(&forged.to_string()).is_err());

    let head = DurableHead::from_record(&DurableRecord {
        revision: Counter(7),
        backup: Some(backup),
    })
    .unwrap();
    assert_eq!(
        wire::decode::<DurableHead>(&wire::encode(&head).unwrap()).unwrap(),
        head
    );
    let mut forged = serde_json::to_value(&head).unwrap();
    forged["content"]["type"] = "tombstone".into();
    assert!(wire::decode::<DurableHead>(&forged.to_string()).is_err());
}
