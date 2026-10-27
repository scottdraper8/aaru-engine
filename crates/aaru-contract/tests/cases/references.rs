use super::*;
use aaru_contract::references::ResolvedSubject;

fn reason<S>(subjects: Vec<S>) -> CausalReason<S> {
    CausalReason {
        id: id("cause"),
        code: ReasonCode::ConstraintFailed,
        rule: id("synthetic:constraint"),
        subjects,
        causes: vec![],
        parameters: BTreeMap::new(),
        provenance: vec![],
    }
}
fn declared_result() -> (ValidatedAccount, DecisionResult) {
    let a = account();
    let trusted = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
    let mut r = result(&a);
    r.spec
        .assumptions
        .values
        .insert(id("global"), AssumptionValue::Flag(true));
    r.spec.encounters[0]
        .parameters
        .insert(id("encounter-only"), AssumptionValue::Quantity(Fixed::ONE));
    rebind(&mut r, &a);
    (trusted, r)
}
fn global(key: &str) -> AssumptionRef {
    AssumptionRef::Global { key: id(key) }
}
fn encounter(index: u16, key: &str) -> AssumptionRef {
    AssumptionRef::Encounter {
        index,
        key: id(key),
    }
}
fn team(allocation: &str, team: &str) -> Subject {
    Subject::Team {
        allocation: id(allocation),
        team: id(team),
    }
}
fn uncertainty(subject: Subject) -> LocalizedUncertainty {
    LocalizedUncertainty {
        subject,
        metric: None,
        fidelity: Fidelity::Unknown,
        assumptions: vec![],
        explanation: "Normative unknown, not a game fact".into(),
        provenance: vec![],
    }
}

#[test]
fn decision_subjects_resolve_owned_not_just_equipped_resources_and_declared_context() {
    let (trusted, mut r) = declared_result();
    r.reasons.push(reason(vec![
        Subject::Account(id("account-1")),
        Subject::Character(id("c7")),
        Subject::Item(id("w1")),
        team("allocation-1", "team-1"),
        Subject::Allocation(id("allocation-1")),
        Subject::Assumption(global("global")),
        Subject::Assumption(encounter(0, "encounter-only")),
        Subject::Dimension(Dimension::DefensiveSufficiency),
    ]));
    r.allocations[0].reasons = vec![id("cause")];
    r.validate_for(&trusted).unwrap();
    let context = r.subject_context(&trusted);
    assert!(
        matches!(context.resolve(&Subject::Item(id("w1"))).unwrap(), ResolvedSubject::Weapon(w) if w.owner == OwnerRef::Unequipped)
    );
    assert!(
        matches!(context.resolve(&team("allocation-1", "team-1")).unwrap(), ResolvedSubject::Team { allocation, team } if allocation.id == id("allocation-1") && team.encounter_index == 0)
    );
    for bad in [
        Subject::Account(id("other")),
        Subject::Character(id("absent")),
        Subject::Item(id("absent")),
        team("allocation-1", "absent"),
        team("absent", "team-1"),
        Subject::Allocation(id("absent")),
        Subject::Assumption(global("absent")),
        Subject::Assumption(global("encounter-only")),
        Subject::Assumption(encounter(0, "global")),
        Subject::Assumption(encounter(1, "encounter-only")),
    ] {
        let mut wrong = r.clone();
        wrong.reasons[0].subjects = vec![bad.clone()];
        assert_eq!(
            wrong.validate_for(&trusted).unwrap_err().code,
            IssueCode::InvalidReference,
            "{bad:?}"
        );
    }
    for bad in [
        Parameter::Character(id("absent")),
        Parameter::Item(id("absent")),
    ] {
        let mut wrong = r.clone();
        wrong.reasons[0].parameters.insert("resource".into(), bad);
        assert_eq!(
            wrong.validate_for(&trusted).unwrap_err().code,
            IssueCode::InvalidReference
        );
    }
    r.reasons[0]
        .parameters
        .insert("unequipped".into(), Parameter::Item(id("w1")));
    r.validate_for(&trusted).unwrap();
    let mut alternate = r.allocations[0].clone();
    alternate.id = id("allocation-2");
    alternate.teams[0].builds[0].weapon = Some(id("w1"));
    alternate.footprint.items = [id("w1")].into();
    r.allocations.push(alternate);
    r.reasons[0].subjects = vec![team("allocation-2", "team-1")];
    r.validate_for(&trusted).unwrap();
    let context = r.subject_context(&trusted);
    assert!(
        matches!(context.resolve(&team("allocation-2", "team-1")).unwrap(), ResolvedSubject::Team { team, .. } if team.builds[0].weapon == Some(id("w1")))
    );
}

