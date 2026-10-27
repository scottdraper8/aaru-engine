use super::*;
use aaru_contract::setup::*;

#[path = "composed.rs"]
mod composed;

fn authority() -> TransportAuthority {
    TransportAuthority {
        epochs: context().epochs,
        generation: Counter(0),
        action: id("action-1"),
    }
}
fn binding(authority: TransportAuthority) -> ActionBinding {
    ActionBinding {
        authority,
        step: Counter(0),
        operation: Digest::of(b"normative operation"),
    }
}
fn descriptor(b: &ActionBinding, purpose: BulkPurpose, bytes: &[u8]) -> BulkDescriptor {
    BulkDescriptor {
        authority: b.authority.clone(),
        step: b.step,
        attachment: Counter(1),
        purpose,
        encoding: match purpose {
            BulkPurpose::SourceDownload => BulkEncoding::PreservedUtf8,
            BulkPurpose::UiCheckpoint => BulkEncoding::PrivateUiState,
            _ => BulkEncoding::StrictUtf8Json,
        },
        byte_length: Counter(bytes.len() as u64),
        sha256: Digest::of(bytes),
        operation: Some(b.operation.clone()),
    }
}
fn checkpoint(
    b: &ActionBinding,
    revision: u64,
    account: Option<Digest>,
    bytes: &[u8],
) -> UiCheckpointHead {
    UiCheckpointHead {
        account_semantics: account,
        revision: Counter(revision),
        schema: id("ui-state-1"),
        payload: descriptor(b, BulkPurpose::UiCheckpoint, bytes),
    }
}
fn restore_dom(
    gate: &mut DomGate,
    b: &ActionBinding,
    account: &Option<AccountHead>,
    checkpoint: &CheckpointGate,
    revision: Counter,
    digest: Option<&Digest>,
) -> ContractResult<Option<ActionIntent>> {
    gate.restored(&RestorationAck {
        binding: b.clone(),
        account: account.clone(),
        generation: b.authority.generation.next().unwrap(),
        checkpoint: checkpoint.accepted().map(|h| {
            let mut commitment = h.commitment();
            commitment.revision = revision;
            if let Some(digest) = digest {
                commitment.sha256 = digest.clone();
            }
            commitment
        }),
    })
}
fn ready_dom(storage: HeadObservation, account: Option<AccountHead>) -> DomGate {
    let mut gate =
        DomGate::new(context().epochs, Counter(0), storage, account.clone(), None).unwrap();
    let bootstrap = binding(gate.begin_bootstrap(id("bootstrap")).unwrap());
    gate.bind(bootstrap.clone()).unwrap();
    let state = CheckpointGate::new(account.as_ref().map(|h| h.semantics.clone()), None).unwrap();
    restore_dom(&mut gate, &bootstrap, &account, &state, Counter(0), None).unwrap();
    gate
}
fn old_head() -> DurableHead {
    DurableHead::from_record(&durable_account()).unwrap()
}
fn saving_case() -> (DomGate, PreparedHead, StorageReservation) {
    let old = old_head();
    let mut gate = ready_dom(HeadObservation::Readable { head: old.clone() }, None);
    let b = binding(gate.start(id("import")).unwrap());
    gate.bind(b.clone()).unwrap();
    let bytes = b"new immutable record bytes";
    let account = AccountHead {
        snapshot: id("new-account"),
        semantics: Digest::of(b"new semantics"),
        backup_digest: Digest::of(b"new backup"),
    };
    let prepared = PreparedHead {
        binding: b.clone(),
        token: id("candidate"),
        account: account.clone(),
        characters: 8,
        weapons: 2,
        artifacts: 0,
        issues: vec![],
    };
    gate.prepared(prepared.clone()).unwrap();
    stage_for(&mut gate, &prepared);
    let reservation = StorageReservation {
        binding: b.clone(),
        token: prepared.token.clone(),
        expected: old,
        proposed: DurableHead {
            revision: Counter(11),
            record_digest: Digest::of(bytes),
            content: DurableContent::Account { account },
        },
        record: descriptor(&b, BulkPurpose::DurableRecord, bytes),
    };
    (gate, prepared, reservation)
}

fn stage_for(gate: &mut DomGate, prepared: &PreparedHead) -> UiCheckpointHead {
    let checkpoint = checkpoint(
        &prepared.binding,
        gate.checkpoint().map_or(1, |h| h.revision.0 + 1),
        Some(prepared.account.semantics.clone()),
        b"normative retained target UI state",
    );
    let policy = if gate
        .intended()
        .is_some_and(|a| a.snapshot == prepared.account.snapshot)
    {
        CheckpointPolicy::PreserveAccepted
    } else {
        CheckpointPolicy::ConfirmedReplacement {
            confirmation: AccountChangeConfirmation {
                binding: prepared.binding.clone(),
                previous: gate.intended().cloned(),
                target: prepared.account.clone(),
                checkpoint: checkpoint.commitment(),
                settings: ReplacementSettings::CarryAcceptedSettings,
            }
            .into(),
        }
    };
    gate.stage_account_transition(StagedAccountTransition {
        previous_checkpoint: gate.checkpoint().map(UiCheckpointHead::commitment),
        checkpoint: checkpoint.clone(),
        policy,
    })
    .unwrap();
    checkpoint
}

