use aaru_contract::*;
use std::collections::{BTreeMap, BTreeSet};

fn id<T: TryFrom<String>>(value: &str) -> T
where
    T::Error: std::fmt::Debug,
{
    value.to_owned().try_into().unwrap()
}
fn supplied<T>(records: Vec<T>) -> Section<T> {
    Section::Supplied {
        completeness: Completeness::Unspecified,
        records,
    }
}
fn account() -> AccountSnapshot {
    AccountSnapshot {
        schema_version: AccountVersion,
        id: id("account-1"),
        characters: supplied(
            (0..8)
                .map(|n| OwnedCharacter {
                    id: id(&format!("c{n}")),
                    definition: id(&format!("syn:character/{n}")),
                    level: Some(1),
                    ascension: None,
                    constellation: None,
                    talents: None,
                    source: None,
                })
                .collect(),
        ),
        weapons: supplied(vec![weapon("w0"), weapon("w1")]),
        artifacts: Section::Omitted,
        preferences: AllocationPreferences::default(),
        source_archive: None,
        provenance: vec![],
    }
}
fn weapon(name: &str) -> WeaponInstance {
    WeaponInstance {
        id: id(name),
        definition: id("syn:weapon/same"),
        level: Some(1),
        ascension: None,
        refinement: Some(1),
        owner: OwnerRef::Unequipped,
        game_lock: Some(false),
        source: None,
    }
}
fn vector(values: [i64; 9]) -> ScoreVector {
    ScoreVector {
        dimensions: DIMENSIONS
            .into_iter()
            .zip(values)
            .map(|(d, n)| {
                (
                    d,
                    Metric::Available {
                        value: Fixed::from_scaled(n),
                        fidelity: Fidelity::ExactUnderModel,
                        provenance: vec![],
                    },
                )
            })
            .collect(),
    }
}
fn spec() -> DecisionSpec {
    DecisionSpec {
        mechanics: MechanicsRef::Synthetic {
            sample: SampleId::SynWelcome,
            revision: id("syn-v1"),
            sha256: Digest::of(b"fiction"),
        },
        scope: EvaluationScope::WholeSuppliedInventory,
        objective: Objective {
            revision: id("objective-v1"),
            label: "Balanced".into(),
            weights: [(Dimension::OutputPotential, Fixed::ONE)].into(),
        },
        encounters: vec![Encounter {
            revision: id("encounter-v1"),
            definition: id("syn:encounter/one"),
            label: "Sample target".into(),
            target_count: 1,
            parameters: BTreeMap::new(),
            provenance: vec![],
        }],
        assumptions: Assumptions {
            revision: id("assumptions-v1"),
            values: BTreeMap::new(),
        },
        constraints: BuildConstraints {
            required_characters: BTreeSet::new(),
            excluded_characters: BTreeSet::new(),
            excluded_items: BTreeSet::new(),
            pinned_assignments: vec![],
            allowed_sets: BTreeMap::new(),
            allowed_main_stats: BTreeMap::new(),
            metric_minima: BTreeMap::new(),
            required_rules: BTreeSet::new(),
        },
        search: SearchPolicy {
            revision: id("search-v1"),
            max_build_nodes: Counter(100),
            max_team_nodes: Counter(100),
            max_allocation_nodes: Counter(100),
            max_results: 10,
            work_units_per_slice: 10,
        },
    }
}
fn allocation() -> Allocation {
    Allocation {
        id: id("allocation-1"),
        candidate: id("candidate-1"),
        teams: vec![TeamAssignment {
            id: id("team-1"),
            encounter_index: 0,
            archetypes: BTreeSet::new(),
            builds: (0..4)
                .map(|n| BuildAssignment {
                    id: id(&format!("build-{n}")),
                    character: id(&format!("c{n}")),
                    weapon: if n == 0 { Some(id("w0")) } else { None },
                    artifacts: BTreeMap::new(),
                    metrics: BTreeMap::new(),
                    reasons: vec![],
                })
                .collect(),
        }],
        footprint: ResourceFootprint {
            characters: (0..4).map(|n| id(&format!("c{n}"))).collect(),
            items: [id("w0")].into(),
        },
        score_vector: vector([1_000_000; 9]),
        presentation_score: Fixed::ONE,
        pareto: ParetoStatus::Frontier,
        constraints: vec![],
        reasons: vec![],
    }
}
fn result(account: &AccountSnapshot) -> DecisionResult {
    let spec = spec();
    let engine_revision: RevisionId = id("engine-1");
    DecisionResult {
        reproducibility: Reproducibility {
            account_semantics: wire::account_semantic_digest(account).unwrap(),
            mechanics: spec.mechanics.clone(),
            engine_version: "0.1.0".into(),
            engine_revision: engine_revision.clone(),
            decision_spec: wire::decision_spec_digest(&spec).unwrap(),
            fingerprint: wire::result_fingerprint(account, &spec, "0.1.0", &engine_revision)
                .unwrap(),
        },
        scope: spec.scope.clone(),
        spec,
        exclusions: vec![],
        allocations: vec![allocation()],
        search: SearchReport {
            status: SearchStatus::Complete,
            counters: SearchCounters {
                build_nodes: Counter(1),
                team_nodes: Counter(1),
                allocation_nodes: Counter(1),
                pruned_nodes: Counter(0),
            },
            domain: "Normative contract-only domain".into(),
            limits_reached: vec![],
            justified_upper_bound: None,
            bound_provenance: vec![],
        },
        reasons: vec![],
        uncertainty: vec![],
        provenance: vec![],
    }
}