#[test]
fn localized_uncertainty_resolves_subject_assumptions_and_declared_metrics_independently() {
    let (trusted, mut r) = declared_result();
    r.uncertainty.push(uncertainty(Subject::Item(id("w1"))));
    r.uncertainty[0].assumptions = vec![global("global"), encounter(0, "encounter-only")];
    r.allocations[0].teams[0].builds[0].metrics.insert(
        id("known-metric"),
        Metric::Unavailable {
            issue: IssueCode::UnsupportedMechanics,
        },
    );
    r.validate_for(&trusted).unwrap();
    r.uncertainty[0].subject = Subject::Character(id("c0"));
    r.uncertainty[0].metric = Some(id("known-metric"));
    r.validate_for(&trusted).unwrap();
    let mut unrelated = r.clone();
    unrelated.uncertainty[0].subject = Subject::Character(id("c7"));
    assert_eq!(
        unrelated.validate_for(&trusted).unwrap_err().code,
        IssueCode::InvalidReference
    );
    for subject in [
        Subject::Character(id("absent")),
        Subject::Item(id("absent")),
        Subject::Account(id("absent")),
        Subject::Allocation(id("absent")),
        team("allocation-1", "absent"),
        Subject::Assumption(global("absent")),
    ] {
        let mut wrong = r.clone();
        wrong.uncertainty[0].subject = subject;
        assert_eq!(
            wrong.validate_for(&trusted).unwrap_err().code,
            IssueCode::InvalidReference
        );
    }
    for reference in [
        global("absent"),
        global("encounter-only"),
        encounter(0, "global"),
        encounter(3, "encounter-only"),
    ] {
        let mut wrong = r.clone();
        wrong.uncertainty[0].assumptions = vec![reference];
        assert_eq!(
            wrong.validate_for(&trusted).unwrap_err().code,
            IssueCode::InvalidReference
        );
    }
    r.uncertainty[0].metric = Some(id("undeclared-metric"));
    assert_eq!(
        r.validate_for(&trusted).unwrap_err().code,
        IssueCode::InvalidReference
    );
}

#[test]
fn blocked_search_can_explain_constraints_and_unavailable_dimensions_without_dangling_candidates() {
    let (trusted, mut r) = declared_result();
    r.allocations.clear();
    r.search.status = SearchStatus::BlockedByUnsupportedMechanics;
    r.search.frontier = FrontierPresentation::AmongReturned;
    r.reasons.push(reason(vec![
        Subject::Character(id("c0")),
        Subject::Item(id("w1")),
        Subject::Assumption(global("global")),
        Subject::Dimension(Dimension::OutputPotential),
    ]));
    r.spec
        .constraints
        .metric_minima
        .insert(id("required-energy"), Fixed::ONE);
    r.uncertainty
        .push(uncertainty(Subject::Dimension(Dimension::EnergyStability)));
    r.uncertainty[0].metric = Some(id("required-energy"));
    rebind(&mut r, trusted.snapshot());
    r.validate_for(&trusted).unwrap();
    r.reasons[0]
        .subjects
        .push(Subject::Allocation(id("nonreturned")));
    assert_eq!(
        r.validate_for(&trusted).unwrap_err().code,
        IssueCode::InvalidReference
    );
    r.reasons[0].subjects.pop();
    r.reasons[0].subjects.push(team("nonreturned", "team-1"));
    assert_eq!(
        r.validate_for(&trusted).unwrap_err().code,
        IssueCode::InvalidReference
    );
}

fn scoped(side: ComparisonSide, subject: Subject) -> ComparisonSubject {
    ComparisonSubject { side, subject }
}