#[test]
fn compact_transport_rejects_wrong_routes_attachments_duplicates_and_oversized_pages() {
    let b = binding(authority());
    let offer = SourceOffer {
        authority: b.authority.clone(),
        step: b.step,
        attachment: Counter(7),
        format: ImportFormat::Good,
        byte_length: Counter(16),
    };
    let mut frame = ControlFrame {
        version: ProtocolVersion,
        from: Endpoint::Dom,
        to: Endpoint::UiSupervisor,
        authority: b.authority.clone(),
        step: b.step,
        payload: Control::Offer {
            offer: offer.clone(),
        },
    };
    let text = encode_control(&frame).unwrap();
    assert_eq!(decode_control(&text).unwrap(), frame);
    frame
        .check_attachment(Some(AttachmentRequirement::File {
            id: Counter(7),
            bytes: Counter(16),
        }))
        .unwrap();
    for actual in [
        None,
        Some(AttachmentRequirement::Blob {
            id: Counter(7),
            bytes: Counter(16),
        }),
        Some(AttachmentRequirement::File {
            id: Counter(8),
            bytes: Counter(16),
        }),
        Some(AttachmentRequirement::File {
            id: Counter(7),
            bytes: Counter(15),
        }),
    ] {
        assert!(frame.check_attachment(actual).is_err());
    }
    frame.to = Endpoint::Engine;
    assert!(encode_control(&frame).is_err());
    frame.to = Endpoint::UiSupervisor;
    frame.step = Counter(1);
    assert!(encode_control(&frame).is_err());
    let duplicate = text.replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
    assert_eq!(
        decode_control(&duplicate).unwrap_err().code,
        IssueCode::DuplicateProperty
    );
    assert!(decode_control(&" ".repeat(MAX_CONTROL_BYTES + 1)).is_err());
    let page = PresentationPage {
        binding: b.clone(),
        view_digest: Digest::of(b"view"),
        page: Counter(0),
        total_pages: Counter(1),
        search: Some(SearchStatus::Complete),
        frontier: Some(FrontierPresentation::Exhaustive),
        rows: (0..24)
            .map(|_| PresentationRow::Issue {
                code: IssueCode::InvalidInput,
                message: "x".repeat(1000),
            })
            .collect(),
    };
    frame.step = b.step;
    frame.from = Endpoint::UiSupervisor;
    frame.to = Endpoint::Dom;
    frame.payload = Control::Page { page };
    assert!(encode_control(&frame).is_err());
    frame.payload = Control::Bootstrap {
        role: WorkerRole::Engine,
        port: Counter(3),
        active: None,
        checkpoint: None,
        storage: HeadObservation::Unavailable {
            last_readable: None,
        },
    };
    frame.from = Endpoint::Dom;
    frame.to = Endpoint::Engine;
    frame
        .check_attachment(Some(AttachmentRequirement::Port { id: Counter(3) }))
        .unwrap();
    assert!(
        frame
            .check_attachment(Some(AttachmentRequirement::Port { id: Counter(4) }))
            .is_err()
    );
}

#[test]
fn verified_bulk_checks_exact_bytes_and_full_authority_but_offers_are_not_trusted_bulk() {
    let b = binding(authority());
    let bytes = b"{\"large_integer\":9007199254740993}\r\n";
    let d = descriptor(&b, BulkPurpose::LogicalRequest, bytes);
    d.verify_bytes(&b, BulkPurpose::LogicalRequest, bytes)
        .unwrap();
    let mut variants = vec![];
    let mut changed = d.clone();
    changed.authority.epochs.worker = Counter(1);
    variants.push(changed);
    let mut changed = d.clone();
    changed.authority.epochs.reset = Counter(1);
    variants.push(changed);
    let mut changed = d.clone();
    changed.authority.generation = Counter(1);
    variants.push(changed);
    let mut changed = d.clone();
    changed.authority.action = id("another-action");
    variants.push(changed);
    let mut changed = d.clone();
    changed.step = Counter(1);
    variants.push(changed);
    let mut changed = d.clone();
    changed.operation = Some(Digest::of(b"changed"));
    variants.push(changed);
    let mut changed = d.clone();
    changed.byte_length = Counter(bytes.len() as u64 - 1);
    variants.push(changed);
    let mut changed = d.clone();
    changed.sha256 = Digest::of(b"wrong");
    variants.push(changed);
    let mut changed = d.clone();
    changed.purpose = BulkPurpose::LogicalEvent;
    variants.push(changed);
    for changed in variants {
        assert!(
            changed
                .verify_bytes(&b, BulkPurpose::LogicalRequest, bytes)
                .is_err()
        );
    }
    assert!(
        d.verify_bytes(&b, BulkPurpose::LogicalRequest, b"{}")
            .is_err()
    );
    let mut recovery = descriptor(&b, BulkPurpose::RecoveryRecord, bytes);
    recovery.operation = None;
    recovery
        .verify_anchor(
            &b.authority,
            b.step,
            BulkPurpose::RecoveryRecord,
            &Digest::of(bytes),
            bytes,
        )
        .unwrap();
    assert!(
        recovery
            .verify_anchor(
                &b.authority,
                b.step,
                BulkPurpose::RecoveryBackup,
                &Digest::of(bytes),
                bytes
            )
            .is_err()
    );
    let malformed = descriptor(&b, BulkPurpose::LogicalRequest, &[0xff]);
    assert!(
        malformed
            .verify_bytes(&b, BulkPurpose::LogicalRequest, &[0xff])
            .is_err()
    );
    let mut gate = ready_dom(
        HeadObservation::Unavailable {
            last_readable: None,
        },
        None,
    );
    let a = gate.start(id("source")).unwrap();
    let offer = SourceOffer {
        authority: a.clone(),
        step: Counter(0),
        attachment: Counter(0),
        format: ImportFormat::Good,
        byte_length: Counter(MAX_INPUT_BYTES as u64),
    };
    gate.admit_offer(&offer).unwrap();
    assert!(gate.admit_offer(&offer).is_err());
    let bound = binding(a);
    assert!(
        gate.admit_attachment(&descriptor(&bound, BulkPurpose::LogicalRequest, b"{}"))
            .is_err()
    );
    gate.bind(bound.clone()).unwrap();
    let verified = descriptor(&bound, BulkPurpose::LogicalRequest, b"{}");
    gate.admit_attachment(&verified).unwrap();
    assert!(gate.admit_attachment(&verified).is_err());
    let next = gate.next_step(&bound).unwrap();
    assert_eq!(next, Counter(1));
    assert!(gate.bind(bound).is_err());
    assert!(gate.admit_attachment(&verified).is_err());
}

