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
                    availability: CharacterAvailability::Resolved {
                        owned: id(&format!("owned-{n}")),
                        identity: id(&format!("syn:identity/{n}")),
                    },
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
        assignments: AssignmentDomain::Unrestricted,
        normalization: Normalization {
            revision: id("normalization-1"),
            dimensions: DIMENSIONS
                .into_iter()
                .map(|d| {
                    (
                        d,
                        MetricRange {
                            unit: QuantityUnit::Comparative,
                            direction: Direction::HigherIsBetter,
                            low: Fixed::ZERO,
                            high: Fixed::ONE,
                        },
                    )
                })
                .collect(),
        },
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
            score_vector: vector([1_000_000; 9]),
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
            availability: (0..4).map(|n| id(&format!("owned-{n}"))).collect(),
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
            frontier: FrontierPresentation::Exhaustive,
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
        objective.score(&vector([1_000_000; 9])).unwrap_err().code,
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
    let restored = ValidatedAccount::imported(wire::decode(&text).unwrap()).unwrap();
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
    let restored = ValidatedAccount::imported(
        wire::decode(&wire::encode(sample.snapshot()).unwrap()).unwrap(),
    )
    .unwrap();
    assert!(!restored.permits(&spec().mechanics));
}

#[test]
fn exact_assignment_footprints_and_local_copy_reuse_are_validated() {
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
    wrong.validate(&a, &s).unwrap();
    s.constraints.excluded_characters.insert(id("c0"));
    assert!(valid.validate(&a, &s).is_err());
}