#[test]
fn arithmetic_uses_hand_calculated_signed_half_even_boundaries() {
    for (a, b, expected) in [
        (1, 2, 0),
        (3, 2, 2),
        (5, 2, 2),
        (7, 2, 4),
        (-1, 2, 0),
        (-3, 2, -2),
        (3, -2, -2),
        (-5, -2, 2),
    ] {
        assert_eq!(numeric::round_ratio(a, b), Ok(expected));
    }
    assert_eq!(
        numeric::round_ratio(i128::MIN, -1),
        Err(NumericError::Overflow)
    );
    assert_eq!(numeric::round_ratio(i128::MIN, i128::MIN), Ok(1));
    assert_eq!(numeric::round_ratio(i128::MAX, i128::MIN), Ok(-1));
    assert_eq!(
        Fixed::ONE.checked_div(Fixed::ZERO),
        Err(NumericError::DivisionByZero)
    );
    assert_eq!(
        Fixed::from_scaled(i64::MAX).checked_add(Fixed::ONE),
        Err(NumericError::Overflow)
    );
    assert_eq!(
        Fixed::from_scaled(i64::MIN).checked_mul(Fixed::from_scaled(-1_000_000)),
        Err(NumericError::Overflow)
    );
    assert_eq!(
        Fixed::from_scaled(3).checked_mul(Fixed::from_scaled(500_000)),
        Ok(Fixed::from_scaled(2))
    );
}

#[test]
fn ratio_matches_independent_nearest_candidate_distance_oracle() {
    // Independent oracle: enumerate integers and minimize exact distance; even wins ties.
    for numerator in -50_i128..=50 {
        for denominator in (-11_i128..=11).filter(|d| *d != 0) {
            let expected = (-51_i128..=51)
                .min_by_key(|candidate| {
                    (
                        (candidate * denominator - numerator).abs(),
                        candidate.rem_euclid(2),
                    )
                })
                .unwrap();
            assert_eq!(numeric::round_ratio(numerator, denominator), Ok(expected));
        }
    }
}

#[test]
fn decimal_import_is_exact_and_percentages_do_not_gain_two_orders_of_magnitude() {
    for (text, scaled) in [
        ("3.9", 3_900_000),
        ("-0.000001", -1),
        ("1e-6", 1),
        ("1.0000000", 1_000_000),
        ("9223372036854.775807", i64::MAX),
        ("-9223372036854.775808", i64::MIN),
    ] {
        assert_eq!(Fixed::from_decimal(text), Ok(Fixed::from_scaled(scaled)));
    }
    assert_eq!(
        Fixed::from_percentage_points("3.9"),
        Ok(Fixed::from_scaled(39_000))
    );
    assert_eq!(
        Fixed::from_percentage_points("0.00001"),
        Err(NumericError::Inexact)
    );
    assert_eq!(Fixed::from_decimal("0.0000001"), Err(NumericError::Inexact));
    assert_eq!(
        Fixed::from_decimal("9223372036854.775808"),
        Err(NumericError::Overflow)
    );
    for text in ["NaN", "1.", "+1", "01", "true", "1e", " 1"] {
        assert!(Fixed::from_decimal(text).is_err(), "{text}");
    }
    for text in ["1", "1.0", "true", "\"+1\"", "\"01\"", "\"-0\""] {
        assert!(wire::decode::<Fixed>(text).is_err(), "{text}");
    }
    assert_eq!(
        wire::encode(&Fixed::from_scaled(i64::MAX)).unwrap(),
        "\"9223372036854775807\""
    );
}