#[test]
fn prebinding_cancel_invalidates_both_siblings_without_a_fingerprint_and_bounds_replacement() {
    let mut gate = ready_dom(
        HeadObservation::Unavailable {
            last_readable: None,
        },
        None,
    );
    let abandoned = gate.start(id("hash-not-yet-computed")).unwrap();
    let plan = gate.cancel().unwrap();
    assert_eq!(plan.workers, [WorkerRole::UiSupervisor, WorkerRole::Engine]);
    assert!(plan.close_private_ports);
    assert_eq!(plan.next_epochs.worker, Counter(1));
    assert!(!gate.admits(&abandoned, Counter(0)));
    assert!(gate.bind(binding(abandoned)).is_err());
    assert!(
        gate.replace_after_cancel(ActionIntent::Evaluate)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        gate.replace_after_cancel(ActionIntent::Compare).unwrap(),
        Some(ActionIntent::Evaluate)
    );
    assert!(gate.start(id("too-early")).is_err());
    let boot = binding(gate.begin_bootstrap(id("replacement-bootstrap")).unwrap());
    assert!(gate.begin_bootstrap(id("duplicate-bootstrap")).is_err());
    gate.bind(boot.clone()).unwrap();
    let checkpoint = CheckpointGate::new(None, None).unwrap();
    assert_eq!(
        restore_dom(&mut gate, &boot, &None, &checkpoint, Counter(0), None).unwrap(),
        Some(ActionIntent::Compare)
    );
    gate.start(id("new-action")).unwrap();
}

#[test]
fn reservation_wins_cancel_races_only_after_prewrite_and_only_transaction_complete_promotes() {
    let (mut cancelled, _, stale) = saving_case();
    cancelled.cancel().unwrap();
    assert!(cancelled.reserve(stale).is_err());
    let (mut gate, prepared, reservation) = saving_case();
    let mut bad = reservation.clone();
    bad.expected.revision = Counter(9);
    assert!(gate.reserve(bad).is_err());
    let mut bad = reservation.clone();
    bad.record.sha256 = Digest::of(b"different record");
    assert!(gate.reserve(bad).is_err());
    gate.reserve(reservation.clone()).unwrap();
    assert_eq!(gate.phase(), &DomPhase::Reserved);
    assert!(gate.cancel().is_err());
    assert!(gate.crash_after_settlement().is_err());
    assert!(gate.start(id("replacement")).is_err());
    assert_eq!(gate.intended(), None);
    let mut stale_binding = reservation.binding.clone();
    stale_binding.authority.epochs.worker = Counter(8);
    assert!(gate.transaction_complete(&stale_binding).is_err());
    gate.transaction_aborted(&reservation.binding).unwrap();
    assert_eq!(gate.phase(), &DomPhase::Bulk);
    assert_eq!(gate.intended(), None);
    gate.reserve(reservation.clone()).unwrap();
    let receipt = gate.transaction_complete(&reservation.binding).unwrap();
    assert_eq!(receipt, reservation.proposed.receipt(context().epochs));
    assert_eq!(gate.intended(), Some(&prepared.account));
    assert!(gate.cancel().is_err());
    assert!(
        gate.activation_acknowledged(&RestorationAck {
            binding: reservation.binding.clone(),
            account: Some(prepared.account.clone()),
            generation: Counter(1),
            checkpoint: gate.checkpoint().map(UiCheckpointHead::commitment),
        })
        .is_err()
    );
    let mut activation = reservation.binding.clone();
    activation.step = Counter(1);
    activation.operation = Digest::of(b"activate logical request");
    gate.bind_activation(activation.clone()).unwrap();
    gate.activation_acknowledged(&RestorationAck {
        binding: activation,
        account: Some(prepared.account),
        generation: Counter(2),
        checkpoint: gate.checkpoint().map(UiCheckpointHead::commitment),
    })
    .unwrap();
    assert_eq!(gate.phase(), &DomPhase::Idle);
}