#[test]
fn comparison_subjects_resolve_in_the_named_before_or_after_world_and_changes_stay_linked() {
    let (trusted, mut c) = comparison_case(false);
    c.after.allocations[0].id = id("after-allocation");
    c.after.allocations[0].teams[0].id = id("after-team");
    c.reasons.push(reason(vec![
        scoped(ComparisonSide::Before, team("allocation-1", "team-1")),
        scoped(
            ComparisonSide::After,
            team("after-allocation", "after-team"),
        ),
        scoped(ComparisonSide::After, Subject::Item(id("w1"))),
    ]));
    c.recipient.reasons.push(id("cause"));
    c.donor.as_mut().unwrap().forms[0].reasons.push(id("cause"));
    c.before.allocations[0].teams[0].builds[0].metrics.insert(
        id("energy"),
        Metric::Unavailable {
            issue: IssueCode::UnsupportedMechanics,
        },
    );
    c.recipient.metric_deltas.insert(
        id("energy"),
        Metric::Unavailable {
            issue: IssueCode::UnsupportedMechanics,
        },
    );
    c.validate_for(&trusted).unwrap();
    for reference in [
        scoped(ComparisonSide::After, team("allocation-1", "team-1")),
        scoped(
            ComparisonSide::Before,
            Subject::Allocation(id("after-allocation")),
        ),
        scoped(ComparisonSide::Before, Subject::Character(id("absent"))),
        scoped(ComparisonSide::After, Subject::Item(id("absent"))),
        scoped(ComparisonSide::Before, Subject::Account(id("unrelated"))),
        scoped(
            ComparisonSide::After,
            Subject::Assumption(global("undeclared")),
        ),
    ] {
        let mut wrong = c.clone();
        wrong.reasons[0].subjects = vec![reference];
        assert_eq!(
            wrong.validate_for(&trusted).unwrap_err().code,
            IssueCode::InvalidReference
        );
    }
    let mut wrong = c.clone();
    wrong.recipient.before_builds = vec![id("absent")];
    assert!(wrong.validate_for(&trusted).is_err());
    let mut wrong = c.clone();
    wrong.donor.as_mut().unwrap().forms[0]
        .reasons
        .push(id("absent"));
    assert!(wrong.validate_for(&trusted).is_err());
    let mut wrong = c.clone();
    wrong.recipient.metric_deltas.insert(
        id("absent"),
        Metric::Unavailable {
            issue: IssueCode::UnsupportedMechanics,
        },
    );
    assert!(wrong.validate_for(&trusted).is_err());
    let mut wrong = c.clone();
    wrong.reasons[0]
        .parameters
        .insert("resource".into(), Parameter::Item(id("absent")));
    assert_eq!(
        wrong.validate_for(&trusted).unwrap_err().code,
        IssueCode::InvalidReference
    );
    let mut wrong = c;
    wrong.reasons[0].causes.push(id("cause"));
    assert_eq!(
        wrong.validate_for(&trusted).unwrap_err().code,
        IssueCode::InvalidReference
    );
}