#[test]
fn shared_weighting_rounds_once_and_missing_dimensions_are_not_zero() {
    let mut objective = spec().objective;
    objective.weights = [
        (Dimension::OutputPotential, Fixed::from_scaled(500_000)),
        (Dimension::EnergyStability, Fixed::from_scaled(500_000)),
    ]
    .into();
    let mut v = vector([1; 9]);
    // Two half-micro products sum to one micro; per-product rounding would give zero.
    assert_eq!(objective.score(&v).unwrap(), Fixed::from_scaled(1));
    v.dimensions.insert(
        Dimension::EnergyStability,
        Metric::Unavailable {
            issue: IssueCode::UnsupportedMechanics,
        },
    );
    assert_eq!(
        objective.score(&v).unwrap_err().code,
        IssueCode::UnavailableDimension
    );
    objective
        .weights
        .insert(Dimension::EnergyStability, Fixed::ZERO);
    assert_eq!(objective.score(&v).unwrap(), Fixed::ZERO);
    objective
        .weights
        .insert(Dimension::OutputPotential, Fixed::from_scaled(-1));
    assert!(objective.validate().is_err());
    objective.weights = DIMENSIONS
        .into_iter()
        .map(|d| (d, Fixed::from_scaled(i64::MAX)))
        .collect();
    assert_eq!(
        objective.score(&vector([i64::MAX; 9])).unwrap_err().code,
        IssueCode::NumericOverflow
    );
    assert_eq!(
        wire::encode(&Counter(u64::MAX)).unwrap(),
        "\"18446744073709551615\""
    );
    assert!(wire::decode::<Counter>("18446744073709551615").is_err());
}

#[test]
fn pareto_keeps_tradeoffs_ties_and_unknowns_without_epsilon() {
    let a = vector([10; 9]);
    let mut b = vector([10; 9]);
    assert_eq!(a.pareto_against(&b).unwrap(), ParetoRelation::Equal);
    b.dimensions.insert(
        Dimension::EnergyStability,
        Metric::Available {
            value: Fixed::from_scaled(9),
            fidelity: Fidelity::Estimated,
            provenance: vec![],
        },
    );
    assert_eq!(a.pareto_against(&b).unwrap(), ParetoRelation::Dominates);
    b.dimensions.insert(
        Dimension::OutputPotential,
        Metric::Available {
            value: Fixed::from_scaled(11),
            fidelity: Fidelity::ExactUnderModel,
            provenance: vec![],
        },
    );
    assert_eq!(a.pareto_against(&b).unwrap(), ParetoRelation::Incomparable);
    b.dimensions.insert(
        Dimension::OutputPotential,
        Metric::Unavailable {
            issue: IssueCode::UnknownDefinition,
        },
    );
    assert_eq!(a.pareto_against(&b).unwrap(), ParetoRelation::Incomparable);
}

