use super::*;

pub(crate) fn balance(definition: &str, quantity: MaterialQuantity) -> MaterialBalance {
    MaterialBalance {
        definition: id(definition),
        quantity,
        source: None,
        provenance: vec![],
    }
}
fn known(amount: u64) -> MaterialQuantity {
    MaterialQuantity::Known {
        amount: Counter(amount),
    }
}
fn imported(materials: Section<MaterialBalance>) -> ValidatedAccount {
    let mut a = account();
    a.materials = materials;
    ValidatedAccount::imported(a).unwrap()
}
fn source_account() -> AccountSnapshot {
    let mut a = account();
    let utf8 = "{\r\n\"source\":\"Synthetic exporter\",\"materials\":{\"syn:ore\":7}}\r\n";
    let sha256 = Digest::of(utf8.as_bytes());
    a.source_archive = Some(Arc::new(SourceArchive {
        format: SourceFormat::Good {
            dialect_version: 3,
            profile: ImportProfileVersion,
        },
        utf8: utf8.into(),
        sha256: sha256.clone(),
    }));
    let mut ore = balance("syn:ore", known(7));
    ore.source = Some(SourceRef {
        archive_digest: sha256,
        pointer: "/materials/syn:ore".into(),
    });
    a.materials = supplied(vec![ore]);
    a
}

#[test]
fn material_knowledge_is_not_inferred_from_empty_records_or_good_presence() {
    let cases = [
        (
            Section::Omitted {},
            MaterialQuantity::Unknown {},
            MaterialQuantity::Unknown {},
        ),
        (
            supplied(vec![]),
            MaterialQuantity::Unknown {},
            MaterialQuantity::Unknown {},
        ),
        (
            Section::Supplied {
                completeness: Completeness::DeclaredPartial,
                records: vec![],
            },
            MaterialQuantity::Unknown {},
            MaterialQuantity::Unknown {},
        ),
        (complete(vec![]), known(0), known(0)),
        (
            supplied(vec![balance("syn:ore", MaterialQuantity::Unknown {})]),
            MaterialQuantity::Unknown {},
            MaterialQuantity::Unknown {},
        ),
        (
            supplied(vec![balance("syn:ore", known(0))]),
            known(0),
            MaterialQuantity::Unknown {},
        ),
        (
            supplied(vec![balance("syn:ore", known(7))]),
            known(7),
            MaterialQuantity::Unknown {},
        ),
        (
            complete(vec![balance("syn:ore", MaterialQuantity::Unknown {})]),
            MaterialQuantity::Unknown {},
            known(0),
        ),
    ];
    let mut encodings = BTreeSet::new();
    for (section, expected, absent) in cases {
        encodings.insert(wire::encode(&section).unwrap());
        let a = imported(section);
        assert_eq!(a.material_balance(&id("syn:ore")), expected);
        assert_eq!(a.material_balance(&id("syn:missing")), absent);
    }
    assert_eq!(encodings.len(), 8);
    let a = ValidatedAccount::imported(source_account()).unwrap();
    assert_eq!(a.material_balance(&id("syn:ore")), known(7));
    assert_eq!(
        a.material_balance(&id("syn:missing")),
        MaterialQuantity::Unknown {}
    );
}