#[test]
fn contextual_result_admission_is_mandatory_and_rejection_does_not_consume_the_pending_request() {
    let (trusted, mut r) = declared_result();
    r.reasons.push(reason(vec![Subject::Character(id("c0"))]));
    let req = request(
        "contextual",
        Request::Evaluate {
            spec: r.spec.clone().into(),
        },
        0,
    );
    let mut gate = SessionGate::new(context(), Counter(0));
    gate.register(&req).unwrap();
    let valid = reply(
        &req,
        Event::Evaluated {
            result: Box::new(r.clone()),
        },
        Counter(0),
    );
    assert!(!gate.accept(&valid));
    for surface in 0..3 {
        let mut wrong = r.clone();
        match surface {
            0 => wrong.reasons[0].subjects = vec![Subject::Item(id("absent"))],
            1 => wrong
                .uncertainty
                .push(uncertainty(Subject::Account(id("unrelated")))),
            _ => wrong.allocations[0].reasons.push(id("dangling")),
        }
        let event = reply(
            &req,
            Event::Evaluated {
                result: Box::new(wrong),
            },
            Counter(0),
        );
        let decoded = wire::decode::<WorkerEvent>(&wire::encode(&event).unwrap()).unwrap();
        assert_eq!(
            gate.accept_result(&decoded, &trusted).unwrap_err().code,
            IssueCode::InvalidReference
        );
    }
    assert!(gate.accept_result(&valid, &trusted).unwrap());
    assert!(!gate.accept_result(&valid, &trusted).unwrap());
    let (trusted, mut c) = comparison_case(false);
    let mut comparison_context = context();
    comparison_context.account_semantics =
        Some(wire::account_semantic_digest(trusted.snapshot()).unwrap());
    let mut gate = SessionGate::new(comparison_context.clone(), Counter(0));
    let req = request_in(
        "comparison",
        Request::Compare {
            comparison: c.comparison.clone().into(),
        },
        Counter(0),
        comparison_context,
    );
    gate.register(&req).unwrap();
    c.reasons.push(reason(vec![scoped(
        ComparisonSide::Before,
        Subject::Character(id("absent")),
    )]));
    let bad = reply(
        &req,
        Event::Compared {
            result: Box::new(c.clone()),
        },
        Counter(0),
    );
    assert!(gate.accept_result(&bad, &trusted).is_err());
    c.reasons[0].subjects.clear();
    let valid = reply(
        &req,
        Event::Compared {
            result: Box::new(c),
        },
        Counter(0),
    );
    assert!(!gate.accept(&valid));
    assert!(gate.accept_result(&valid, &trusted).unwrap());
    let (trusted, baseline) = declared_result();
    let scenario = SensitivityScenario {
        id: id("scenario"),
        label: "Explicit unchanged world".into(),
        assumptions: baseline.spec.assumptions.clone(),
        encounters: baseline.spec.encounters.clone(),
    };
    let req = request(
        "sensitivity",
        Request::Sensitivity {
            baseline: baseline.spec.clone().into(),
            scenarios: vec![scenario.clone()].into(),
        },
        0,
    );
    let mut gate = SessionGate::new(context(), Counter(0));
    gate.register(&req).unwrap();
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
    sensitivity.scenarios[0]
        .result
        .uncertainty
        .push(uncertainty(Subject::Item(id("absent"))));
    let bad = reply(
        &req,
        Event::SensitivityCompleted {
            result: Box::new(sensitivity.clone()),
        },
        Counter(0),
    );
    assert!(gate.accept_result(&bad, &trusted).is_err());
    sensitivity.scenarios[0].result.uncertainty.clear();
    let valid = reply(
        &req,
        Event::SensitivityCompleted {
            result: Box::new(sensitivity),
        },
        Counter(0),
    );
    assert!(!gate.accept(&valid));
    assert!(gate.accept_result(&valid, &trusted).unwrap());
}