#[test]
fn wire_boundary_rejects_information_loss_and_resource_abuse_before_typed_decode() {
    for text in [
        r#"{"a":1,"a":2}"#,
        r#"{"outer":{"a":1,"\u0061":2}}"#,
        r#"[{"same":0,"same":1}]"#,
    ] {
        assert_eq!(
            wire::decode::<serde_json::Value>(text).unwrap_err().code,
            IssueCode::DuplicateProperty
        );
    }
    for text in ["{}{}", "\u{feff}{}", "[1,]", "NaN"] {
        assert!(wire::decode::<serde_json::Value>(text).is_err());
    }
    let deep = format!("{}0{}", "[".repeat(65), "]".repeat(65));
    assert_eq!(
        wire::decode::<serde_json::Value>(&deep).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    let long = format!("\"{}\"", "x".repeat(4097));
    assert_eq!(
        wire::decode::<String>(&long).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    let mut request = serde_json::to_value(request("r", Request::GetCatalog, 0)).unwrap();
    request["protocol_version"] = 2.into();
    assert_eq!(
        wire::decode_request(&request.to_string()).unwrap_err().code,
        IssueCode::UnsupportedVersion
    );
    assert!(
        wire::decode::<AccountSnapshot>(r#"{"schema_version":1,"id":"x","synthetic":true}"#)
            .is_err()
    );
}

#[test]
fn copies_omissions_external_reservations_and_source_bytes_are_distinct() {
    let mut a = account();
    a.validate().unwrap();
    assert_eq!(a.weapons.records().len(), 2);
    assert!(!a.artifacts.is_supplied());
    let omitted = wire::account_semantic_digest(&a).unwrap();
    a.artifacts = supplied(vec![]);
    assert_ne!(omitted, wire::account_semantic_digest(&a).unwrap());
    if let Section::Supplied { records, .. } = &mut a.weapons {
        records[0].owner = OwnerRef::UnresolvedExternal("MissingOwner".into());
        records[1].owner = OwnerRef::UnresolvedExternal("MissingOwner".into());
    }
    assert_eq!(a.validate().unwrap_err().code, IssueCode::EquipmentConflict);
    if let Section::Supplied { records, .. } = &mut a.weapons {
        records[1].owner = OwnerRef::Unequipped;
    }
    a.validate().unwrap();
    assert_eq!(
        a.validate_move(&ItemMove {
            item: id("w0"),
            recipient: id("c0")
        })
        .unwrap_err()
        .code,
        IssueCode::UnresolvedOwner
    );
    let source = "{\n  \"format\": \"GOOD\", \"value\": 3.9000\n}\n";
    a.source_archive = Some(SourceArchive {
        format: SourceFormat::Good {
            dialect_version: 3,
            profile: ImportProfileVersion,
        },
        utf8: source.into(),
        sha256: Digest::of(source.as_bytes()),
    });
    a.validate().unwrap();
    let text = wire::encode(&a).unwrap();
    let restored = wire::decode_account(&text).unwrap();
    assert_eq!(
        restored
            .snapshot()
            .source_archive
            .as_ref()
            .unwrap()
            .utf8
            .as_bytes(),
        source.as_bytes()
    );
    let mut download = Download {
        format: ExportFormat::PreservedSource,
        filename: "account-source.json".into(),
        media_type: "application/json".into(),
        utf8: source.into(),
        sha256: Digest::of(source.as_bytes()),
    };
    download.validate().unwrap();
    download.utf8.push(' ');
    assert_eq!(
        download.validate().unwrap_err().code,
        IssueCode::InvalidSourceDigest
    );
    download.utf8 = source.into();
    download.filename = "../account.json".into();
    assert_eq!(
        download.validate().unwrap_err().code,
        IssueCode::InvalidInput
    );
    a.source_archive.as_mut().unwrap().utf8.push(' ');
    assert_eq!(
        a.validate().unwrap_err().code,
        IssueCode::InvalidSourceDigest
    );
    assert_eq!(
        String::from(Digest::of(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn imported_game_lock_is_not_an_aaru_policy_and_synthetic_trust_cannot_be_imported() {
    let mut a = account();
    if let Section::Supplied { records, .. } = &mut a.weapons {
        records[0].game_lock = Some(true);
    }
    let movement = ItemMove {
        item: id("w0"),
        recipient: id("c0"),
    };
    assert_eq!(
        a.validate_move(&movement).unwrap_err().code,
        IssueCode::LockedItem
    );
    a.preferences.game_lock_policy = GameLockPolicy::AllowReassignment;
    a.validate_move(&movement).unwrap();
    assert_eq!(a.weapons.records()[0].game_lock, Some(true));
    a.preferences.protected_items.insert(id("w0"));
    assert_eq!(
        a.validate_move(&movement).unwrap_err().code,
        IssueCode::ProtectedItem
    );
    let imported = ValidatedAccount::imported(a.clone()).unwrap();
    assert_eq!(
        spec().validate_for(&imported).unwrap_err().code,
        IssueCode::SyntheticOnImportedAccount
    );
    let sample = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
    spec().validate_for(&sample).unwrap();
    assert!(
        !ValidatedAccount::bundled(a.clone(), SampleId::SynScarcity)
            .unwrap()
            .permits(&spec().mechanics)
    );
    a.source_archive = Some(SourceArchive {
        format: SourceFormat::Good {
            dialect_version: 3,
            profile: ImportProfileVersion,
        },
        utf8: "{}".into(),
        sha256: Digest::of(b"{}"),
    });
    assert_eq!(
        ValidatedAccount::bundled(a, SampleId::SynWelcome)
            .unwrap_err()
            .code,
        IssueCode::SyntheticOnImportedAccount
    );
    // Exporting a sample and importing its backup does not carry the private trust marker.
    let restored = wire::decode_account(&wire::encode(sample.snapshot()).unwrap()).unwrap();
    assert!(!restored.permits(&spec().mechanics));
}

#[test]
fn exact_assignment_footprints_reject_copy_reuse_even_across_teams() {
    let a = account();
    let mut s = spec();
    let valid = allocation();
    valid.validate(&a, &s).unwrap();
    let mut wrong = valid.clone();
    wrong.footprint.items.clear();
    assert!(wrong.validate(&a, &s).is_err());
    wrong = valid.clone();
    wrong.teams[0].builds[1].weapon = Some(id("w0"));
    assert!(wrong.validate(&a, &s).is_err());
    wrong = valid.clone();
    wrong.teams[0].builds[1].weapon = Some(id("w1"));
    wrong.footprint.items.insert(id("w1"));
    wrong.validate(&a, &s).unwrap(); // Identical definitions, independent physical copies.
    let mut second = valid.teams[0].clone();
    second.id = id("team-2");
    second.encounter_index = 1;
    for (n, b) in second.builds.iter_mut().enumerate() {
        b.id = id(&format!("build-x{n}"));
        b.character = id(&format!("c{}", n + 4));
    }
    wrong.teams.push(second);
    wrong
        .footprint
        .characters
        .extend((4..8).map(|n| id(&format!("c{n}"))));
    s.encounters.push(s.encounters[0].clone());
    assert!(wrong.validate(&a, &s).is_err());
    s = spec();
    s.constraints.excluded_characters.insert(id("c0"));
    assert!(valid.validate(&a, &s).is_err());
}

#[test]
fn result_validation_challenges_fingerprints_completeness_and_reason_graphs() {
    let a = account();
    let trusted = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
    let valid = result(&a);
    valid.validate_for(&trusted).unwrap();
    let mut bad = valid.clone();
    bad.search.status = SearchStatus::Bounded;
    assert_eq!(
        bad.validate_for(&trusted).unwrap_err().code,
        IssueCode::InvalidSearchReport
    );
    bad.search.limits_reached.push(SearchLimit::AllocationNodes);
    bad.validate_for(&trusted).unwrap();
    bad = valid.clone();
    bad.reproducibility.fingerprint = Digest::of(b"wrong");
    assert!(bad.validate_for(&trusted).is_err());
    bad = valid.clone();
    bad.allocations[0].reasons.push(id("missing"));
    assert!(bad.validate_for(&trusted).is_err());
    bad = valid;
    bad.reasons.push(CausalReason {
        id: id("cycle"),
        code: ReasonCode::ResourceConflict,
        rule: id("rule"),
        subjects: vec![],
        causes: vec![id("cycle")],
        parameters: BTreeMap::new(),
        provenance: vec![],
    });
    assert!(bad.validate_for(&trusted).is_err());
}

#[test]
fn semantic_hash_is_permutation_stable_but_sensitive_to_coverage_and_policy() {
    let mut a = account();
    let original = wire::account_semantic_digest(&a).unwrap();
    if let Section::Supplied { records, .. } = &mut a.weapons {
        records.reverse();
    }
    if let Section::Supplied { records, .. } = &mut a.characters {
        records.reverse();
    }
    assert_eq!(original, wire::account_semantic_digest(&a).unwrap());
    a.preferences.game_lock_policy = GameLockPolicy::AllowReassignment;
    assert_ne!(original, wire::account_semantic_digest(&a).unwrap());
}

fn request(name: &str, payload: Request, generation: u64) -> WorkerRequest {
    Envelope {
        protocol_version: ProtocolVersion,
        request_id: id(name),
        account_generation: Counter(generation),
        payload,
    }
}
fn event(name: &str, payload: Event, generation: u64) -> WorkerEvent {
    Envelope {
        protocol_version: ProtocolVersion,
        request_id: id(name),
        account_generation: Counter(generation),
        payload,
    }
}
fn error_event() -> Event {
    Event::Error {
        error: ProtocolError {
            code: IssueCode::Cancelled,
            message: "Cancelled".into(),
            issues: vec![],
            recoverable: true,
        },
    }
}
#[test]
fn ui_gate_rejects_stale_objectives_cancelled_requests_unsolicited_transitions_and_old_generations()
{
    let mut gate = SessionGate::default();
    gate.register(&request("old", Request::Evaluate { spec: spec() }, 0))
        .unwrap();
    gate.register(&request("new", Request::Evaluate { spec: spec() }, 0))
        .unwrap();
    assert!(!gate.accept(&event("old", error_event(), 0)));
    assert!(!gate.accept(&event(
        "new",
        Event::Catalog {
            catalog: Catalog {
                schema_version: CatalogVersion,
                samples: vec![],
                mechanics: vec![]
            }
        },
        0
    )));
    gate.register(&request("cancel", Request::Cancel { target: id("new") }, 0))
        .unwrap();
    assert!(!gate.accept(&event("new", error_event(), 0)));
    assert!(!gate.accept(&event(
        "unsolicited",
        Event::ResetAcknowledged {
            previous_generation: Counter(0)
        },
        1
    )));
    gate.register(&request("reset", Request::Reset, 0)).unwrap();
    assert!(
        gate.register(&request("race", Request::Evaluate { spec: spec() }, 0))
            .is_err()
    );
    assert!(
        gate.register(&request(
            "cancel-reset",
            Request::Cancel {
                target: id("reset")
            },
            0
        ))
        .is_err()
    );
    assert!(!gate.accept(&event(
        "reset",
        Event::ResetAcknowledged {
            previous_generation: Counter(0)
        },
        2
    )));
    assert!(gate.accept(&event(
        "reset",
        Event::ResetAcknowledged {
            previous_generation: Counter(0)
        },
        1
    )));
    assert_eq!(gate.generation(), Counter(1));
    assert!(!gate.accept(&event("old", error_event(), 0)));
    assert!(
        gate.register(&request("stale", Request::GetCatalog, 0))
            .is_err()
    );
    gate.register(&request("catalog", Request::GetCatalog, 1))
        .unwrap();
    assert!(!gate.accept(&event("catalog", error_event(), 0)));
    assert!(gate.accept(&event("catalog", error_event(), 1)));
    assert!(!gate.accept(&event("catalog", error_event(), 1)));
    assert_eq!(Counter(u64::MAX).next(), None);
}

#[test]
fn import_prepare_activate_is_atomic_digest_bound_and_single_use() {
    let a = account();
    let preview = ImportPreview {
        token: id("prepared-1"),
        replaces_generation: Counter(0),
        canonical_digest: wire::canonical_backup_digest(&a).unwrap(),
        account: a,
        issues: vec![],
        capability: EvaluationCapability {
            supported: [Operation::ExportCanonical].into(),
            blockers: vec![],
            scope: EvaluationScope::WholeSuppliedInventory,
        },
        sample: None,
    };
    let mut registry = PreparedRegistry::default();
    registry.prepare(&preview, Counter(0)).unwrap();
    assert!(
        registry
            .activate(&id("wrong"), Counter(0), &StorageDisposition::SessionOnly)
            .is_err()
    );
    assert!(
        registry
            .activate(&preview.token, Counter(1), &StorageDisposition::SessionOnly)
            .is_err()
    );
    assert_eq!(
        registry
            .activate(
                &preview.token,
                Counter(0),
                &StorageDisposition::Persisted {
                    account_digest: Digest::of(b"wrong")
                }
            )
            .unwrap_err()
            .code,
        IssueCode::StorageFailure
    );
    assert_eq!(
        registry
            .activate(
                &preview.token,
                Counter(0),
                &StorageDisposition::Persisted {
                    account_digest: preview.canonical_digest.clone()
                }
            )
            .unwrap(),
        Counter(1)
    );
    assert!(
        registry
            .activate(&preview.token, Counter(0), &StorageDisposition::SessionOnly)
            .is_err()
    );
    registry.prepare(&preview, Counter(0)).unwrap();
    registry.clear();
    assert!(
        registry
            .activate(&preview.token, Counter(0), &StorageDisposition::SessionOnly)
            .is_err()
    );
}

#[test]
fn counterfactual_worlds_account_for_donor_replacements_and_cannot_claim_bounded_gains() {
    let mut before_account = account();
    if let Section::Supplied { records, .. } = &mut before_account.weapons {
        records[0].owner = OwnerRef::Character(id("c1"));
    }
    let trusted = ValidatedAccount::bundled(before_account.clone(), SampleId::SynWelcome).unwrap();
    let mut after_account = before_account.clone();
    if let Section::Supplied { records, .. } = &mut after_account.weapons {
        records[0].owner = OwnerRef::Character(id("c0"));
        records[1].owner = OwnerRef::Character(id("c1"));
    }
    let change = |character: &str| CharacterChange {
        character: id(character),
        before_builds: vec![],
        after_builds: vec![],
        metric_deltas: BTreeMap::new(),
        reasons: vec![],
    };
    let mut after = result(&after_account);
    after.allocations[0].score_vector = vector([500_000; 9]);
    after.allocations[0].presentation_score = Fixed::from_scaled(500_000);
    let mut comparison = ComparisonResult {
        movement: ItemMove {
            item: id("w0"),
            recipient: id("c0"),
        },
        original_owner: OwnerRef::Character(id("c1")),
        before_ownership: ownership(&before_account),
        after_ownership: ownership(&after_account),
        before: result(&before_account),
        after,
        recipient: change("c0"),
        donor: Some(change("c1")),
        other_changes: vec![],
        global_delta: Metric::Available {
            value: Fixed::from_scaled(-500_000),
            fidelity: Fidelity::ExactUnderModel,
            provenance: vec![],
        },
        claim: ImprovementClaim::ProvenWithinDeclaredDomain,
        reasons: vec![],
    };
    comparison.validate_for(&trusted).unwrap();
    let serialized = wire::encode(&comparison).unwrap();
    wire::decode::<ComparisonResult>(&serialized)
        .unwrap()
        .validate_for(&trusted)
        .unwrap();
    let mut bad = comparison.clone();
    bad.donor = None;
    assert!(bad.validate_for(&trusted).is_err());
    bad = comparison.clone();
    bad.after_ownership.remove(&id("w1"));
    assert!(bad.validate_for(&trusted).is_err());
    bad = comparison.clone();
    bad.global_delta = Metric::Available {
        value: Fixed::ONE,
        fidelity: Fidelity::Estimated,
        provenance: vec![],
    };
    assert!(bad.validate_for(&trusted).is_err());
    comparison.after.search.status = SearchStatus::Bounded;
    comparison
        .after
        .search
        .limits_reached
        .push(SearchLimit::AllocationNodes);
    assert!(comparison.validate_for(&trusted).is_err());
    comparison.claim = ImprovementClaim::BestFoundDifference;
    comparison.validate_for(&trusted).unwrap();
}

#[test]
fn sensitivity_summaries_are_bound_to_explicit_scenario_reruns() {
    let a = account();
    let trusted = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
    let baseline = result(&a);
    let scenario = SensitivityScenario {
        id: id("scenario-1"),
        label: "Unchanged control".into(),
        assumptions: baseline.spec.assumptions.clone(),
        encounters: baseline.spec.encounters.clone(),
    };
    let mut sensitivity = SensitivityResult {
        baseline: baseline.clone(),
        scenarios: vec![ScenarioResult {
            scenario,
            result: baseline,
            ranking_changed: false,
            global_delta: Metric::Available {
                value: Fixed::ZERO,
                fidelity: Fidelity::ExactUnderModel,
                provenance: vec![],
            },
        }],
    };
    sensitivity.validate_for(&trusted).unwrap();
    sensitivity.scenarios[0].ranking_changed = true;
    assert!(sensitivity.validate_for(&trusted).is_err());
    sensitivity.scenarios[0].ranking_changed = false;
    sensitivity.scenarios[0].result.spec.objective.label = "Silently changed objective".into();
    assert!(sensitivity.validate_for(&trusted).is_err());
}

#[test]
fn documented_messages_are_exactly_the_frozen_wire_types() {
    let docs = include_str!("../../../.agent-docs/CONTRACTS.md");
    let mut requests = 0;
    let mut events = 0;
    let mut accounts = 0;
    for block in docs.split("```json\n").skip(1) {
        let text = block.split("\n```").next().unwrap();
        if text.contains("\"schema_version\"") && !text.contains("\"protocol_version\"") {
            wire::decode_account(text).unwrap();
            accounts += 1;
        } else if wire::decode_request(text).is_ok() {
            requests += 1;
        } else {
            wire::decode::<WorkerEvent>(text).unwrap();
            events += 1;
        }
    }
    assert!(requests >= 9 && events >= 2 && accounts >= 1);
}