#[test]
fn logical_save_proposal_remains_cancellable_and_restart_drops_candidate_without_claiming_write() {
    let trusted = ValidatedAccount::imported(account()).unwrap();
    let p = preview(&trusted);
    let mut gate = SessionGate::new(context(), Counter(0));
    let import = gate_request(
        &gate,
        "prepare",
        Request::PrepareImport {
            input: ImportInput::Restore {
                backup: p.backup.clone(),
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
    let mut machine =
        CommitMachine::new(context().epochs, true, observed(durable_account())).unwrap();
    let record = gate.propose_prepared_save(&mut machine, &p.token).unwrap();
    assert_eq!(machine.phase(), &CommitPhase::Prepared);
    assert!(
        gate.storage_committed(&mut machine, &receipt(&record, &context().epochs))
            .is_err()
    );
    gate.cancel_prepared_save(&mut machine).unwrap();
    assert_eq!(machine.phase(), &CommitPhase::Idle);
    assert_eq!(machine.durable().unwrap().revision, Counter(10));
    assert!(
        gate.reserve_prepared_save(&mut machine, &old_head())
            .is_err()
    );
    gate.restart_worker(&machine).unwrap();
}

#[test]
fn session_anchor_commits_before_ack_and_survives_restart_over_an_older_durable_account() {
    let old = durable_account();
    let old_backup = old.backup.clone().unwrap();
    let mut new = old_backup.clone();
    Arc::make_mut(&mut Arc::make_mut(&mut new).account).id = id("explicit-session-account");
    let mut machine = CommitMachine::new(context().epochs, true, observed(old.clone())).unwrap();
    machine
        .prepare(Some(new.clone()), &context().epochs)
        .unwrap();
    machine.promote_session_only().unwrap();
    assert_eq!(machine.phase(), &CommitPhase::ActivatingSession);
    assert!(machine.prepare(None, &context().epochs).is_err());
    assert_eq!(machine.active(), Some(&new));
    assert_eq!(
        machine.durable_head().unwrap(),
        Some(DurableHead::from_record(&old).unwrap())
    );
    let retained = machine.recovery_input();
    assert_eq!(
        retained.active,
        ActiveSelection::RetainedSession {
            backup: new.clone()
        }
    );
    let mut restarted = context().epochs;
    restarted.worker = Counter(1);
    assert!(
        machine
            .bootstrap(restarted.clone(), observed(old.clone()))
            .is_err()
    );
    machine.bootstrap(restarted, retained.clone()).unwrap();
    assert_eq!(machine.active(), Some(&new));
    assert_eq!(
        retained.restore(None).unwrap().unwrap().snapshot().id,
        id("explicit-session-account")
    );
    let fresh_page = CommitMachine::new(context().epochs, true, observed(old)).unwrap();
    assert_eq!(fresh_page.active(), Some(&old_backup));
    let sample = ValidatedAccount::bundled(account(), SampleId::SynWelcome).unwrap();
    let mut sample_input = retained;
    sample_input.active = ActiveSelection::RetainedSession {
        backup: Backup::from_validated(&sample).unwrap().into(),
    };
    assert!(sample_input.restore(None).is_err());
    sample_input.restore(Some(&sample)).unwrap();
}

#[test]
fn checkpoint_acknowledgements_preserve_independent_accepted_edits_and_apply_backpressure() {
    let account = Some(Digest::of(b"account"));
    let b = binding(authority());
    let mut checkpoints = CheckpointGate::new(account.clone(), None).unwrap();
    checkpoints
        .offer_edit(
            b.clone(),
            Counter(0),
            FormEdit::ObjectiveWeight {
                dimension: Dimension::EnergyStability,
                value: Fixed::ONE,
            },
        )
        .unwrap();
    assert!(
        checkpoints
            .offer_edit(
                b.clone(),
                Counter(0),
                FormEdit::EncounterTargets {
                    encounter: 0,
                    count: 2
                }
            )
            .is_err()
    );
    let first = checkpoint(&b, 1, account.clone(), b"opaque accepted objective");
    let mut wrong = first.clone();
    wrong.revision = Counter(2);
    assert!(checkpoints.retain_checkpoint(wrong).is_err());
    let mut wrong = first.clone();
    wrong.payload.authority.epochs.worker = Counter(1);
    assert!(checkpoints.retain_checkpoint(wrong).is_err());
    assert_eq!(
        checkpoints.retain_checkpoint(first.clone()).unwrap(),
        Counter(1)
    );
    assert!(checkpoints.retain_checkpoint(first.clone()).is_err());
    checkpoints
        .offer_edit(
            b.clone(),
            Counter(1),
            FormEdit::EncounterTargets {
                encounter: 0,
                count: 2,
            },
        )
        .unwrap();
    let second = checkpoint(
        &b,
        2,
        account.clone(),
        b"opaque state containing objective AND target",
    );
    assert_eq!(
        checkpoints.retain_checkpoint(second.clone()).unwrap(),
        Counter(2)
    );
    checkpoints
        .offer_edit(
            b.clone(),
            Counter(2),
            FormEdit::MoveRecipient {
                item: id("w0"),
                recipient: id("c2"),
            },
        )
        .unwrap();
    checkpoints.abandon_unaccepted_edit();
    assert_eq!(checkpoints.accepted(), Some(&second));
    checkpoints
        .restore_acknowledged(&account, Counter(2), Some(&second.payload.sha256))
        .unwrap();
    assert!(
        checkpoints
            .restore_acknowledged(&account, Counter(1), Some(&first.payload.sha256))
            .is_err()
    );
    assert!(
        checkpoints
            .restore_acknowledged(
                &Some(Digest::of(b"other")),
                Counter(2),
                Some(&second.payload.sha256)
            )
            .is_err()
    );
    let head = AccountHead {
        snapshot: id("active"),
        semantics: account.clone().unwrap(),
        backup_digest: Digest::of(b"backup"),
    };
    let mut gate = DomGate::new(
        context().epochs,
        Counter(1),
        HeadObservation::Unavailable {
            last_readable: None,
        },
        Some(head.clone()),
        Some(second.clone()),
    )
    .unwrap();
    let initial = binding(gate.begin_bootstrap(id("initial-settings")).unwrap());
    gate.bind(initial.clone()).unwrap();
    restore_dom(
        &mut gate,
        &initial,
        &Some(head.clone()),
        &checkpoints,
        Counter(2),
        Some(&second.payload.sha256),
    )
    .unwrap();
    gate.start(id("cancel-with-settings")).unwrap();
    gate.cancel().unwrap();
    let boot = binding(gate.begin_bootstrap(id("restore-settings")).unwrap());
    gate.bind(boot.clone()).unwrap();
    assert!(
        restore_dom(
            &mut gate,
            &boot,
            &Some(head.clone()),
            &checkpoints,
            Counter(0),
            None
        )
        .is_err()
    );
    restore_dom(
        &mut gate,
        &boot,
        &Some(head),
        &checkpoints,
        Counter(2),
        Some(&second.payload.sha256),
    )
    .unwrap();
}

#[test]
fn corrupt_payload_or_unreadable_head_can_be_privacy_cleared_without_decoding_old_backup() {
    for readable_head in [None, Some(old_head())] {
        let observation = match &readable_head {
            Some(head) => HeadObservation::Corrupt {
                readable_head: head.clone(),
            },
            None => HeadObservation::Unreadable {
                last_known_head: None,
            },
        };
        let mut gate = ready_dom(observation.clone(), None);
        let a = gate.begin_clear(id("privacy-clear")).unwrap();
        assert_eq!(a.epochs.reset, Counter(1));
        let b = binding(a.clone());
        gate.bind(b.clone()).unwrap();
        let plan = gate
            .reserve_privacy_clear(&b, id("replacement-store"))
            .unwrap();
        assert!(gate.cancel().is_err());
        assert!(gate.crash_after_settlement().is_err());
        let mut clear =
            PrivacyClearMachine::reserve(a, observation, id("replacement-store")).unwrap();
        assert!(!clear.erased());
        assert!(clear.cleared_bootstrap().is_err());
        match &plan {
            ClearPlan::CompareHead {
                expected,
                tombstone,
            } => {
                assert_eq!(expected, &old_head());
                assert_eq!(tombstone.revision, Counter(11));
            }
            ClearPlan::Reinitialize {
                new_store,
                tombstone,
            } => {
                assert_eq!(new_store, &id("replacement-store"));
                assert_eq!(tombstone.revision, Counter(1));
            }
        }
        let c = clear.authority().clone();
        clear.transaction_complete(&c, &plan).unwrap();
        assert!(clear.erased());
        let receipt = gate.privacy_clear_complete(&b, &plan).unwrap();
        assert_eq!(receipt.backup_digest, Digest::of(b"aaru-reset-v1"));
        let mut machine = CommitMachine::new(
            context().epochs,
            true,
            RecoveryInput {
                storage: match readable_head {
                    Some(head) => StorageObservation::Corrupt { head },
                    None => StorageObservation::Unreadable {
                        last_known_head: None,
                    },
                },
                active: ActiveSelection::EmptySession,
            },
        )
        .unwrap();
        machine.apply_privacy_clear(&clear).unwrap();
        let mut session = SessionGate::new(context(), Counter(1));
        session.install_privacy_clear(&clear).unwrap();
        let input = machine.recovery_input();
        let bootstrap = gate_request(
            &session,
            "cleared-bootstrap",
            Request::Bootstrap {
                input: input.clone(),
            },
        );
        session.register(&bootstrap).unwrap();
        machine
            .bootstrap(session.context().epochs.clone(), input)
            .unwrap();
        assert!(session.accept(&reply(
            &bootstrap,
            Event::ResetAcknowledged {
                previous_generation: Counter(1)
            },
            Counter(2)
        )));
        assert_eq!(machine.active(), None);
        gate.crash_after_settlement().unwrap();
        assert_eq!(gate.intended(), None);
    }
    assert!(
        PrivacyClearMachine::reserve(
            authority(),
            HeadObservation::Unavailable {
                last_readable: Some(old_head())
            },
            id("new-store")
        )
        .is_err()
    );
    let mut a = authority();
    a.epochs.reset = Counter(1);
    let mut aborted = PrivacyClearMachine::reserve(
        a.clone(),
        HeadObservation::Unreadable {
            last_known_head: None,
        },
        id("new-store"),
    )
    .unwrap();
    aborted.transaction_aborted(&a).unwrap();
    assert!(!aborted.erased());
    assert!(
        aborted
            .transaction_complete(&a, &aborted.plan().clone())
            .is_err()
    );
    let mut machine =
        CommitMachine::new(context().epochs, true, observed(durable_account())).unwrap();
    let regressed = DurableRecord {
        revision: Counter(1),
        backup: None,
    };
    assert!(
        machine
            .bootstrap(context().epochs, observed(regressed))
            .is_err()
    );
    assert!(machine.apply_privacy_clear(&aborted).is_err());
}

fn catalog() -> Catalog {
    let spec = spec();
    let parameters = vec![
        EditableParameter {
            target: ParameterTarget::TargetCount { index: 0 },
            label: "Number of targets".into(),
            explanation: "Explicit illustrative encounter count, not verified combat data".into(),
            fidelity: Fidelity::Unknown,
            provenance: vec![],
            domain: ParameterDomain::Count {
                minimum: 1,
                maximum: 8,
            },
        },
        EditableParameter {
            target: ParameterTarget::ObjectiveWeight {
                dimension: Dimension::OutputPotential,
            },
            label: "Comparative output weight".into(),
            explanation: "Explicit comparative objective, not DPS".into(),
            fidelity: Fidelity::ExactUnderModel,
            provenance: vec![],
            domain: ParameterDomain::Quantity {
                unit: QuantityUnit::Ratio,
                minimum: Fixed::ZERO,
                maximum: Fixed::ONE,
            },
        },
    ];
    Catalog {
        schema_version: CatalogVersion,
        samples: vec![],
        definitions: vec![PublicDefinition {
            id: id("syn:encounter/one"),
            kind: DefinitionKind::Encounter,
            label: "Illustrative target".into(),
        }],
        mechanics: vec![MechanicsEntry {
            mechanics: spec.mechanics.clone(),
            normalization: spec.normalization.clone(),
            presets: vec![DecisionPreset {
                id: id("syn:explicit-preset"),
                revision: id("preset-1"),
                label: "Illustrative explicit setup".into(),
                spec,
                parameters,
            }],
            label: "Synthetic contract fixture".into(),
            path: "data/mechanics/synthetic/syn-welcome.json".into(),
            covered_definitions: BTreeSet::new(),
            unsupported_definitions: BTreeSet::new(),
            provenance: vec![],
            capabilities: [Operation::Evaluate].into(),
        }],
    }
}
#[test]
fn public_setup_presets_are_complete_typed_and_bound_to_exact_mechanics_normalization_and_labels() {
    let valid = catalog();
    valid.validate().unwrap();
    let mut bad = valid.clone();
    bad.mechanics[0].presets[0].spec.normalization.revision = id("other");
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.mechanics[0].presets[0].spec.mechanics = MechanicsRef::Live {
        game_version: "unknown".into(),
        revision: id("other"),
        sha256: Digest::of(b"other"),
    };
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.mechanics[0].presets[0].parameters.pop();
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.mechanics[0].presets[0].parameters[0].domain = ParameterDomain::Flag;
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.mechanics[0].presets[0].spec.assumptions.values.insert(
        id("syn:unlabelled-assumption"),
        AssumptionValue::Flag(false),
    );
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.definitions.clear();
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.definitions[0].label = "x".repeat(161);
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.mechanics[0].path = "../private-data/data.json".into();
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.mechanics[0].presets[0].parameters[1].domain = ParameterDomain::Quantity {
        unit: QuantityUnit::Ratio,
        minimum: Fixed::ZERO,
        maximum: Fixed::ZERO,
    };
    assert!(bad.validate().is_err());
    wire::decode::<Catalog>(&wire::encode(&valid).unwrap())
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn current_slot_knowledge_accepts_complete_empty_and_partial_occupied_but_never_array_presence() {
    let mut current = spec();
    current.assignments = AssignmentDomain::CurrentLoadout;
    let mut a = account();
    let mut empty = allocation();
    empty.teams[0].builds[0].weapon = None;
    empty.footprint.items.clear();
    for completeness in [Completeness::Unspecified, Completeness::DeclaredPartial] {
        a.weapons = Section::Supplied {
            completeness: completeness.clone(),
            records: vec![],
        };
        a.artifacts = Section::Supplied {
            completeness,
            records: vec![],
        };
        assert_eq!(
            empty.validate(&a, &current).unwrap_err().code,
            IssueCode::MissingSection
        );
    }
    a.weapons = complete(vec![]);
    a.artifacts = complete(vec![]);
    empty.validate(&a, &current).unwrap();
    let mut records = vec![];
    let mut artifacts = vec![];
    for n in 0..4 {
        let c: CharacterId = id(&format!("c{n}"));
        let w = format!("equipped-{n}");
        let mut weapon = weapon(&w);
        weapon.owner = OwnerRef::Character(c.clone());
        records.push(weapon);
        empty.teams[0].builds[n].weapon = Some(id(&w));
        empty.footprint.items.insert(id(&w));
        for (index, slot) in [
            ArtifactSlot::Flower,
            ArtifactSlot::Plume,
            ArtifactSlot::Sands,
            ArtifactSlot::Goblet,
            ArtifactSlot::Circlet,
        ]
        .into_iter()
        .enumerate()
        {
            let name = format!("artifact-{n}-{index}");
            let mut artifact = flower(&name, OwnerRef::Character(c.clone()));
            artifact.slot = slot;
            artifacts.push(artifact);
            empty.teams[0].builds[n].artifacts.insert(slot, id(&name));
            empty.footprint.items.insert(id(&name));
        }
    }
    a.weapons = supplied(records);
    a.artifacts = supplied(artifacts);
    empty.validate(&a, &current).unwrap();
    if let Section::Supplied { records, .. } = &mut a.artifacts {
        records.pop();
    }
    empty.teams[0].builds[3]
        .artifacts
        .remove(&ArtifactSlot::Circlet);
    empty.footprint.items.remove(&id("artifact-3-4"));
    assert_eq!(
        empty.validate(&a, &current).unwrap_err().code,
        IssueCode::MissingSection
    );
    let declaration = ImportDeclaration {
        section: InventorySection::Artifacts,
        completeness: Completeness::DeclaredComplete {
            asserted_by: CompletenessAuthority::User {
                statement: "I explicitly confirm this supplied section is complete".into(),
            },
        },
    };
    let declared = a
        .with_import_declarations(vec![declaration.clone()])
        .unwrap();
    empty.validate(&declared, &current).unwrap();
    assert!(
        account()
            .with_import_declarations(vec![declaration])
            .is_err()
    );
    let mut a = account();
    a.artifacts = supplied(vec![]);
    let exporter = ImportDeclaration {
        section: InventorySection::Artifacts,
        completeness: Completeness::DeclaredComplete {
            asserted_by: CompletenessAuthority::Exporter {
                name: "fixture exporter".into(),
                source: SourceRef {
                    archive_digest: Digest::of(b"{}"),
                    pointer: "/artifacts".into(),
                },
            },
        },
    };
    assert!(a.with_import_declarations(vec![exporter]).is_err());
}

#[test]
fn exact_displaced_weapon_swap_is_allowed_but_unrelated_occupied_fallback_is_not() {
    let (trusted, mut comparison) = comparison_case(true);
    let mut a = trusted.snapshot().clone();
    if let Section::Supplied { records, .. } = &mut a.weapons {
        records[1].owner = OwnerRef::Character(id("c0"));
    }
    comparison.before.allocations[0].teams[0].builds[0].weapon = Some(id("w1"));
    comparison.before.allocations[0]
        .footprint
        .items
        .insert(id("w1"));
    if let ComparisonMode::CurrentLoadout { allocation, .. } = &mut comparison.comparison.mode {
        **allocation = comparison.before.allocations[0].clone();
    }
    comparison.after.spec = comparison.comparison.intervention_spec();
    rebind(&mut comparison.before, &a);
    rebind(&mut comparison.after, &a);
    let swapped = ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap();
    comparison.validate_for(&swapped).unwrap();
    assert_eq!(
        comparison.before.allocations[0].teams[0].builds[0].weapon,
        Some(id("w1"))
    );
    assert_eq!(
        comparison.before.allocations[0].teams[0].builds[1].weapon,
        Some(id("w0"))
    );
    assert_eq!(
        comparison.after.allocations[0].teams[0].builds[0].weapon,
        Some(id("w0"))
    );
    assert_eq!(
        comparison.after.allocations[0].teams[0].builds[1].weapon,
        Some(id("w1"))
    );
    assert_eq!(comparison.original_owner, OwnerRef::Character(id("c1")));
    for policy in 0..3 {
        let mut blocked = a.clone();
        match policy {
            0 => {
                blocked.preferences.protected_items.insert(id("w1"));
            }
            1 => {
                if let Section::Supplied { records, .. } = &mut blocked.weapons {
                    records[1].game_lock = Some(true);
                }
            }
            _ => {
                blocked.preferences.pinned.push(PinnedAssignment {
                    item: id("w1"),
                    owner: OwnerRef::Character(id("c0")),
                });
            }
        }
        assert!(
            comparison
                .comparison
                .validate_for(&ValidatedAccount::bundled(blocked, SampleId::SynWelcome).unwrap())
                .is_err()
        );
    }
    let mut unrelated = a;
    if let Section::Supplied { records, .. } = &mut unrelated.weapons {
        let mut third = weapon("w2");
        third.owner = OwnerRef::Character(id("c7"));
        records.push(third);
    }
    let mut bad = comparison.comparison.clone();
    if let ComparisonMode::CurrentLoadout { donor_fallback, .. } = &mut bad.mode {
        *donor_fallback = Some(id("w2"));
    }
    let unrelated = ValidatedAccount::bundled(unrelated, SampleId::SynWelcome).unwrap();
    assert!(bad.validate_for(&unrelated).is_err());
}

#[test]
fn artifact_swap_uses_only_the_displaced_same_slot_and_move_only_rechecks_current_knowledge() {
    let mut a = current_account();
    a.weapons = complete(vec![]);
    a.artifacts = complete(vec![
        flower("donor-flower", OwnerRef::Character(id("c1"))),
        flower("recipient-flower", OwnerRef::Character(id("c0"))),
        flower("third-flower", OwnerRef::Character(id("c7"))),
    ]);
    let mut before = allocation();
    before.teams[0].builds[0].weapon = None;
    before.teams[0].builds[0]
        .artifacts
        .insert(ArtifactSlot::Flower, id("recipient-flower"));
    before.teams[0].builds[1]
        .artifacts
        .insert(ArtifactSlot::Flower, id("donor-flower"));
    before.footprint.items = [id("donor-flower"), id("recipient-flower")].into();
    let mut baseline = spec();
    baseline.assignments = AssignmentDomain::CurrentLoadout;
    before.validate(&a, &baseline).unwrap();
    let comparison = ComparisonSpec {
        baseline,
        mode: ComparisonMode::CurrentLoadout {
            allocation: Box::new(before.clone()),
            donor_fallback: Some(id("recipient-flower")),
        },
        movement: ItemMove {
            item: id("donor-flower"),
            recipient: id("c0"),
        },
    };
    comparison
        .validate_for(&ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap())
        .unwrap();
    let after_spec = comparison.intervention_spec();
    let mut after = before.clone();
    after.teams[0].builds[0]
        .artifacts
        .insert(ArtifactSlot::Flower, id("donor-flower"));
    after.teams[0].builds[1]
        .artifacts
        .insert(ArtifactSlot::Flower, id("recipient-flower"));
    after.validate(&a, &after_spec).unwrap();
    let mut bad = comparison.clone();
    if let ComparisonMode::CurrentLoadout { donor_fallback, .. } = &mut bad.mode {
        *donor_fallback = Some(id("third-flower"));
    }
    assert!(
        bad.validate_for(&ValidatedAccount::bundled(a.clone(), SampleId::SynWelcome).unwrap())
            .is_err()
    );
    if let Section::Supplied { records, .. } = &mut a.artifacts {
        records[1].slot = ArtifactSlot::Plume;
    }
    assert!(after.validate(&a, &after_spec).is_err());
    a.artifacts = supplied(vec![
        flower("donor-flower", OwnerRef::Character(id("c1"))),
        flower("recipient-flower", OwnerRef::Character(id("c0"))),
    ]);
    assert_eq!(
        after.validate(&a, &after_spec).unwrap_err().code,
        IssueCode::MissingSection
    );
}

#[test]
fn presentation_backpressure_keeps_one_unacknowledged_page_and_rejects_stale_steps() {
    let mut gate = ready_dom(
        HeadObservation::Unavailable {
            last_readable: None,
        },
        None,
    );
    let b = binding(gate.start(id("present")).unwrap());
    gate.bind(b.clone()).unwrap();
    let page = PresentationPage {
        binding: b.clone(),
        view_digest: Digest::of(b"actual result commitment"),
        page: Counter(0),
        total_pages: Counter(2),
        search: Some(SearchStatus::Bounded),
        frontier: Some(FrontierPresentation::AmongReturned),
        rows: vec![],
    };
    gate.present(&page).unwrap();
    assert!(gate.present(&page).is_err());
    assert!(gate.finish(&b).is_err());
    assert!(
        gate.acknowledge_page(&page.view_digest, Counter(1))
            .is_err()
    );
    gate.acknowledge_page(&page.view_digest, Counter(0))
        .unwrap();
    gate.next_step(&b).unwrap();
    assert!(gate.present(&page).is_err());
    let mut next = b;
    next.step = Counter(1);
    next.operation = Digest::of(b"new view request");
    gate.bind(next.clone()).unwrap();
    gate.finish(&next).unwrap();
    assert_eq!(gate.phase(), &DomPhase::Idle);
}

#[test]
fn immutable_prepared_paths_share_account_and_backup_instead_of_multiplying_source_copies() {
    let validated = ValidatedAccount::imported(account()).unwrap();
    let backup = Arc::new(Backup::from_validated(&validated).unwrap());
    assert!(Arc::ptr_eq(&backup.account, &validated.shared_snapshot()));
    let restored = backup.restore(None).unwrap();
    assert!(Arc::ptr_eq(&backup.account, &restored.shared_snapshot()));
    let mut machine =
        CommitMachine::new(context().epochs, true, observed(durable_account())).unwrap();
    machine
        .prepare(Some(backup.clone()), &context().epochs)
        .unwrap();
    let candidate = machine.proposed_record().unwrap();
    assert!(Arc::ptr_eq(candidate.backup.as_ref().unwrap(), &backup));
    let proposed_again = machine.proposed_record().unwrap();
    assert!(Arc::ptr_eq(&candidate, &proposed_again));
    machine.promote_session_only().unwrap();
    assert!(Arc::ptr_eq(machine.active().unwrap(), &backup));
    let input = machine.recovery_input();
    assert!(Arc::ptr_eq(
        input.selected_backup().unwrap().unwrap(),
        &backup
    ));
    let mut blobs = BlobLedger::default();
    blobs
        .retain(Counter(1), Counter(MAX_BLOB_LEDGER_BYTES))
        .unwrap();
    assert!(blobs.retain(Counter(2), Counter(1)).is_err());
    assert!(blobs.retain(Counter(1), Counter(1)).is_err());
    assert_eq!(blobs.bytes(), MAX_BLOB_LEDGER_BYTES);
    blobs.share(Counter(1)).unwrap();
    blobs.release(Counter(1)).unwrap();
    assert_eq!(blobs.bytes(), MAX_BLOB_LEDGER_BYTES);
    blobs.release(Counter(1)).unwrap();
    assert_eq!(blobs.bytes(), 0);
    assert!(blobs.release(Counter(1)).is_err());
    assert_eq!(MAX_WORKER_LINEAR_BYTES, 512 * 1024 * 1024);
}

#[test]
fn initial_bootstrap_and_restart_acknowledgements_advance_generation_without_invented_storage() {
    let mut context = context();
    context.account_semantics = None;
    let mut session = SessionGate::new(context.clone(), Counter(0));
    session.begin_bootstrap(unread()).unwrap();
    assert!(session.begin_bootstrap(unread()).is_err());
    assert!(
        session
            .register(&gate_request(&session, "too-early", Request::GetCatalog))
            .is_err()
    );
    let request = gate_request(
        &session,
        "initial-bootstrap",
        Request::Bootstrap { input: unread() },
    );
    session.register(&request).unwrap();
    assert!(session.accept(&reply(
        &request,
        Event::SessionDiscarded {
            previous_generation: Counter(0)
        },
        Counter(1)
    )));
    let mut dom = DomGate::new(
        context.epochs,
        Counter(0),
        HeadObservation::Unavailable {
            last_readable: None,
        },
        None,
        None,
    )
    .unwrap();
    let b = binding(dom.begin_bootstrap(id("initial-bootstrap")).unwrap());
    dom.bind(b.clone()).unwrap();
    let mut acknowledgement = RestorationAck {
        binding: b,
        account: None,
        generation: Counter(0),
        checkpoint: None,
    };
    assert!(dom.restored(&acknowledgement).is_err());
    acknowledgement.generation = Counter(1);
    dom.restored(&acknowledgement).unwrap();
    assert_eq!(
        dom.start(id("next")).unwrap().generation,
        session.generation()
    );
}

#[test]
fn compact_settlement_requires_the_exact_candidate_and_reinitialization_store_marker() {
    let (mut dom, _, reservation) = saving_case();
    dom.reserve(reservation.clone()).unwrap();
    let mut unrelated = reservation.proposed.clone();
    unrelated.revision = Counter(12);
    assert!(
        dom.settle_reservation(&reservation.binding, &unrelated)
            .is_err()
    );
    assert_eq!(dom.phase(), &DomPhase::Reserved);
    assert!(
        dom.settle_reservation(&reservation.binding, &reservation.expected)
            .unwrap()
            .is_none()
    );
    dom.reserve(reservation.clone()).unwrap();
    assert!(
        dom.settle_reservation(&reservation.binding, &reservation.proposed)
            .unwrap()
            .is_some()
    );
    let mut dom = ready_dom(
        HeadObservation::Unreadable {
            last_known_head: None,
        },
        None,
    );
    let b = binding(dom.begin_clear(id("settle-clear")).unwrap());
    dom.bind(b.clone()).unwrap();
    let plan = dom.reserve_privacy_clear(&b, id("new-store")).unwrap();
    let ClearPlan::Reinitialize { tombstone, .. } = plan else {
        panic!("expected reinitialization");
    };
    let mut observed = StoredHead {
        store: id("old-store"),
        head: tombstone,
    };
    assert!(dom.settle_privacy_clear(&b, &observed).is_err());
    assert_eq!(dom.phase(), &DomPhase::Reserved);
    observed.store = id("new-store");
    assert!(dom.settle_privacy_clear(&b, &observed).unwrap().is_some());
    assert_eq!(dom.phase(), &DomPhase::Activating);
}