#[test]
fn topological_reasons_and_exact_metadata_limits_remain_enforced() {
    let (trusted, mut r) = declared_result();
    r.reasons.push(reason(vec![Subject::Item(id("w1"))]));
    let mut effect = reason(vec![Subject::Dimension(Dimension::EnergyStability)]);
    effect.id = id("effect");
    effect.causes = vec![id("cause")];
    r.reasons.push(effect);
    r.allocations[0].teams[0].builds[0]
        .reasons
        .push(id("effect"));
    r.validate_for(&trusted).unwrap();
    for variant in 0..4 {
        let mut wrong = r.clone();
        match variant {
            0 => wrong.reasons.swap(0, 1),
            1 => wrong.reasons[1].id = id("cause"),
            2 => wrong.reasons[1].causes.push(id("absent")),
            _ => wrong.allocations[0].constraints.push(ConstraintOutcome {
                rule: id("rule"),
                hard: false,
                satisfied: false,
                reasons: vec![id("absent")],
            }),
        }
        assert_eq!(
            wrong.validate_for(&trusted).unwrap_err().code,
            IssueCode::InvalidReference
        );
    }
    r.reasons = (0..1024)
        .map(|n| {
            let mut reason = reason(vec![]);
            reason.id = id(&format!("reason-{n}"));
            reason
        })
        .collect();
    r.allocations[0].teams[0].builds[0].reasons.clear();
    r.uncertainty = vec![uncertainty(Subject::Item(id("w1"))); 128];
    r.validate_for(&trusted).unwrap();
    let mut wrong = r.clone();
    wrong.reasons.push(reason(vec![]));
    assert_eq!(
        wrong.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    r.uncertainty.push(uncertainty(Subject::Item(id("w1"))));
    assert_eq!(
        r.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    let mut r = result(trusted.snapshot());
    r.spec.search.max_results = 128;
    r.allocations = (0..128)
        .map(|n| {
            let mut a = allocation();
            a.id = id(&format!("allocation-{n:03}"));
            a
        })
        .collect();
    rebind(&mut r, trusted.snapshot());
    r.validate_for(&trusted).unwrap();
    let mut extra = allocation();
    extra.id = id("allocation-128");
    r.allocations.push(extra);
    assert_eq!(
        r.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    r.allocations.truncate(2);
    r.spec.search.max_results = 1;
    rebind(&mut r, trusted.snapshot());
    assert_eq!(
        r.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    r.allocations.truncate(1);
    r.validate_for(&trusted).unwrap();
    let (trusted, mut c) = comparison_case(false);
    c.reasons = (0..1024)
        .map(|n| {
            let mut reason = reason(vec![]);
            reason.id = id(&format!("reason-{n}"));
            reason
        })
        .collect();
    c.validate_for(&trusted).unwrap();
    c.reasons.push(reason(vec![]));
    assert_eq!(
        c.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
}

fn padding(additional: usize) -> BTreeMap<String, Parameter> {
    let mut map = BTreeMap::new();
    let mut remaining = additional;
    let mut n = 0;
    while remaining > 4096 {
        let key = format!("padding-{n}");
        let value = Parameter::Text("x".repeat(4000));
        let entry = BTreeMap::from([(key.clone(), value.clone())]);
        let cost = wire::encode(&entry).unwrap().len() - 2 + usize::from(!map.is_empty());
        map.insert(key, value);
        remaining -= cost;
        n += 1;
    }
    let key = "last".to_owned();
    let empty = BTreeMap::from([(key.clone(), Parameter::Text(String::new()))]);
    let overhead = wire::encode(&empty).unwrap().len() - 2 + usize::from(!map.is_empty());
    map.insert(key, Parameter::Text("x".repeat(remaining - overhead)));
    map
}

#[test]
fn decisions_enforce_exactly_sixteen_mib_including_inside_comparisons() {
    let (trusted, mut r) = declared_result();
    r.reasons
        .push(reason(vec![Subject::Account(id("account-1"))]));
    r.reasons[0].parameters = padding(16 * 1024 * 1024 - wire::encode(&r).unwrap().len());
    assert_eq!(wire::encode(&r).unwrap().len(), 16 * 1024 * 1024);
    r.validate_for(&trusted).unwrap();
    if let Parameter::Text(text) = r.reasons[0].parameters.get_mut("last").unwrap() {
        text.push('x');
    }
    assert_eq!(
        r.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    for before in [true, false] {
        let (trusted, mut c) = comparison_case(false);
        let nested = if before { &mut c.before } else { &mut c.after };
        nested
            .reasons
            .push(reason(vec![Subject::Account(id("account-1"))]));
        nested.reasons[0].parameters =
            padding(16 * 1024 * 1024 - wire::encode(nested).unwrap().len());
        c.validate_for(&trusted).unwrap();
        let nested = if before { &mut c.before } else { &mut c.after };
        if let Parameter::Text(text) = nested.reasons[0].parameters.get_mut("last").unwrap() {
            text.push('x');
        }
        assert_eq!(
            c.validate_for(&trusted).unwrap_err().code,
            IssueCode::ResourceLimit
        );
    }
}

#[test]
fn comparison_enforces_exactly_the_total_wire_budget_without_lowering_decision_allowances() {
    let (trusted, mut c) = comparison_case(false);
    for nested in [&mut c.before, &mut c.after] {
        nested
            .reasons
            .push(reason(vec![Subject::Account(id("account-1"))]));
        nested.reasons[0].parameters =
            padding(9 * 1024 * 1024 - wire::encode(nested).unwrap().len());
    }
    assert!(wire::encode(&c).unwrap().len() > 16 * 1024 * 1024);
    c.validate_for(&trusted).unwrap();
    c.reasons.push(reason(vec![scoped(
        ComparisonSide::Before,
        Subject::Item(id("w1")),
    )]));
    c.reasons[0].parameters = padding(128 * 1024 * 1024 - wire::encode(&c).unwrap().len());
    assert_eq!(wire::encode(&c).unwrap().len(), 128 * 1024 * 1024);
    c.validate_for(&trusted).unwrap();
    if let Parameter::Text(text) = c.reasons[0].parameters.get_mut("last").unwrap() {
        text.push('x');
    }
    assert_eq!(
        c.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
}

#[test]
fn sensitivity_enforces_exactly_the_total_wire_budget_and_nested_decision_budgets() {
    let (trusted, mut baseline) = declared_result();
    baseline
        .reasons
        .push(reason(vec![Subject::Account(id("account-1"))]));
    baseline.reasons[0].parameters =
        padding(14 * 1024 * 1024 - wire::encode(&baseline).unwrap().len());
    let mut sensitivity = SensitivityResult {
        scenarios: (0..8)
            .map(|n| ScenarioResult {
                scenario: SensitivityScenario {
                    id: id(&format!("budget-{n}")),
                    label: format!("Unchanged budget control {n}"),
                    assumptions: baseline.spec.assumptions.clone(),
                    encounters: baseline.spec.encounters.clone(),
                },
                result: baseline.clone(),
                ranking_changed: false,
                global_delta: Metric::Available {
                    value: Fixed::ZERO,
                    fidelity: Fidelity::ExactUnderModel,
                    provenance: vec![],
                },
            })
            .collect(),
        baseline,
    };
    let remaining = 128 * 1024 * 1024 - wire::encode(&sensitivity).unwrap().len();
    let last = &mut sensitivity.scenarios.last_mut().unwrap().result;
    let additional = wire::encode(&last.reasons[0].parameters).unwrap().len() - 2;
    last.reasons[0].parameters = padding(additional + remaining);
    assert_eq!(wire::encode(&sensitivity).unwrap().len(), 128 * 1024 * 1024);
    sensitivity.validate_for(&trusted).unwrap();
    let last = &mut sensitivity.scenarios.last_mut().unwrap().result;
    if let Parameter::Text(text) = last.reasons[0].parameters.get_mut("last").unwrap() {
        text.push('x');
    }
    last.validate_for(&trusted).unwrap();
    assert_eq!(
        sensitivity.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    sensitivity.scenarios.truncate(1);
    sensitivity.validate_for(&trusted).unwrap();
    sensitivity.baseline.reasons[0].parameters = padding(
        16 * 1024 * 1024 + 1
            - (wire::encode(&sensitivity.baseline).unwrap().len()
                - wire::encode(&sensitivity.baseline.reasons[0].parameters)
                    .unwrap()
                    .len()
                + 2),
    );
    assert_eq!(
        sensitivity.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
    sensitivity.baseline = sensitivity.scenarios[0].result.clone();
    let nested = &mut sensitivity.scenarios[0].result;
    let base = wire::encode(nested).unwrap().len()
        - wire::encode(&nested.reasons[0].parameters).unwrap().len()
        + 2;
    nested.reasons[0].parameters = padding(16 * 1024 * 1024 + 1 - base);
    assert_eq!(
        sensitivity.validate_for(&trusted).unwrap_err().code,
        IssueCode::ResourceLimit
    );
}

#[test]
fn malformed_subject_shapes_cannot_hide_ambiguous_scopes() {
    for raw in [
        r#"{"type":"team","id":"team-1"}"#,
        r#"{"type":"team","allocation":"allocation-1"}"#,
        r#"{"type":"assumption","id":"global"}"#,
        r#"{"type":"assumption","id":{"type":"encounter","index":65536,"key":"x"}}"#,
    ] {
        assert!(wire::decode::<Subject>(raw).is_err(), "{raw}");
    }
    assert!(wire::decode::<ComparisonSubject>(r#"{"subject":{"type":"item","id":"w1"}}"#).is_err());
    let reference = scoped(ComparisonSide::After, team("allocation-1", "team-1"));
    assert_eq!(
        wire::decode::<ComparisonSubject>(&wire::encode(&reference).unwrap()).unwrap(),
        reference
    );
}