#[test]
fn material_count_codec_preserves_full_u64_and_rejects_noncanonical_counts() {
    for (text, amount) in [
        ("0", 0),
        ("9007199254740993", 9_007_199_254_740_993),
        ("18446744073709551615", u64::MAX),
    ] {
        let json = format!(r#"{{"status":"known","amount":"{text}"}}"#);
        assert_eq!(
            wire::decode::<MaterialQuantity>(&json).unwrap(),
            known(amount)
        );
        assert_eq!(wire::encode(&known(amount)).unwrap(), json);
        let a = imported(supplied(vec![balance("syn:ore", known(amount))]));
        let backup = Backup::from_validated(&a).unwrap();
        let restored: Backup = wire::decode(&wire::encode(&backup).unwrap()).unwrap();
        assert_eq!(
            restored
                .restore(None)
                .unwrap()
                .material_balance(&id("syn:ore")),
            known(amount)
        );
    }
    for amount in [
        "-1",
        "-0",
        "1.5",
        "1.0",
        "1e3",
        "+1",
        "01",
        " 1",
        "1 ",
        "",
        "18446744073709551616",
    ] {
        let json = format!(r#"{{"status":"known","amount":"{amount}"}}"#);
        assert!(wire::decode::<MaterialQuantity>(&json).is_err(), "{json}");
    }
    for json in [
        r#"{"status":"known","amount":9007199254740993}"#,
        r#"{"status":"known","amount":18446744073709551615}"#,
        r#"{"status":"known","amount":-1}"#,
        r#"{"status":"known","amount":1.5}"#,
        r#"{"status":"known","amount":18446744073709551616}"#,
        r#"{"status":"unknown","amount":"0"}"#,
        r#"{"status":"known"}"#,
        r#"{"status":"infinite"}"#,
        r#"{"status":"known","amount":"1","amount":"2"}"#,
    ] {
        assert!(wire::decode::<MaterialQuantity>(json).is_err(), "{json}");
    }
}

#[test]
fn mandatory_material_schema_has_no_old_or_missing_field_compatibility() {
    let a = account();
    let mut json = serde_json::to_value(&a).unwrap();
    assert_eq!(json["schema_version"], 2);
    json["schema_version"] = 1.into();
    assert_eq!(
        wire::decode::<AccountSnapshot>(&json.to_string())
            .unwrap_err()
            .code,
        IssueCode::UnsupportedVersion
    );
    json["schema_version"] = 2.into();
    json.as_object_mut().unwrap().remove("materials");
    assert!(wire::decode::<AccountSnapshot>(&json.to_string()).is_err());
    for materials in [
        "null",
        "[]",
        "{}",
        r#"{"coverage":"omitted","records":[]}"#,
        r#"{"coverage":"supplied","records":[]}"#,
        r#"{"coverage":"supplied","completeness":"unspecified","records":[{"id":"ore"}]}"#,
    ] {
        json["materials"] = serde_json::from_str(materials).unwrap();
        assert!(
            wire::decode::<AccountSnapshot>(&json.to_string()).is_err(),
            "{materials}"
        );
    }
    let mut request =
        serde_json::to_value(request("old-protocol", Request::GetCatalog, 0)).unwrap();
    request["protocol_version"] = 1.into();
    assert_eq!(
        wire::decode_request(&request.to_string()).unwrap_err().code,
        IssueCode::UnsupportedVersion
    );
    assert_eq!(u16::from(ProtocolVersion), 2);
    assert_eq!(u16::from(ImportProfileVersion), 1);
    assert_eq!(u16::from(CatalogVersion), 1);
}

#[test]
fn materials_require_unique_bounded_kinds_and_cannot_be_gear_references() {
    let mut a = account();
    a.materials = supplied(vec![
        balance("syn:ore", known(1)),
        balance("syn:ore", known(2)),
    ]);
    assert_eq!(a.validate().unwrap_err().code, IssueCode::DuplicateIdentity);
    assert_eq!(
        Backup {
            account: Arc::new(a.clone()),
            sample: None
        }
        .restore(None)
        .unwrap_err()
        .code,
        IssueCode::DuplicateIdentity
    );
    a.materials = supplied(
        (0..MAX_MATERIAL_KINDS)
            .map(|n| balance(&format!("syn:material/{n}"), known(0)))
            .collect(),
    );
    a.validate().unwrap();
    let backup = Backup::from_validated(&ValidatedAccount::imported(a.clone()).unwrap()).unwrap();
    let restored: Backup = wire::decode(&wire::encode(&backup).unwrap()).unwrap();
    assert_eq!(
        restored
            .restore(None)
            .unwrap()
            .snapshot()
            .materials
            .records()
            .len(),
        16384
    );
    if let Section::Supplied { records, .. } = &mut a.materials {
        records.push(balance("syn:one-more", known(1)));
    }
    assert_eq!(a.validate().unwrap_err().code, IssueCode::ResourceLimit);
    a.materials = supplied(vec![balance("syn:ore", known(1))]);
    a.preferences.protected_items.insert(id("syn:ore"));
    assert_eq!(a.validate().unwrap_err().code, IssueCode::InvalidReference);
    a.preferences.protected_items.clear();
    let movement = ItemMove {
        item: id("syn:ore"),
        recipient: id("c0"),
    };
    assert_eq!(
        a.validate_move(&movement).unwrap_err().code,
        IssueCode::InvalidReference
    );
}

#[test]
fn material_completeness_and_sources_are_explicit_attributable_assertions() {
    let a = source_account();
    let declaration = ImportDeclaration {
        section: InventorySection::Materials,
        completeness: Completeness::DeclaredComplete {
            asserted_by: CompletenessAuthority::Exporter {
                name: "Synthetic exporter".into(),
                source: SourceRef {
                    archive_digest: a.source_archive.as_ref().unwrap().sha256.clone(),
                    pointer: "/source".into(),
                },
            },
        },
    };
    let declared = a
        .clone()
        .with_import_declarations(vec![declaration.clone()])
        .unwrap();
    assert_eq!(
        ValidatedAccount::imported(declared.clone())
            .unwrap()
            .material_balance(&id("syn:missing")),
        known(0)
    );
    let backup =
        Backup::from_validated(&ValidatedAccount::imported(declared.clone()).unwrap()).unwrap();
    let decoded: Backup = wire::decode(&wire::encode(&backup).unwrap()).unwrap();
    assert_eq!(decoded.restore(None).unwrap().snapshot(), &declared);
    assert_eq!(
        a.clone()
            .with_import_declarations(vec![declaration.clone(), declaration.clone()])
            .unwrap_err()
            .code,
        IssueCode::InvalidInput
    );
    assert_eq!(
        account()
            .with_import_declarations(vec![declaration])
            .unwrap_err()
            .code,
        IssueCode::MissingSection
    );
    for asserted_by in [
        CompletenessAuthority::User {
            statement: String::new(),
        },
        CompletenessAuthority::User {
            statement: "x".repeat(1025),
        },
        CompletenessAuthority::Exporter {
            name: String::new(),
            source: SourceRef {
                archive_digest: Digest::of(b"wrong"),
                pointer: "/source".into(),
            },
        },
        CompletenessAuthority::Exporter {
            name: "claimed".into(),
            source: SourceRef {
                archive_digest: Digest::of(b"wrong"),
                pointer: "/source".into(),
            },
        },
    ] {
        assert!(
            a.clone()
                .with_import_declarations(vec![ImportDeclaration {
                    section: InventorySection::Materials,
                    completeness: Completeness::DeclaredComplete { asserted_by }
                }])
                .is_err()
        );
    }
    for pointer in ["materials", "/materials/~", "/materials/~2"] {
        let mut invalid = a.clone();
        if let Section::Supplied { records, .. } = &mut invalid.materials {
            records[0].source.as_mut().unwrap().pointer = pointer.into();
        }
        assert_eq!(
            invalid.validate().unwrap_err().code,
            IssueCode::InvalidReference
        );
    }
    let mut missing = a.clone();
    missing.source_archive = None;
    assert_eq!(
        missing.validate().unwrap_err().code,
        IssueCode::InvalidReference
    );
    let mut mismatch = a.clone();
    if let Section::Supplied { records, .. } = &mut mismatch.materials {
        records[0].source.as_mut().unwrap().archive_digest = Digest::of(b"wrong");
    }
    assert_eq!(
        mismatch.validate().unwrap_err().code,
        IssueCode::InvalidReference
    );
}

#[test]
fn material_knowledge_and_amounts_invalidate_semantics_and_result_keys() {
    let mut digests = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for section in [
        Section::Omitted {},
        supplied(vec![]),
        Section::Supplied {
            completeness: Completeness::DeclaredPartial,
            records: vec![],
        },
        complete(vec![]),
        supplied(vec![balance("syn:ore", MaterialQuantity::Unknown {})]),
        supplied(vec![balance("syn:ore", known(0))]),
        supplied(vec![balance("syn:ore", known(9_007_199_254_740_992))]),
        supplied(vec![balance("syn:ore", known(9_007_199_254_740_993))]),
    ] {
        let mut a = account();
        a.materials = section;
        digests.insert(String::from(wire::account_semantic_digest(&a).unwrap()));
        keys.insert(String::from(
            wire::result_fingerprint(&a, &spec(), "0.1.0", &id("engine-1")).unwrap(),
        ));
    }
    assert_eq!(digests.len(), 8);
    assert_eq!(keys.len(), 8);
    let mut a = source_account();
    if let Section::Supplied { records, .. } = &mut a.materials {
        records.push(balance("syn:dust", MaterialQuantity::Unknown {}));
    }
    let before = wire::account_semantic_digest(&a).unwrap();
    let snapshot = wire::snapshot_digest(&a).unwrap();
    if let Section::Supplied { records, .. } = &mut a.materials {
        records.reverse();
        for record in records {
            record.source = None;
            record.provenance.push(Provenance {
                id: id("synthetic-note"),
                origin: EvidenceOrigin::Synthetic,
                fidelity: Fidelity::Unknown,
                source: "Fabricated balance, not game data".into(),
                applicable_game_version: None,
                license: None,
                limitations: vec![],
            });
        }
    }
    assert_eq!(wire::account_semantic_digest(&a).unwrap(), before);
    assert_ne!(wire::snapshot_digest(&a).unwrap(), snapshot);
    let trusted = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome);
    assert!(trusted.is_err());
    a.source_archive = None;
    let trusted = ValidatedAccount::bundled(a, SampleId::SynWelcome).unwrap();
    let mut forged = Backup::from_validated(&trusted).unwrap();
    Arc::make_mut(&mut forged.account).materials = Section::Omitted {};
    assert_eq!(
        forged.restore(Some(&trusted)).unwrap_err().code,
        IssueCode::SyntheticOnImportedAccount
    );
}

#[test]
fn material_labels_and_preview_counts_use_existing_bounded_public_contracts() {
    use aaru_contract::setup::*;
    let mut catalog = Catalog {
        schema_version: CatalogVersion,
        samples: vec![],
        mechanics: vec![],
        definitions: vec![PublicDefinition {
            id: id("syn:ore"),
            kind: DefinitionKind::Material,
            label: "Synthetic ore".into(),
        }],
    };
    catalog.validate().unwrap();
    let mut a = account();
    let note = Provenance {
        id: id("synthetic-count-note"),
        origin: EvidenceOrigin::Synthetic,
        fidelity: Fidelity::Unknown,
        source: "Fabricated, not verified game data".into(),
        applicable_game_version: None,
        license: None,
        limitations: vec![],
    };
    let mut ore = balance("syn:ore", known(1));
    ore.provenance = vec![note.clone(); 128];
    a.materials = supplied(vec![ore]);
    a.validate().unwrap();
    if let Section::Supplied { records, .. } = &mut a.materials {
        records[0].provenance.push(note);
    }
    assert_eq!(a.validate().unwrap_err().code, IssueCode::ResourceLimit);
    let restored: Catalog = wire::decode(&wire::encode(&catalog).unwrap()).unwrap();
    assert_eq!(restored, catalog);
    catalog.definitions[0].label = "x".repeat(161);
    assert!(catalog.validate().is_err());
    let mut head = PreparedHead {
        binding: ActionBinding {
            authority: TransportAuthority {
                epochs: context().epochs,
                generation: Counter(0),
                action: id("material-preview"),
            },
            step: Counter(0),
            operation: Digest::of(b"preview"),
        },
        token: id("material-token"),
        account: AccountHead {
            snapshot: id("material-account"),
            semantics: Digest::of(b"semantics"),
            backup_digest: Digest::of(b"backup"),
        },
        characters: 0,
        weapons: 0,
        artifacts: 0,
        material_kinds: 16384,
        issues: vec![],
    };
    head.validate().unwrap();
    let frame = ControlFrame {
        version: ProtocolVersion,
        from: Endpoint::UiSupervisor,
        to: Endpoint::Dom,
        authority: head.binding.authority.clone(),
        step: head.binding.step,
        payload: Control::Prepared {
            head: head.clone(),
            backup: BulkDescriptor {
                authority: head.binding.authority.clone(),
                step: head.binding.step,
                attachment: Counter(1),
                purpose: BulkPurpose::PreparedBackup,
                encoding: BulkEncoding::StrictUtf8Json,
                byte_length: Counter(6),
                sha256: head.account.backup_digest.clone(),
                operation: Some(head.binding.operation.clone()),
            },
        },
    };
    assert_eq!(
        decode_control(&encode_control(&frame).unwrap()).unwrap(),
        frame
    );
    let mut old = serde_json::to_value(&frame).unwrap();
    old["version"] = 1.into();
    assert_eq!(
        decode_control(&old.to_string()).unwrap_err().code,
        IssueCode::UnsupportedVersion
    );
    let mut missing = serde_json::to_value(&head).unwrap();
    missing.as_object_mut().unwrap().remove("material_kinds");
    assert!(wire::decode::<PreparedHead>(&missing.to_string()).is_err());
    head.material_kinds = 16385;
    assert!(head.validate().is_err());
}

#[test]
fn unknown_materials_do_not_block_non_cost_current_loadout_or_equipment_checks() {
    for materials in [
        Section::Omitted {},
        supplied(vec![]),
        supplied(vec![balance("syn:ore", MaterialQuantity::Unknown {})]),
    ] {
        let mut a = current_account();
        a.materials = materials;
        let validated = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
        let mut decision = result(&a);
        decision.spec.assignments = AssignmentDomain::CurrentLoadout;
        decision.allocations[0].teams[0].builds[0].weapon = None;
        decision.allocations[0].footprint.items.clear();
        decision.reproducibility.decision_spec =
            wire::decision_spec_digest(&decision.spec).unwrap();
        decision.reproducibility.fingerprint =
            wire::result_fingerprint(&a, &decision.spec, "0.1.0", &id("engine-1")).unwrap();
        decision.validate_for(&validated).unwrap();
        a.validate_move(&ItemMove {
            item: id("w0"),
            recipient: id("c0"),
        })
        .unwrap();
    }
}