#[test]
fn cross_team_weapon_reuse_changes_only_the_physical_copy() {
    let a = account();
    let mut s = spec();
    let valid = allocation();
    let mut second = valid.teams[0].clone();
    second.id = id("team-2");
    second.encounter_index = 1;
    for (n, b) in second.builds.iter_mut().enumerate() {
        b.id = id(&format!("build-x{n}"));
        b.character = id(&format!("c{}", n + 4));
    }
    let mut pair = valid.clone();
    second.builds[0].weapon = Some(id("w1"));
    pair.teams.push(second);
    pair.footprint
        .characters
        .extend((4..8).map(|n| id(&format!("c{n}"))));
    pair.footprint
        .availability
        .extend((4..8).map(|n| id(&format!("owned-{n}"))));
    pair.footprint.items.insert(id("w1"));
    s.encounters.push(s.encounters[0].clone());
    pair.validate(&a, &s).unwrap();
    pair.teams[1].builds[0].weapon = Some(id("w0"));
    pair.footprint.items.remove(&id("w1"));
    assert_eq!(
        pair.validate(&a, &s).unwrap_err().message,
        "unknown/repeated weapon copy"
    );
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
    bad.search.frontier = FrontierPresentation::AmongReturned;
    bad.search.counters.allocation_nodes = Counter(100);
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

fn context() -> RequestContext {
    RequestContext {
        epochs: Epochs {
            session: id("session-1"),
            worker: Counter(0),
            reset: Counter(0),
        },
        account_semantics: Some(wire::account_semantic_digest(&account()).unwrap()),
        engine: EngineIdentity {
            version: "0.1.0".into(),
            revision: id("engine-1"),
        },
    }
}
fn request(name: &str, payload: Request, generation: u64) -> WorkerRequest {
    request_in(name, payload, Counter(generation), context())
}
fn request_in(
    name: &str,
    payload: Request,
    generation: Counter,
    context: RequestContext,
) -> WorkerRequest {
    Envelope {
        protocol_version: ProtocolVersion,
        fingerprint: wire::operation_fingerprint(&context, generation, &payload).unwrap(),
        context,
        request_id: id(name),
        account_generation: generation,
        payload,
    }
}
fn reply(request: &WorkerRequest, payload: Event, generation: Counter) -> WorkerEvent {
    Envelope {
        protocol_version: ProtocolVersion,
        context: request.context.clone(),
        fingerprint: request.fingerprint.clone(),
        request_id: request.request_id.clone(),
        account_generation: generation,
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
fn ui_authority_rejects_reversed_completion_wrong_fingerprints_cancellation_and_restart() {
    let mut gate = SessionGate::new(context(), Counter(0));
    let old = request("old", Request::Evaluate { spec: spec() }, 0);
    gate.register(&old).unwrap();
    let mut changed = spec();
    changed.objective.weights = [(Dimension::EnergyStability, Fixed::ONE)].into();
    let new = request("new", Request::Evaluate { spec: changed }, 0);
    gate.register(&new).unwrap();
    let old_result = Event::Evaluated {
        result: Box::new(result(&account())),
    };
    assert!(!gate.accept(&reply(&old, old_result.clone(), Counter(0))));
    assert!(!gate.accept(&reply(&new, old_result, Counter(0))));
    let mut forged = reply(&new, error_event(), Counter(0));
    forged.fingerprint = old.fingerprint.clone();
    assert!(!gate.accept(&forged));
    let mut late = result(&account());
    if let Request::Evaluate { spec } = &new.payload {
        late.spec = spec.clone();
    }
    rebind(&mut late, &account());
    let cancel = request("cancel", Request::Cancel { target: id("new") }, 0);
    gate.register(&cancel).unwrap();
    assert!(!gate.accept(&reply(
        &new,
        Event::Evaluated {
            result: Box::new(late)
        },
        Counter(0)
    )));
    assert!(gate.accept(&reply(
        &cancel,
        Event::CancelAcknowledged {
            target: id("new"),
            was_running: true
        },
        Counter(0)
    )));
    let catalog = request("reuse", Request::GetCatalog, 0);
    gate.register(&catalog).unwrap();
    let machine = CommitMachine::new(
        context().epochs,
        true,
        DurableRecord {
            revision: Counter(0),
            backup: None,
        },
    )
    .unwrap();
    gate.restart_worker(&machine).unwrap();
    assert!(!gate.accept(&reply(&catalog, error_event(), Counter(0))));
    let recover = request_in(
        "reuse",
        Request::Recover {
            state: machine.recovery_state(),
        },
        Counter(0),
        gate.context().clone(),
    );
    gate.register(&recover).unwrap();
    assert!(!gate.accept(&reply(&catalog, error_event(), Counter(0))));
    assert!(gate.accept(&reply(
        &recover,
        Event::ResetAcknowledged {
            previous_generation: Counter(0)
        },
        Counter(1)
    )));
    let catalog = request_in(
        "catalog",
        Request::GetCatalog,
        Counter(1),
        gate.context().clone(),
    );
    gate.register(&catalog).unwrap();
    assert!(!gate.accept(&reply(&catalog, error_event(), Counter(0))));
    assert!(gate.accept(&reply(&catalog, error_event(), Counter(1))));
    assert!(!gate.accept(&reply(&catalog, error_event(), Counter(1))));
    assert_eq!(Counter(u64::MAX).next(), None);
}

fn preview(validated: &ValidatedAccount) -> ImportPreview {
    let backup = Backup::from_validated(validated).unwrap();
    ImportPreview {
        token: id("prepared-1"),
        replaces_generation: Counter(0),
        context: context(),
        canonical_digest: backup.digest().unwrap(),
        backup,
        issues: vec![],
        capability: EvaluationCapability {
            supported: [Operation::ExportCanonical].into(),
            blockers: vec![],
            scope: EvaluationScope::WholeSuppliedInventory,
        },
    }
}
fn receipt(record: &DurableRecord, epochs: &Epochs) -> CommitReceipt {
    CommitReceipt {
        epochs: epochs.clone(),
        revision: record.revision,
        record_digest: record.digest().unwrap(),
        backup_digest: record
            .backup
            .as_ref()
            .map_or_else(|| Digest::of(b"aaru-reset-v1"), |b| b.digest().unwrap()),
    }
}
#[test]
fn prepared_activation_binds_source_preferences_epochs_and_is_idempotent() {
    let trusted = ValidatedAccount::bundled(account(), SampleId::SynWelcome).unwrap();
    let p = preview(&trusted);
    let c = context();
    let mut registry = PreparedRegistry::default();
    registry
        .prepare(&p, Counter(0), &c, trusted.clone())
        .unwrap();
    assert!(
        registry
            .activate(
                &id("wrong"),
                Counter(0),
                &c,
                &StorageDisposition::SessionOnly
            )
            .is_err()
    );
    assert!(
        registry
            .activate(&p.token, Counter(1), &c, &StorageDisposition::SessionOnly)
            .is_err()
    );
    let mut record = DurableRecord {
        revision: Counter(1),
        backup: Some(p.backup.clone()),
    };
    record
        .backup
        .as_mut()
        .unwrap()
        .account
        .preferences
        .game_lock_policy = GameLockPolicy::AllowReassignment;
    let bad = StorageDisposition::Persisted {
        receipt: receipt(&record, &c.epochs),
    };
    assert_eq!(
        registry
            .activate(&p.token, Counter(0), &c, &bad)
            .unwrap_err()
            .code,
        IssueCode::StorageFailure
    );
    let good = StorageDisposition::Persisted {
        receipt: receipt(
            &DurableRecord {
                revision: Counter(1),
                backup: Some(p.backup.clone()),
            },
            &c.epochs,
        ),
    };
    assert_eq!(
        registry.activate(&p.token, Counter(0), &c, &good).unwrap(),
        Counter(1)
    );
    assert_eq!(
        registry.activate(&p.token, Counter(1), &c, &good).unwrap(),
        Counter(1)
    );
    assert_eq!(registry.active().unwrap().snapshot(), trusted.snapshot());
    assert!(
        registry
            .activate(&p.token, Counter(1), &c, &StorageDisposition::SessionOnly)
            .is_err()
    );
    assert!(registry.prepare(&p, Counter(0), &c, trusted).is_err());
    registry.clear();
    assert!(registry.activate(&p.token, Counter(1), &c, &good).is_err());
}

fn rebind(result: &mut DecisionResult, account: &AccountSnapshot) {
    result.reproducibility.account_semantics = wire::account_semantic_digest(account).unwrap();
    result.reproducibility.decision_spec = wire::decision_spec_digest(&result.spec).unwrap();
    result.reproducibility.fingerprint = wire::result_fingerprint(
        account,
        &result.spec,
        &result.reproducibility.engine_version,
        &result.reproducibility.engine_revision,
    )
    .unwrap();
}
fn comparison_case(current: bool) -> (ValidatedAccount, ComparisonResult) {
    let mut a = account();
    if let Section::Supplied { records, .. } = &mut a.weapons {
        records[0].owner = OwnerRef::Character(id("c1"));
    }
    let mut before = result(&a);
    before.allocations[0].teams[0].builds[0].weapon = None;
    before.allocations[0].teams[0].builds[1].weapon = Some(id("w0"));
    if current {
        before.spec.assignments = AssignmentDomain::CurrentLoadout;
    }
    rebind(&mut before, &a);
    let comparison = ComparisonSpec {
        baseline: before.spec.clone(),
        movement: ItemMove {
            item: id("w0"),
            recipient: id("c0"),
        },
        mode: if current {
            ComparisonMode::CurrentLoadout {
                allocation: Box::new(before.allocations[0].clone()),
                donor_fallback: Some(id("w1")),
            }
        } else {
            ComparisonMode::UnrestrictedOptimum
        },
    };
    let mut after = result(&a);
    after.spec = comparison.intervention_spec();
    after.allocations[0].teams[0].builds[1].weapon = Some(id("w1"));
    after.allocations[0].footprint.items.insert(id("w1"));
    after.allocations[0].teams[0].score_vector = vector([500_000; 9]);
    after.allocations[0].score_vector = vector([500_000; 9]);
    after.allocations[0].presentation_score = Fixed::from_scaled(500_000);
    rebind(&mut after, &a);
    let change = |n| CharacterChange {
        character: id(&format!("c{n}")),
        before_builds: vec![id(&format!("build-{n}"))],
        after_builds: vec![id(&format!("build-{n}"))],
        metric_deltas: BTreeMap::new(),
        reasons: vec![],
    };
    let result = ComparisonResult {
        comparison,
        original_owner: OwnerRef::Character(id("c1")),
        before,
        after,
        recipient: change(0),
        donor: Some(DonorChange {
            availability: id("owned-1"),
            forms: vec![change(1)],
        }),
        other_changes: vec![change(2), change(3)],
        global_delta: Metric::Available {
            value: Fixed::from_scaled(-500_000),
            fidelity: Fidelity::ExactUnderModel,
            provenance: vec![],
        },
        claim: ComparisonClaim::ExactDifference,
        reasons: vec![],
    };
    (
        ValidatedAccount::bundled(a, SampleId::SynWelcome).unwrap(),
        result,
    )
}
#[test]
fn current_loadout_move_has_exact_recipient_donor_fallback_and_no_unrelated_changes() {
    let (trusted, comparison) = comparison_case(true);
    comparison.validate_for(&trusted).unwrap();
    wire::decode::<ComparisonResult>(&wire::encode(&comparison).unwrap())
        .unwrap()
        .validate_for(&trusted)
        .unwrap();
    let mut bad = comparison.clone();
    bad.donor = None;
    assert!(bad.validate_for(&trusted).is_err());
    bad = comparison.clone();
    bad.after.allocations[0].teams[0].builds[1].weapon = None;
    bad.after.allocations[0].footprint.items.remove(&id("w1"));
    assert!(bad.validate_for(&trusted).is_err());
    bad = comparison.clone();
    bad.after.allocations[0].teams[0].builds.swap(2, 3);
    assert!(bad.validate_for(&trusted).is_err());
    bad = comparison.clone();
    bad.after.search.status = SearchStatus::Bounded;
    bad.after.search.frontier = FrontierPresentation::AmongReturned;
    bad.after
        .search
        .limits_reached
        .push(SearchLimit::AllocationNodes);
    assert!(bad.validate_for(&trusted).is_err());
    bad.claim = ComparisonClaim::BestFoundDifference;
    bad.after.search.counters.allocation_nodes = Counter(100);
    bad.validate_for(&trusted).unwrap();
}
#[test]
fn complete_unrestricted_optimum_cannot_gain_from_assignment_restrictions() {
    let (trusted, mut comparison) = comparison_case(false);
    comparison.validate_for(&trusted).unwrap();
    let mut accepts_scores = |before: i64, after: i64| {
        let score = |n| vector([n * 100_000; 9]);
        comparison.before.allocations[0].teams[0].score_vector = score(before);
        comparison.before.allocations[0].score_vector = score(before);
        comparison.before.allocations[0].presentation_score = Fixed::from_scaled(before * 100_000);
        comparison.after.allocations[0].teams[0].score_vector = score(after);
        comparison.after.allocations[0].score_vector = score(after);
        comparison.after.allocations[0].presentation_score = Fixed::from_scaled(after * 100_000);
        comparison.global_delta = Metric::Available {
            value: Fixed::from_scaled((after - before) * 100_000),
            fidelity: Fidelity::ExactUnderModel,
            provenance: vec![],
        };
        comparison.validate_for(&trusted).is_ok()
    };
    for before in 0..=8 {
        for after in 0..=8 {
            assert_eq!(accepts_scores(before, after), after <= before);
        }
    }
    let all = [(false, 2), (true, 7), (false, 9), (true, 4)];
    for mask in 1..16 {
        let unrestricted = all
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, (_, value))| value)
            .max()
            .unwrap();
        if let Some(restricted) = all
            .iter()
            .enumerate()
            .filter(|(i, (reserved, _))| mask & (1 << i) != 0 && *reserved)
            .map(|(_, (_, value))| value)
            .max()
        {
            assert!(accepts_scores(*unrestricted, *restricted));
        }
    }
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
fn availability_identity_blocks_duplicate_characters_forms_and_unknown_resources() {
    let mut a = account();
    if let Section::Supplied { records, .. } = &mut a.characters {
        records[1].definition = records[0].definition.clone();
    }
    assert_eq!(a.validate().unwrap_err().code, IssueCode::DuplicateIdentity);
    a = account();
    if let Section::Supplied { records, .. } = &mut a.characters {
        records[1].availability = CharacterAvailability::Resolved {
            owned: id("other-copy"),
            identity: id("syn:identity/0"),
        };
    }
    assert_eq!(a.validate().unwrap_err().code, IssueCode::DuplicateIdentity);
    a = account();
    if let Section::Supplied { records, .. } = &mut a.characters {
        records[4].availability = records[0].availability.clone();
    }
    a.validate().unwrap();
    let mut gear = a.clone();
    if let Section::Supplied { records, .. } = &mut gear.weapons {
        records[0].owner = OwnerRef::Character(id("c0"));
        records[0].game_lock = Some(true);
    }
    gear.preferences.pinned.push(PinnedAssignment {
        item: id("w0"),
        owner: OwnerRef::Character(id("c0")),
    });
    let mut alternate_form = allocation();
    alternate_form.teams[0].builds[0].character = id("c4");
    alternate_form.footprint.characters.remove(&id("c0"));
    alternate_form.footprint.characters.insert(id("c4"));
    let mut current = spec();
    current.assignments = AssignmentDomain::CurrentLoadout;
    alternate_form.validate(&gear, &current).unwrap();
    let mut s = spec();
    s.encounters.push(s.encounters[0].clone());
    let mut pair = allocation();
    let mut second = pair.teams[0].clone();
    second.id = id("team-2");
    second.encounter_index = 1;
    second.builds[0].weapon = None;
    for (n, b) in second.builds.iter_mut().enumerate() {
        b.character = id(&format!("c{}", n + 4));
        b.id = id(&format!("second-{n}"));
    }
    pair.teams.push(second);
    pair.footprint
        .characters
        .extend((4..8).map(|n| id(&format!("c{n}"))));
    pair.footprint
        .availability
        .extend((5..8).map(|n| id(&format!("owned-{n}"))));
    assert!(pair.validate(&a, &s).is_err());
    if let Section::Supplied { records, .. } = &mut a.weapons {
        records[0].owner = OwnerRef::Character(id("c0"));
        records[1].owner = OwnerRef::Character(id("c4"));
    }
    assert_eq!(a.validate().unwrap_err().code, IssueCode::EquipmentConflict);
    a = account();
    if let Section::Supplied { records, .. } = &mut a.characters {
        records[7].availability = CharacterAvailability::Unknown;
    }
    let trusted = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
    assert_eq!(
        spec().validate_for(&trusted).unwrap_err().code,
        IssueCode::UnknownAvailability
    );
    s = spec();
    s.scope = EvaluationScope::Subset {
        characters: (0..4).map(|n| id(&format!("c{n}"))).collect(),
        items: [id("w0")].into(),
        exclusions: vec![],
    };
    s.validate_for(&trusted).unwrap();
    allocation().validate(&a, &s).unwrap();
    let mut bad = allocation();
    bad.footprint.availability.clear();
    assert!(bad.validate(&a, &s).is_err());
}

#[test]
fn normalization_aggregation_and_frontier_claims_have_distinct_boundaries() {
    let mut range = MetricRange {
        unit: QuantityUnit::Seconds,
        direction: Direction::HigherIsBetter,
        low: Fixed::from_scaled(-10),
        high: Fixed::from_scaled(30),
    };
    assert_eq!(
        range.normalize(Fixed::ZERO).unwrap(),
        Fixed::from_scaled(250_000)
    );
    range.direction = Direction::LowerIsBetter;
    assert_eq!(
        range.normalize(Fixed::ZERO).unwrap(),
        Fixed::from_scaled(750_000)
    );
    assert!(range.normalize(Fixed::from_scaled(31)).is_err());
    let mean = ScoreVector::aggregate(&[vector([0; 9]), vector([1; 9])]).unwrap();
    assert_eq!(mean, vector([0; 9]));
    assert_eq!(
        ScoreVector::aggregate(&[vector([1; 9]), vector([2; 9])]).unwrap(),
        vector([2; 9])
    );
    let mut missing = vector([2; 9]);
    missing.dimensions.insert(
        Dimension::EnergyStability,
        Metric::Unavailable {
            issue: IssueCode::UnsupportedMechanics,
        },
    );
    let mean = ScoreVector::aggregate(&[missing.clone(), vector([0; 9])]).unwrap();
    assert!(matches!(
        mean.dimensions[&Dimension::EnergyStability],
        Metric::Unavailable { .. }
    ));
    assert_eq!(
        missing.pareto_against(&missing).unwrap(),
        ParetoRelation::Incomparable
    );
    let trusted = ValidatedAccount::bundled(account(), SampleId::SynWelcome).unwrap();
    let mut r = result(trusted.snapshot());
    r.search.frontier = FrontierPresentation::Truncated {
        omitted: Counter(3),
    };
    r.validate_for(&trusted).unwrap();
    r.search.status = SearchStatus::Bounded;
    r.search.limits_reached.push(SearchLimit::AllocationNodes);
    assert!(r.validate_for(&trusted).is_err());
    r.search.frontier = FrontierPresentation::AmongReturned;
    r.search.counters.allocation_nodes = Counter(100);
    r.validate_for(&trusted).unwrap();
    r.allocations[0].score_vector = vector([1_000_001; 9]);
    assert!(r.validate_for(&trusted).is_err());
}

#[test]
fn durable_failure_and_recovery_follow_the_actual_commit_point_not_worker_activation() {
    let c = context();
    let old = Backup::from_validated(&ValidatedAccount::imported(account()).unwrap()).unwrap();
    let mut new = old.clone();
    new.account.preferences.game_lock_policy = GameLockPolicy::AllowReassignment;
    let durable = DurableRecord {
        revision: Counter(1),
        backup: Some(old.clone()),
    };
    let mut m = CommitMachine::new(c.epochs.clone(), true, durable.clone()).unwrap();
    m.prepare(Some(new.clone()), &c.epochs).unwrap();
    m.begin_save(&c.epochs).unwrap();
    assert!(m.prepare(None, &c.epochs).is_err());
    let mut restarted = c.epochs.clone();
    restarted.worker = Counter(1);
    assert!(m.invalidate_prepared(restarted.clone()).is_err());
    m.storage_failed(&c.epochs).unwrap();
    assert_eq!(m.active(), Some(&old));
    assert_eq!(m.durable(), Some(&durable));
    let candidate = m.begin_save(&c.epochs).unwrap().clone();
    let mut wrong = receipt(&candidate, &c.epochs);
    wrong.record_digest = Digest::of(b"partial");
    assert!(m.storage_committed(&wrong).is_err());
    m.storage_committed(&receipt(&candidate, &c.epochs))
        .unwrap();
    assert_eq!(m.active(), Some(&old));
    assert_eq!(m.durable(), Some(&candidate));
    m.recover(
        restarted.clone(),
        RecoveryState::Durable {
            record: Box::new(candidate.clone()),
        },
    )
    .unwrap();
    assert_eq!(m.active(), Some(&new));
    assert!(
        m.storage_committed(&receipt(&candidate, &c.epochs))
            .is_err()
    );
    m.prepare(None, &restarted).unwrap();
    let tombstone = m.begin_save(&restarted).unwrap().clone();
    m.storage_committed(&receipt(&tombstone, &restarted))
        .unwrap();
    m.activate_committed().unwrap();
    assert_eq!(m.active(), None);
    assert!(CommitMachine::new(restarted.clone(), false, durable.clone()).is_err());
    let mut lost_callback = CommitMachine::new(c.epochs.clone(), true, durable.clone()).unwrap();
    lost_callback.prepare(Some(new.clone()), &c.epochs).unwrap();
    lost_callback.begin_save(&c.epochs).unwrap();
    lost_callback
        .settle_storage(&c.epochs, candidate.clone())
        .unwrap();
    lost_callback
        .recover(
            restarted.clone(),
            RecoveryState::Durable {
                record: Box::new(candidate),
            },
        )
        .unwrap();
    assert_eq!(lost_callback.active(), Some(&new));
    let mut aborted = CommitMachine::new(c.epochs.clone(), true, durable.clone()).unwrap();
    aborted.prepare(Some(new.clone()), &c.epochs).unwrap();
    aborted.begin_save(&c.epochs).unwrap();
    aborted.settle_storage(&c.epochs, durable.clone()).unwrap();
    aborted
        .recover(
            restarted.clone(),
            RecoveryState::Durable {
                record: Box::new(durable.clone()),
            },
        )
        .unwrap();
    assert_eq!(aborted.active(), Some(&old));
    aborted.prepare(Some(new.clone()), &restarted).unwrap();
    aborted.activate_session_only().unwrap();
    assert_eq!(aborted.active(), Some(&new));
    assert_eq!(aborted.durable(), Some(&durable));
    aborted
        .recover(
            restarted,
            RecoveryState::Durable {
                record: Box::new(durable),
            },
        )
        .unwrap();
    assert_eq!(aborted.active(), Some(&old));
}

#[test]
fn sample_restoration_rechecks_bundled_inventory_while_preferences_preserve_trust() {
    let sample = ValidatedAccount::bundled(account(), SampleId::SynWelcome).unwrap();
    let preferences = AllocationPreferences {
        game_lock_policy: GameLockPolicy::AllowReassignment,
        ..AllocationPreferences::default()
    };
    let changed = sample.with_preferences(preferences).unwrap();
    let b = Backup::from_validated(&changed).unwrap();
    assert!(b.restore(None).is_err());
    let imported = ValidatedAccount::imported(account()).unwrap();
    assert!(b.restore(Some(&imported)).is_err());
    assert!(b.restore(Some(&sample)).unwrap().permits(&spec().mechanics));
    let mut tampered = b.clone();
    tampered.account.id = id("different-account");
    assert!(tampered.restore(Some(&sample)).is_err());
    tampered = b;
    tampered.sample = None;
    assert!(!tampered.restore(None).unwrap().permits(&spec().mechanics));
}

#[test]
fn reset_during_import_invalidates_previews_and_storage_authority_before_acknowledgement() {
    let trusted = ValidatedAccount::imported(account()).unwrap();
    let p = preview(&trusted);
    let mut gate = SessionGate::new(context(), Counter(0));
    let import = request(
        "import",
        Request::PrepareImport {
            input: ImportInput::Sample {
                id: SampleId::SynWelcome,
            },
        },
        0,
    );
    gate.register(&import).unwrap();
    let response = reply(
        &import,
        Event::ImportPrepared {
            preview: Box::new(p.clone()),
        },
        Counter(0),
    );
    let mut commits = CommitMachine::new(
        context().epochs,
        true,
        DurableRecord {
            revision: Counter(0),
            backup: None,
        },
    )
    .unwrap();
    commits
        .prepare(Some(p.backup.clone()), &context().epochs)
        .unwrap();
    gate.reset_barrier().unwrap();
    commits
        .invalidate_prepared(gate.context().epochs.clone())
        .unwrap();
    assert!(!gate.accept(&response));
    assert!(!gate.preview_authoritative(&p.token, &p.context));
    assert!(commits.begin_save(&p.context.epochs).is_err());
    let tombstone = gate.begin_reset_save(&mut commits).unwrap();
    assert!(commits.activate_session_only().is_err());
    let committed = receipt(&tombstone, &gate.context().epochs);
    gate.storage_committed(&mut commits, &committed).unwrap();
    let mut bad_receipt = committed.clone();
    bad_receipt.backup_digest = p.canonical_digest.clone();
    assert!(
        request_in(
            "bad-reset",
            Request::Reset {
                receipt: bad_receipt
            },
            Counter(0),
            gate.context().clone()
        )
        .validate()
        .is_err()
    );
    let reset = request_in(
        "reset",
        Request::Reset { receipt: committed },
        Counter(0),
        gate.context().clone(),
    );
    gate.register(&reset).unwrap();
    assert!(
        gate.register(&request_in(
            "race",
            Request::GetCatalog,
            Counter(0),
            gate.context().clone()
        ))
        .is_err()
    );
    assert!(!gate.accept(&reply(
        &reset,
        Event::ResetAcknowledged {
            previous_generation: Counter(0)
        },
        Counter(2)
    )));
    assert!(gate.accept(&reply(
        &reset,
        Event::ResetAcknowledged {
            previous_generation: Counter(0)
        },
        Counter(1)
    )));
    assert!(!gate.accept(&response));
}

#[test]
fn numeric_and_slice_work_budgets_reject_extreme_exponents_and_force_yield() {
    for text in ["0e2147483647", "1e-2147483648", "0e129", "1e-129"] {
        assert!(Fixed::from_decimal(text).is_err());
        assert_eq!(
            wire::decode::<serde_json::Value>(text).unwrap_err().code,
            IssueCode::ResourceLimit
        );
    }
    assert!(Fixed::from_decimal("1e+-2").is_err());
    let mut budget = work::SliceBudget::new(2).unwrap();
    assert!(budget.admit(0, false));
    assert!(budget.admit(100, false));
    assert!(!budget.admit(200, false));
    let mut budget = work::SliceBudget::new(2).unwrap();
    assert!(!budget.admit(work::MAX_SLICE_MICROSECONDS, false));
    assert!(!budget.admit(0, true));
    assert!(work::SliceBudget::new(MAX_WORK_UNITS_PER_SLICE + 1).is_err());
}

#[test]
fn saving_reserves_foreground_until_commit_and_matching_activation() {
    let trusted = ValidatedAccount::imported(account()).unwrap();
    let p = preview(&trusted);
    let mut gate = SessionGate::new(context(), Counter(0));
    let import = request(
        "import",
        Request::PrepareImport {
            input: ImportInput::Restore {
                backup: Box::new(p.backup.clone()),
            },
        },
        0,
    );
    gate.register(&import).unwrap();
    assert!(gate.accept(&reply(
        &import,
        Event::ImportPrepared {
            preview: Box::new(p.clone())
        },
        Counter(0)
    )));
    let mut machine = CommitMachine::new(
        context().epochs,
        true,
        DurableRecord {
            revision: Counter(0),
            backup: None,
        },
    )
    .unwrap();
    let candidate = gate.begin_prepared_save(&mut machine, &p.token).unwrap();
    for storage in [
        StorageDisposition::SessionOnly,
        StorageDisposition::Persisted {
            receipt: receipt(&candidate, &context().epochs),
        },
    ] {
        assert!(
            gate.register(&request(
                "premature-activation",
                Request::ActivatePrepared {
                    token: p.token.clone(),
                    storage
                },
                0
            ))
            .is_err()
        );
    }
    assert!(
        gate.register(&request(
            "premature-recover",
            Request::Recover {
                state: machine.recovery_state()
            },
            0
        ))
        .is_err()
    );
    assert!(
        gate.register(&request("other", Request::GetCatalog, 0))
            .is_err()
    );
    assert!(gate.reset_barrier().is_err());
    assert!(
        gate.register(&request(
            "cancel-save",
            Request::Cancel {
                target: import.request_id.clone()
            },
            0
        ))
        .is_err()
    );
    assert!(gate.restart_worker(&machine).is_err());
    gate.storage_failed(&mut machine).unwrap();
    assert_eq!(machine.active(), None);
    assert_eq!(
        candidate,
        gate.begin_prepared_save(&mut machine, &p.token).unwrap()
    );
    let receipt = receipt(&candidate, &context().epochs);
    gate.storage_committed(&mut machine, &receipt).unwrap();
    let activation = request(
        "activate",
        Request::ActivatePrepared {
            token: p.token.clone(),
            storage: StorageDisposition::Persisted { receipt },
        },
        0,
    );
    gate.register(&activation).unwrap();
    assert!(
        gate.register(&request(
            "cancel-activation",
            Request::Cancel {
                target: activation.request_id.clone()
            },
            0
        ))
        .is_err()
    );
    assert!(!gate.accept(&reply(
        &activation,
        Event::Activated {
            snapshot: id("wrong"),
            account_semantics: wire::account_semantic_digest(trusted.snapshot()).unwrap(),
            previous_generation: Counter(0),
            capability: p.capability.clone()
        },
        Counter(1)
    )));
    assert!(gate.accept(&reply(
        &activation,
        Event::Activated {
            snapshot: trusted.snapshot().id.clone(),
            account_semantics: wire::account_semantic_digest(trusted.snapshot()).unwrap(),
            previous_generation: Counter(0),
            capability: p.capability
        },
        Counter(1)
    )));
    machine.activate_committed().unwrap();
    assert_eq!(machine.active(), Some(&p.backup));
    let evaluate = request_in(
        "decision",
        Request::Evaluate { spec: spec() },
        Counter(1),
        gate.context().clone(),
    );
    gate.register(&evaluate).unwrap();
    assert!(gate.accept(&reply(
        &evaluate,
        Event::Evaluated {
            result: Box::new(result(&account()))
        },
        Counter(1)
    )));
}

#[test]
fn donor_fallback_and_required_rule_boundaries_reject_adversarial_resources() {
    let (trusted, comparison) = comparison_case(true);
    let mut wrong = comparison.comparison.clone();
    wrong.movement.recipient = id("c7");
    assert!(wrong.validate_for(&trusted).is_err());
    wrong = comparison.comparison.clone();
    if let ComparisonMode::CurrentLoadout { donor_fallback, .. } = &mut wrong.mode {
        *donor_fallback = Some(id("w0"));
    }
    assert!(wrong.validate_for(&trusted).is_err());
    let mut preferences = trusted.snapshot().preferences.clone();
    preferences.protected_items.insert(id("w1"));
    assert!(
        comparison
            .comparison
            .validate_for(&trusted.with_preferences(preferences).unwrap())
            .is_err()
    );
    let mut a = trusted.snapshot().clone();
    a.artifacts = supplied(vec![ArtifactInstance {
        id: id("flower"),
        set: id("syn:set"),
        slot: ArtifactSlot::Flower,
        level: Some(0),
        rarity: Some(1),
        main_stat: id("syn:hp"),
        substats: None,
        owner: OwnerRef::Unequipped,
        game_lock: Some(false),
        metadata: ArtifactMetadata {
            total_rolls: None,
            astral_mark: None,
            elixir_crafted: None,
            unactivated_substats: None,
        },
        source: None,
    }]);
    let a = ValidatedAccount::bundled(a, SampleId::SynWelcome).unwrap();
    if let ComparisonMode::CurrentLoadout { donor_fallback, .. } = &mut wrong.mode {
        *donor_fallback = Some(id("flower"));
    }
    assert!(wrong.validate_for(&a).is_err());
    let mut s = spec();
    s.constraints.required_rules.insert(id("required-rule"));
    assert!(allocation().validate(&account(), &s).is_err());
    let mut candidate = allocation();
    candidate.constraints.push(ConstraintOutcome {
        rule: id("required-rule"),
        hard: true,
        satisfied: true,
        reasons: vec![],
    });
    candidate.validate(&account(), &s).unwrap();
    s.constraints
        .metric_minima
        .insert(id("syn:minimum"), Fixed::ONE);
    assert!(candidate.validate(&account(), &s).is_err());
}

#[test]
fn documented_messages_are_exactly_the_frozen_wire_types() {
    let docs = include_str!("../../../.agent-docs/CONTRACTS.md");
    let mut requests = 0;
    let mut backups = 0;
    for block in docs.split("```json\n").skip(1) {
        let text = block.split("\n```").next().unwrap();
        if text.contains("\"account\"") {
            wire::decode::<Backup>(text).unwrap().restore(None).unwrap();
            backups += 1;
        } else {
            wire::decode_request(text).unwrap();
            requests += 1;
        }
    }
    assert_eq!((requests, backups), (2, 1));
}

fn flower(name: &str, owner: OwnerRef) -> ArtifactInstance {
    ArtifactInstance {
        id: id(name),
        set: id("syn:set"),
        slot: ArtifactSlot::Flower,
        level: Some(0),
        rarity: Some(1),
        main_stat: id("syn:hp"),
        substats: None,
        owner,
        game_lock: Some(false),
        metadata: ArtifactMetadata {
            total_rolls: None,
            astral_mark: None,
            elixir_crafted: None,
            unactivated_substats: None,
        },
        source: None,
    }
}

#[test]
fn included_equipment_closes_unknown_owner_availability_without_blocking_unrelated_subsets() {
    for artifact in [false, true] {
        let mut a = account();
        if let Section::Supplied { records, .. } = &mut a.characters {
            records[7].availability = CharacterAvailability::Unknown;
        }
        let item: ItemId = if artifact {
            a.artifacts = supplied(vec![flower("flower", OwnerRef::Character(id("c7")))]);
            id("flower")
        } else {
            if let Section::Supplied { records, .. } = &mut a.weapons {
                records[0].owner = OwnerRef::Character(id("c7"));
            }
            id("w0")
        };
        let trusted = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
        let mut s = spec();
        s.scope = EvaluationScope::Subset {
            characters: (0..4).map(|n| id(&format!("c{n}"))).collect(),
            items: [item.clone()].into(),
            exclusions: vec![],
        };
        assert_eq!(
            s.validate_for(&trusted).unwrap_err().code,
            IssueCode::UnknownAvailability
        );
        let mut used = allocation();
        if artifact {
            used.teams[0].builds[0].weapon = None;
            used.teams[0].builds[0]
                .artifacts
                .insert(ArtifactSlot::Flower, item.clone());
            used.footprint.items = [item.clone()].into();
        }
        assert_eq!(
            used.validate(&a, &s).unwrap_err().code,
            IssueCode::UnknownAvailability
        );
        assert_eq!(
            a.validate_move(&ItemMove {
                item: item.clone(),
                recipient: id("c0")
            })
            .unwrap_err()
            .code,
            IssueCode::UnknownAvailability
        );
        if let EvaluationScope::Subset { items, .. } = &mut s.scope {
            *items = [id("w1")].into();
        }
        let mut unrelated = allocation();
        unrelated.teams[0].builds[0].weapon = Some(id("w1"));
        unrelated.footprint.items = [id("w1")].into();
        s.validate_for(&trusted).unwrap();
        unrelated.validate(&a, &s).unwrap();
        assert_eq!(
            a.validate_move(&ItemMove {
                item: id("w1"),
                recipient: id("c7")
            })
            .unwrap_err()
            .code,
            IssueCode::UnknownAvailability
        );
        if let Section::Supplied { records, .. } = &mut a.characters {
            records[7].availability = CharacterAvailability::Resolved {
                owned: id("owned-7"),
                identity: id("syn:identity/7"),
            };
        }
        a.validate_move(&ItemMove {
            item,
            recipient: id("c0"),
        })
        .unwrap();
    }
}

fn select_donor_form(result: &mut DecisionResult, form: &str) {
    let a = &mut result.allocations[0];
    a.teams[0].builds[1].character = id(form);
    a.teams[0].builds[1].id = id(&format!("build-{form}"));
    a.footprint.characters.remove(&id("c1"));
    a.footprint.characters.insert(id(form));
}

#[test]
fn donor_fallback_and_loss_follow_participating_forms_but_preserve_imported_owner() {
    for current in [false, true] {
        let (original, mut comparison) = comparison_case(current);
        comparison.validate_for(&original).unwrap();
        let mut a = original.snapshot().clone();
        if let Section::Supplied { records, .. } = &mut a.characters {
            records[5].availability = records[1].availability.clone();
        }
        let trusted = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
        select_donor_form(&mut comparison.before, "c5");
        select_donor_form(&mut comparison.after, "c5");
        if let ComparisonMode::CurrentLoadout { allocation, .. } = &mut comparison.comparison.mode {
            **allocation = comparison.before.allocations[0].clone();
        }
        comparison.after.spec = comparison.comparison.intervention_spec();
        rebind(&mut comparison.before, &a);
        rebind(&mut comparison.after, &a);
        let donor = &mut comparison.donor.as_mut().unwrap().forms[0];
        donor.character = id("c5");
        donor.before_builds = vec![id("build-c5")];
        donor.after_builds = vec![id("build-c5")];
        comparison.validate_for(&trusted).unwrap();
        assert_eq!(comparison.original_owner, OwnerRef::Character(id("c1")));
        assert_eq!(
            comparison.donor.as_ref().unwrap().availability,
            id("owned-1")
        );
        if current {
            let mut no_fallback = comparison.clone();
            if let ComparisonMode::CurrentLoadout { donor_fallback, .. } =
                &mut no_fallback.comparison.mode
            {
                *donor_fallback = None;
            }
            no_fallback.after.allocations[0].teams[0].builds[1].weapon = None;
            no_fallback.after.allocations[0]
                .footprint
                .items
                .remove(&id("w1"));
            no_fallback.after.spec = no_fallback.comparison.intervention_spec();
            rebind(&mut no_fallback.after, &a);
            no_fallback.validate_for(&trusted).unwrap();
        }
        let mut misattributed = comparison.clone();
        misattributed
            .other_changes
            .push(misattributed.donor.as_ref().unwrap().forms[0].clone());
        misattributed.donor.as_mut().unwrap().forms = vec![CharacterChange {
            character: id("c1"),
            before_builds: vec![],
            after_builds: vec![],
            metric_deltas: BTreeMap::new(),
            reasons: vec![],
        }];
        assert!(misattributed.validate_for(&trusted).is_err());
        let mut pseudo = comparison.comparison.clone();
        pseudo.movement.recipient = id("c5");
        assert!(pseudo.validate_for(&trusted).is_err());
        pseudo.movement.recipient = id("c1");
        assert!(pseudo.validate_for(&trusted).is_err());
        if !current {
            comparison.before.allocations[0].teams[0].builds[1].character = id("c1");
            comparison.before.allocations[0]
                .footprint
                .characters
                .remove(&id("c5"));
            comparison.before.allocations[0]
                .footprint
                .characters
                .insert(id("c1"));
            let mut before_form = comparison.donor.as_ref().unwrap().forms[0].clone();
            before_form.character = id("c1");
            before_form.after_builds.clear();
            comparison.donor.as_mut().unwrap().forms[0]
                .before_builds
                .clear();
            comparison.donor.as_mut().unwrap().forms.push(before_form);
            comparison.validate_for(&trusted).unwrap();
        }
    }
}

#[test]
fn reached_search_limits_require_the_exact_admitted_node_boundary() {
    let a = account();
    let trusted = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
    for limit in [
        SearchLimit::BuildNodes,
        SearchLimit::TeamNodes,
        SearchLimit::AllocationNodes,
    ] {
        for count in [1, 99, 100, 101] {
            let mut r = result(&a);
            r.search.status = SearchStatus::Bounded;
            r.search.frontier = FrontierPresentation::AmongReturned;
            r.search.limits_reached = vec![limit.clone()];
            match limit {
                SearchLimit::BuildNodes => r.search.counters.build_nodes = Counter(count),
                SearchLimit::TeamNodes => r.search.counters.team_nodes = Counter(count),
                SearchLimit::AllocationNodes => r.search.counters.allocation_nodes = Counter(count),
            }
            assert_eq!(
                r.validate_for(&trusted).is_ok(),
                count == 100,
                "{limit:?}, count {count}"
            );
            r.search.status = SearchStatus::Cancelled;
            assert_eq!(r.validate_for(&trusted).is_ok(), count == 100);
        }
    }
    let mut presentation_only = result(&a);
    presentation_only.search.frontier = FrontierPresentation::Truncated {
        omitted: Counter(10),
    };
    presentation_only.validate_for(&trusted).unwrap();
    presentation_only
        .search
        .limits_reached
        .push(SearchLimit::AllocationNodes);
    assert!(presentation_only.validate_for(&trusted).is_err());
    let mut multiple = result(&a);
    multiple.search.status = SearchStatus::Bounded;
    multiple.search.frontier = FrontierPresentation::AmongReturned;
    multiple.search.counters.build_nodes = Counter(100);
    multiple.search.counters.team_nodes = Counter(100);
    multiple.search.limits_reached = vec![SearchLimit::BuildNodes, SearchLimit::TeamNodes];
    multiple.validate_for(&trusted).unwrap();
    multiple
        .search
        .limits_reached
        .push(SearchLimit::AllocationNodes);
    assert!(multiple.validate_for(&trusted).is_err());
    multiple.search.limits_reached = vec![SearchLimit::BuildNodes, SearchLimit::BuildNodes];
    assert!(multiple.validate_for(&trusted).is_err());
}

#[test]
fn foreground_preparation_export_and_catalog_are_explicitly_cancellable() {
    let trusted = ValidatedAccount::imported(account()).unwrap();
    let p = preview(&trusted);
    let download = Download {
        format: ExportFormat::Canonical,
        filename: "account.json".into(),
        media_type: "application/json".into(),
        utf8: "{}".into(),
        sha256: Digest::of(b"{}"),
    };
    for (payload, success) in [
        (
            Request::PrepareImport {
                input: ImportInput::Sample {
                    id: SampleId::SynWelcome,
                },
            },
            Event::ImportPrepared {
                preview: Box::new(p.clone()),
            },
        ),
        (
            Request::PreparePreferences {
                preferences: AllocationPreferences::default(),
            },
            Event::ImportPrepared {
                preview: Box::new(p.clone()),
            },
        ),
        (
            Request::Export {
                format: ExportFormat::Canonical,
            },
            Event::Exported { download },
        ),
        (
            Request::GetCatalog,
            Event::Catalog {
                catalog: Catalog {
                    schema_version: CatalogVersion,
                    samples: vec![],
                    mechanics: vec![],
                },
            },
        ),
    ] {
        let operation = request("foreground", payload, 0);
        let mut control = SessionGate::new(context(), Counter(0));
        control.register(&operation).unwrap();
        assert!(control.accept(&reply(&operation, success.clone(), Counter(0))));
        let mut gate = SessionGate::new(context(), Counter(0));
        gate.register(&operation).unwrap();
        assert!(
            gate.register(&request(
                "wrong-target",
                Request::Cancel {
                    target: id("other")
                },
                0
            ))
            .is_err()
        );
        let cancel = request(
            "cancel",
            Request::Cancel {
                target: operation.request_id.clone(),
            },
            0,
        );
        gate.register(&cancel).unwrap();
        assert!(!gate.accept(&reply(&operation, success.clone(), Counter(0))));
        assert!(gate.accept(&reply(
            &cancel,
            Event::CancelAcknowledged {
                target: operation.request_id.clone(),
                was_running: true
            },
            Counter(0)
        )));
        assert!(!gate.accept(&reply(&operation, success, Counter(0))));
        assert!(!gate.preview_authoritative(&p.token, &p.context));
        assert!(
            gate.register(&request(
                "revive-preview",
                Request::ActivatePrepared {
                    token: p.token.clone(),
                    storage: StorageDisposition::SessionOnly
                },
                0
            ))
            .is_err()
        );
    }
}

fn durable_account() -> DurableRecord {
    DurableRecord {
        revision: Counter(10),
        backup: Some(
            Backup::from_validated(&ValidatedAccount::imported(account()).unwrap()).unwrap(),
        ),
    }
}
fn gate_request(gate: &SessionGate, name: &str, payload: Request) -> WorkerRequest {
    request_in(name, payload, gate.generation(), gate.context().clone())
}
fn assert_foreground_blocked(gate: &mut SessionGate) {
    for payload in [
        Request::GetCatalog,
        Request::Evaluate { spec: spec() },
        Request::Export {
            format: ExportFormat::Canonical,
        },
        Request::Cancel {
            target: id("reset"),
        },
    ] {
        assert!(
            gate.register(&gate_request(gate, "blocked", payload))
                .is_err()
        );
    }
}
fn assert_old_account_usable(gate: &mut SessionGate) {
    let evaluation = gate_request(
        gate,
        "evaluate-after-abort",
        Request::Evaluate { spec: spec() },
    );
    gate.register(&evaluation).unwrap();
    assert!(gate.accept(&reply(
        &evaluation,
        Event::Evaluated {
            result: Box::new(result(&account()))
        },
        gate.generation()
    )));
    let export = gate_request(
        gate,
        "export-after-abort",
        Request::Export {
            format: ExportFormat::Canonical,
        },
    );
    gate.register(&export).unwrap();
    let text = wire::encode(&durable_account().backup.unwrap()).unwrap();
    let download = Download {
        format: ExportFormat::Canonical,
        filename: "account.json".into(),
        media_type: "application/json".into(),
        sha256: Digest::of(text.as_bytes()),
        utf8: text,
    };
    assert!(gate.accept(&reply(
        &export,
        Event::Exported { download },
        gate.generation()
    )));
}

#[test]
fn confirmed_reset_abort_restores_usability_without_rolling_back_the_reset_epoch() {
    let old = durable_account();
    let mut machine = CommitMachine::new(context().epochs, true, old.clone()).unwrap();
    let mut gate = SessionGate::new(context(), Counter(0));
    let import = gate_request(
        &gate,
        "late-import",
        Request::PrepareImport {
            input: ImportInput::Sample {
                id: SampleId::SynWelcome,
            },
        },
    );
    gate.register(&import).unwrap();
    gate.reset_barrier().unwrap();
    let candidate = gate.begin_reset_save(&mut machine).unwrap();
    assert_eq!(candidate.revision, Counter(11));
    assert_eq!(candidate.backup, None);
    assert_foreground_blocked(&mut gate);
    assert!(gate.restart_worker(&machine).is_err());
    assert!(gate.abort_reset_before_save(&mut machine).is_err());
    gate.storage_failed(&mut machine).unwrap();
    assert_eq!(gate.context().epochs.reset, Counter(1));
    assert_eq!(gate.generation(), Counter(0));
    assert_eq!(machine.durable(), Some(&old));
    assert_eq!(machine.active(), old.backup.as_ref());
    assert!(!gate.accept(&reply(
        &import,
        Event::ImportPrepared {
            preview: Box::new(preview(&ValidatedAccount::imported(account()).unwrap()))
        },
        Counter(0)
    )));
    assert_old_account_usable(&mut gate);
    gate.reset_barrier().unwrap();
    assert_eq!(gate.context().epochs.reset, Counter(2));
    gate.abort_reset_before_save(&mut machine).unwrap();
    assert_eq!(machine.active(), old.backup.as_ref());
}

#[test]
fn uncertain_reset_storage_stays_blocked_until_a_settled_read_proves_abort() {
    let old = durable_account();
    let mut machine = CommitMachine::new(context().epochs, true, old.clone()).unwrap();
    let mut gate = SessionGate::new(context(), Counter(0));
    gate.reset_barrier().unwrap();
    gate.begin_reset_save(&mut machine).unwrap();
    assert_foreground_blocked(&mut gate);
    assert!(gate.restart_worker(&machine).is_err());
    assert!(
        machine
            .recover(
                gate.context().epochs.clone(),
                RecoveryState::Durable {
                    record: Box::new(old.clone())
                }
            )
            .is_err()
    );
    let mut unrelated = old.clone();
    unrelated.revision = Counter(12);
    assert!(gate.settle_storage(&mut machine, unrelated).is_err());
    assert_eq!(machine.phase(), &CommitPhase::Writing);
    assert_foreground_blocked(&mut gate);
    gate.settle_storage(&mut machine, old.clone()).unwrap();
    assert_eq!(machine.phase(), &CommitPhase::Idle);
    assert_eq!(machine.active(), old.backup.as_ref());
    assert_eq!(gate.context().epochs.reset, Counter(1));
    assert_old_account_usable(&mut gate);
}

#[test]
fn committed_reset_survives_lost_acknowledgement_and_restart_without_another_clear() {
    for callback_received in [false, true] {
        let old = durable_account();
        let mut machine = CommitMachine::new(context().epochs, true, old.clone()).unwrap();
        let mut gate = SessionGate::new(context(), Counter(0));
        gate.reset_barrier().unwrap();
        let tombstone = gate.begin_reset_save(&mut machine).unwrap();
        let committed = receipt(&tombstone, &gate.context().epochs);
        if callback_received {
            gate.storage_committed(&mut machine, &committed).unwrap();
        } else {
            assert!(gate.restart_worker(&machine).is_err());
            gate.settle_storage(&mut machine, tombstone.clone())
                .unwrap();
        }
        assert_eq!(machine.active(), old.backup.as_ref());
        assert_foreground_blocked(&mut gate);
        let reset = gate_request(&gate, "reset", Request::Reset { receipt: committed });
        gate.register(&reset).unwrap();
        assert_foreground_blocked(&mut gate);
        gate.restart_worker(&machine).unwrap();
        assert!(!gate.accept(&reply(
            &reset,
            Event::ResetAcknowledged {
                previous_generation: Counter(0)
            },
            Counter(1)
        )));
        assert_foreground_blocked(&mut gate);
        assert!(gate.reset_barrier().is_err());
        assert!(gate.begin_reset_save(&mut machine).is_err());
        for state in [
            RecoveryState::Durable {
                record: Box::new(old),
            },
            RecoveryState::SessionOnly { backup: None },
        ] {
            assert!(
                gate.register(&gate_request(
                    &gate,
                    "resurrect",
                    Request::Recover { state }
                ))
                .is_err()
            );
        }
        let state = RecoveryState::Durable {
            record: Box::new(tombstone.clone()),
        };
        let recovery = gate_request(
            &gate,
            "recover-tombstone",
            Request::Recover {
                state: state.clone(),
            },
        );
        gate.register(&recovery).unwrap();
        assert_foreground_blocked(&mut gate);
        machine
            .recover(gate.context().epochs.clone(), state)
            .unwrap();
        assert!(gate.accept(&reply(
            &recovery,
            Event::ResetAcknowledged {
                previous_generation: Counter(0)
            },
            Counter(1)
        )));
        assert_eq!(machine.active(), None);
        assert_eq!(machine.durable(), Some(&tombstone));
        assert_eq!(gate.context().account_semantics, None);
        assert_eq!(gate.context().epochs.reset, Counter(1));
        assert_eq!(gate.generation(), Counter(1));
        assert!(!gate.accept(&reply(
            &reset,
            Event::ResetAcknowledged {
                previous_generation: Counter(0)
            },
            Counter(1)
        )));
        gate.register(&gate_request(&gate, "usable", Request::GetCatalog))
            .unwrap();
    }
}

#[test]
fn unavailable_idb_allows_initial_session_activation_restart_and_explicit_memory_discard() {
    let mut c = context();
    c.account_semantics = None;
    let mut machine = CommitMachine::session_only(c.epochs.clone(), true).unwrap();
    assert!(CommitMachine::session_only(c.epochs.clone(), false).is_err());
    assert_eq!(machine.durable(), None);
    let mut gate = SessionGate::new(c.clone(), Counter(0));
    let trusted = ValidatedAccount::imported(account()).unwrap();
    let mut p = preview(&trusted);
    p.context = c;
    let import = gate_request(
        &gate,
        "import",
        Request::PrepareImport {
            input: ImportInput::Restore {
                backup: Box::new(p.backup.clone()),
            },
        },
    );
    gate.register(&import).unwrap();
    assert!(gate.accept(&reply(
        &import,
        Event::ImportPrepared {
            preview: Box::new(p.clone())
        },
        Counter(0)
    )));
    machine
        .prepare(Some(p.backup.clone()), &gate.context().epochs)
        .unwrap();
    assert!(machine.begin_save(&gate.context().epochs).is_err());
    let activation = gate_request(
        &gate,
        "activate-session",
        Request::ActivatePrepared {
            token: p.token.clone(),
            storage: StorageDisposition::SessionOnly,
        },
    );
    gate.register(&activation).unwrap();
    let activated = |previous_generation| Event::Activated {
        snapshot: trusted.snapshot().id.clone(),
        account_semantics: wire::account_semantic_digest(trusted.snapshot()).unwrap(),
        previous_generation,
        capability: p.capability.clone(),
    };
    machine.activate_session_only().unwrap();
    assert!(gate.accept(&reply(&activation, activated(Counter(0)), Counter(1))));
    assert_eq!(machine.active(), Some(&p.backup));
    assert_eq!(machine.durable(), None);
    gate.restart_worker(&machine).unwrap();
    let state = RecoveryState::SessionOnly {
        backup: Some(Box::new(p.backup.clone())),
    };
    assert_eq!(machine.recovery_state(), state);
    let invented = RecoveryState::Durable {
        record: Box::new(durable_account()),
    };
    assert!(
        gate.register(&gate_request(
            &gate,
            "invented-read",
            Request::Recover { state: invented }
        ))
        .is_err()
    );
    let recover = gate_request(
        &gate,
        "recover-session",
        Request::Recover {
            state: state.clone(),
        },
    );
    gate.register(&recover).unwrap();
    machine
        .recover(gate.context().epochs.clone(), state)
        .unwrap();
    assert!(gate.accept(&reply(&recover, activated(Counter(1)), Counter(2))));
    gate.reset_barrier().unwrap();
    assert!(gate.begin_reset_save(&mut machine).is_err());
    gate.abort_reset_before_save(&mut machine).unwrap();
    assert_eq!(machine.active(), Some(&p.backup));
    assert_eq!(gate.context().epochs.reset, Counter(1));
    gate.discard_session_barrier(&mut machine).unwrap();
    assert_eq!(machine.active(), None);
    assert_eq!(machine.durable(), None);
    let discard = gate_request(&gate, "discard-session", Request::DiscardSession);
    gate.register(&discard).unwrap();
    assert_foreground_blocked(&mut gate);
    assert!(gate.accept(&reply(
        &discard,
        Event::SessionDiscarded {
            previous_generation: Counter(2)
        },
        Counter(3)
    )));
    assert_eq!(gate.context().account_semantics, None);
    gate.restart_worker(&machine).unwrap();
    let state = RecoveryState::SessionOnly { backup: None };
    let recover = gate_request(
        &gate,
        "recover-empty-session",
        Request::Recover {
            state: state.clone(),
        },
    );
    gate.register(&recover).unwrap();
    assert!(!gate.accept(&reply(
        &recover,
        Event::ResetAcknowledged {
            previous_generation: Counter(3)
        },
        Counter(4)
    )));
    machine
        .recover(gate.context().epochs.clone(), state)
        .unwrap();
    assert!(gate.accept(&reply(
        &recover,
        Event::SessionDiscarded {
            previous_generation: Counter(3)
        },
        Counter(4)
    )));
    assert_eq!(machine.durable(), None);
    assert_eq!(machine.active(), None);
    let mut durable = CommitMachine::new(context().epochs, true, durable_account()).unwrap();
    assert!(durable.discard_session(context().epochs).is_err());
}
