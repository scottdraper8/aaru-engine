use super::*;
use std::sync::Weak;

const ACCEPTED_UI: &[u8] = b"objective=energy;targets=2;form=alternate";

fn private_backup(name: &str) -> (Arc<Backup>, Weak<SourceArchive>) {
    let utf8 = format!(
        "{{\"format\":\"GOOD\",\"version\":3,\"source\":\"{name}\",\"materials\":{{\"syn:ore\":18446744073709551615}}}}"
    );
    let source = Arc::new(SourceArchive {
        format: SourceFormat::Good {
            dialect_version: 3,
            profile: ImportProfileVersion,
        },
        sha256: Digest::of(utf8.as_bytes()),
        utf8,
    });
    let weak = Arc::downgrade(&source);
    let account = ValidatedAccount::imported(AccountSnapshot {
        schema_version: AccountVersion,
        id: id(name),
        characters: Section::Omitted {},
        weapons: Section::Omitted {},
        artifacts: Section::Omitted {},
        materials: supplied(vec![
            MaterialBalance {
                definition: id("syn:ore"),
                quantity: MaterialQuantity::Known {
                    amount: Counter(u64::MAX),
                },
                source: Some(SourceRef {
                    archive_digest: source.sha256.clone(),
                    pointer: "/materials/syn:ore".into(),
                }),
                provenance: vec![],
            },
            crate::material_cases::balance("syn:unknown", MaterialQuantity::Unknown {}),
        ]),
        preferences: AllocationPreferences::default(),
        source_archive: Some(source),
        provenance: vec![],
    })
    .unwrap();
    (Arc::new(Backup::from_validated(&account).unwrap()), weak)
}
fn account_head(backup: &Backup) -> AccountHead {
    AccountHead {
        snapshot: backup.account.id.clone(),
        semantics: wire::account_semantic_digest(&backup.account).unwrap(),
        backup_digest: backup.digest().unwrap(),
    }
}
fn transition(gate: &DomGate, prepared: &PreparedHead, bytes: &[u8]) -> StagedAccountTransition {
    let checkpoint = checkpoint(
        &prepared.binding,
        gate.checkpoint().map_or(1, |h| h.revision.0 + 1),
        Some(prepared.account.semantics.clone()),
        bytes,
    );
    let policy = if gate
        .intended()
        .is_some_and(|h| h.snapshot == prepared.account.snapshot)
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
    StagedAccountTransition {
        previous_checkpoint: gate.checkpoint().map(UiCheckpointHead::commitment),
        checkpoint,
        policy,
    }
}
fn ack(dom: &DomGate, binding: &ActionBinding) -> RestorationAck {
    RestorationAck {
        binding: binding.clone(),
        account: dom.intended().cloned(),
        generation: dom.generation().next().unwrap(),
        checkpoint: dom.checkpoint().map(UiCheckpointHead::commitment),
    }
}
struct Lifecycle {
    dom: DomGate,
    session: SessionGate,
    machine: CommitMachine,
    checkpoints: CheckpointGate,
}
impl Lifecycle {
    fn bootstrap(
        input: RecoveryInput,
        epochs: Epochs,
        generation: Counter,
        checkpoint: Option<UiCheckpointHead>,
    ) -> Self {
        let selected = input.selected_backup().unwrap().map(|b| account_head(b));
        let mut context = context();
        context.epochs = epochs.clone();
        context.account_semantics = selected.as_ref().map(|h| h.semantics.clone());
        let mut machine = CommitMachine::new(epochs.clone(), true, input).unwrap();
        let input = machine.recovery_input();
        let mut session = SessionGate::new(context, generation);
        session.begin_bootstrap(input.clone()).unwrap();
        let mut dom = DomGate::new(
            epochs,
            generation,
            input.storage.head_observation().unwrap(),
            selected.clone(),
            checkpoint.clone(),
        )
        .unwrap();
        let authority = dom.begin_bootstrap(id("bootstrap-pair")).unwrap();
        let request = gate_request(
            &session,
            "bootstrap-pair",
            Request::Bootstrap {
                input: input.clone(),
            },
        );
        session.register(&request).unwrap();
        let b = ActionBinding {
            authority,
            step: Counter(0),
            operation: request.fingerprint.clone(),
        };
        dom.bind(b.clone()).unwrap();
        machine
            .bootstrap(session.context().epochs.clone(), input.clone())
            .unwrap();
        let payload = match selected {
            Some(head) => Event::Activated {
                snapshot: head.snapshot,
                account_semantics: head.semantics,
                previous_generation: generation,
                capability: capability(),
            },
            None if matches!(input.active, ActiveSelection::ObservedDurable) => {
                Event::ResetAcknowledged {
                    previous_generation: generation,
                }
            }
            None => Event::SessionDiscarded {
                previous_generation: generation,
            },
        };
        assert!(session.accept(&reply(&request, payload, generation.next().unwrap())));
        dom.restored(&ack(&dom, &b)).unwrap();
        let checkpoints =
            CheckpointGate::new(dom.intended().map(|h| h.semantics.clone()), checkpoint).unwrap();
        assert_eq!(dom.generation(), session.generation());
        Self {
            dom,
            session,
            machine,
            checkpoints,
        }
    }
    fn reconstruct(mut self) -> Self {
        let intended = self.dom.intended().cloned();
        let checkpoint = self.dom.checkpoint().cloned();
        let generation = self.dom.generation();
        let next_epochs = if self.dom.phase() == &DomPhase::Recovering {
            self.dom.epochs().clone()
        } else {
            self.dom.crash_after_settlement().unwrap().next_epochs
        };
        let input = self.machine.recovery_input();
        assert_eq!(
            input.selected_backup().unwrap().map(|b| account_head(b)),
            intended
        );
        assert_eq!(
            input.storage.head_observation().unwrap(),
            *self.dom.storage()
        );
        drop(self);
        Self::bootstrap(input, next_epochs, generation, checkpoint)
    }
    fn edit(&mut self, name: &str, edit: FormEdit, bytes: &[u8]) {
        let b = binding(self.dom.start(id(name)).unwrap());
        self.dom.bind(b.clone()).unwrap();
        self.checkpoints
            .offer_edit(b.clone(), self.checkpoints.revision(), edit)
            .unwrap();
        let next = checkpoint(
            &b,
            self.checkpoints.revision().0 + 1,
            self.dom.intended().map(|h| h.semantics.clone()),
            bytes,
        );
        self.checkpoints.retain_checkpoint(next.clone()).unwrap();
        self.dom.retain_checkpoint(next).unwrap();
        self.dom.finish(&b).unwrap();
    }
    fn prepare(
        &mut self,
        backup: Arc<Backup>,
        name: &str,
        preference: bool,
    ) -> (PreparedHead, PreparedRegistry) {
        let authority = self.dom.start(id(name)).unwrap();
        let request = gate_request(
            &self.session,
            name,
            if preference {
                Request::PreparePreferences {
                    preferences: backup.account.preferences.clone(),
                }
            } else {
                Request::PrepareImport {
                    input: ImportInput::Restore {
                        backup: backup.clone(),
                    },
                }
            },
        );
        self.session.register(&request).unwrap();
        let b = ActionBinding {
            authority,
            step: Counter(0),
            operation: request.fingerprint.clone(),
        };
        self.dom.bind(b.clone()).unwrap();
        let preview = ImportPreview {
            token: id(name),
            replaces_generation: self.session.generation(),
            context: self.session.context().clone(),
            canonical_digest: backup.digest().unwrap(),
            backup: backup.clone(),
            issues: vec![],
            capability: capability(),
        };
        let mut registry = PreparedRegistry::default();
        registry
            .prepare(
                &preview,
                self.session.generation(),
                self.session.context(),
                backup.restore(None).unwrap(),
            )
            .unwrap();
        assert!(self.session.accept(&reply(
            &request,
            Event::ImportPrepared {
                preview: Box::new(preview.clone())
            },
            self.session.generation()
        )));
        let head = PreparedHead {
            binding: b,
            token: preview.token,
            account: account_head(&backup),
            characters: 0,
            weapons: 0,
            artifacts: 0,
            material_kinds: backup.account.materials.records().len() as u16,
            issues: vec![],
        };
        self.dom.prepared(head.clone()).unwrap();
        (head, registry)
    }
    fn promote(&mut self, head: &PreparedHead) {
        self.dom.promote_session_anchor(head).unwrap();
        self.session
            .promote_prepared_session(&mut self.machine, &head.token)
            .unwrap();
        self.checkpoints = CheckpointGate::new(
            self.dom.intended().map(|h| h.semantics.clone()),
            self.dom.checkpoint().cloned(),
        )
        .unwrap();
    }
    fn activate(
        &mut self,
        head: &PreparedHead,
        registry: &mut PreparedRegistry,
        storage: StorageDisposition,
    ) {
        let request = gate_request(
            &self.session,
            "activate",
            Request::ActivatePrepared {
                token: head.token.clone(),
                storage: storage.clone(),
            },
        );
        let b = ActionBinding {
            authority: head.binding.authority.clone(),
            step: Counter(1),
            operation: request.fingerprint.clone(),
        };
        self.dom.bind_activation(b.clone()).unwrap();
        self.session.register(&request).unwrap();
        let generation = registry
            .activate(
                &head.token,
                self.session.generation(),
                self.session.context(),
                &storage,
            )
            .unwrap();
        assert_eq!(
            registry
                .activate(&head.token, generation, self.session.context(), &storage)
                .unwrap(),
            generation
        );
        assert!(self.session.accept(&reply(
            &request,
            Event::Activated {
                snapshot: head.account.snapshot.clone(),
                account_semantics: head.account.semantics.clone(),
                previous_generation: self.session.generation(),
                capability: capability()
            },
            generation
        )));
        self.dom
            .activation_acknowledged(&ack(&self.dom, &b))
            .unwrap();
        match storage {
            StorageDisposition::SessionOnly => {
                self.machine.acknowledge_session_activation().unwrap()
            }
            StorageDisposition::Persisted { .. } => self.machine.activate_committed().unwrap(),
        }
        self.checkpoints = CheckpointGate::new(
            Some(head.account.semantics.clone()),
            self.dom.checkpoint().cloned(),
        )
        .unwrap();
        assert_eq!(self.dom.generation(), self.session.generation());
    }
    fn reserve(&mut self, head: &PreparedHead) -> (StorageReservation, Arc<DurableRecord>) {
        let record = self
            .session
            .propose_prepared_save(&mut self.machine, &head.token)
            .unwrap();
        let bytes = wire::encode(&record).unwrap();
        let reservation = StorageReservation {
            binding: head.binding.clone(),
            token: head.token.clone(),
            expected: self.machine.durable_head().unwrap().unwrap(),
            proposed: DurableHead::from_record(&record).unwrap(),
            record: descriptor(&head.binding, BulkPurpose::DurableRecord, bytes.as_bytes()),
        };
        self.dom.reserve(reservation.clone()).unwrap();
        self.session
            .reserve_prepared_save(&mut self.machine, &reservation.expected)
            .unwrap();
        (reservation, record)
    }
}
fn capability() -> EvaluationCapability {
    EvaluationCapability {
        supported: [Operation::ExportCanonical].into(),
        blockers: vec![],
        scope: EvaluationScope::WholeSuppliedInventory,
    }
}

#[test]
fn nonempty_materials_survive_prepared_session_durable_and_lost_ack_recovery() {
    for durable in [false, true] {
        for acknowledge in [false, true] {
            let saved = private_backup("material-saved").0;
            let candidate = private_backup("material-candidate").0;
            let expected = candidate.account.materials.clone();
            let original = candidate.account.source_archive.clone();
            let record = Arc::new(DurableRecord {
                revision: Counter(7),
                backup: Some(saved),
            });
            let mut life =
                Lifecycle::bootstrap(observed(record), context().epochs, Counter(0), None);
            life.edit(
                "material-ui",
                FormEdit::EncounterTargets {
                    encounter: 0,
                    count: 2,
                },
                ACCEPTED_UI,
            );
            let (head, mut registry) = life.prepare(candidate, "material-prepare", false);
            assert_eq!(head.material_kinds, 2);
            life.dom
                .stage_account_transition(transition(&life.dom, &head, ACCEPTED_UI))
                .unwrap();
            let storage = if durable {
                let (_, record) = life.reserve(&head);
                let encoded = wire::encode(&record).unwrap();
                let decoded: DurableRecord = wire::decode(&encoded).unwrap();
                assert_eq!(decoded.backup.as_ref().unwrap().account.materials, expected);
                let receipt = life.dom.transaction_complete(&head.binding).unwrap();
                life.session
                    .storage_committed(&mut life.machine, &receipt)
                    .unwrap();
                StorageDisposition::Persisted { receipt }
            } else {
                life.promote(&head);
                StorageDisposition::SessionOnly
            };
            if acknowledge {
                life.activate(&head, &mut registry, storage);
                assert_eq!(registry.active().unwrap().snapshot().materials, expected);
            }
            let input = life.machine.recovery_input();
            let bytes = wire::encode(&input).unwrap();
            let decoded: RecoveryInput = wire::decode(&bytes).unwrap();
            let restored = decoded.restore(None).unwrap().unwrap();
            assert_eq!(restored.snapshot().materials, expected);
            assert_eq!(restored.snapshot().source_archive, original);
            assert_eq!(
                restored.material_balance(&id("syn:ore")),
                MaterialQuantity::Known {
                    amount: Counter(u64::MAX)
                }
            );
            assert_eq!(
                restored.material_balance(&id("syn:unknown")),
                MaterialQuantity::Unknown {}
            );
            let rebuilt = life.reconstruct();
            assert_eq!(
                rebuilt.machine.active().unwrap().account.materials,
                expected
            );
            assert_eq!(rebuilt.dom.intended(), Some(&head.account));
            assert_eq!(
                rebuilt.dom.checkpoint().unwrap().account_semantics,
                Some(head.account.semantics)
            );
            assert_eq!(
                rebuilt.machine.durable_head().unwrap().unwrap().revision,
                Counter(if durable { 8 } else { 7 })
            );
        }
    }
}

#[test]
fn discard_commits_empty_account_and_ui_before_ack_without_erasing_any_storage() {
    for mode in 0..3 {
        for crash_before_ack in [false, true] {
            let saved = private_backup("saved-a").0;
            let retained = private_backup("active-b").0;
            let record = Arc::new(DurableRecord {
                revision: Counter(7),
                backup: Some(saved),
            });
            let saved_head = DurableHead::from_record(&record).unwrap();
            let storage = match mode {
                0 => StorageObservation::Unavailable {
                    last_observed_head: Some(saved_head),
                },
                1 => StorageObservation::Readable { record },
                _ => StorageObservation::Corrupt { head: saved_head },
            };
            let mut life = Lifecycle::bootstrap(
                RecoveryInput {
                    storage,
                    active: ActiveSelection::RetainedSession { backup: retained },
                },
                context().epochs,
                Counter(0),
                None,
            );
            life.edit(
                "accepted-ui",
                FormEdit::EncounterTargets {
                    encounter: 0,
                    count: 2,
                },
                ACCEPTED_UI,
            );
            let observed = life.dom.storage().clone();
            let old = life.dom.start(id("old-operation")).unwrap();
            let authority = life.dom.discard_session(id("discard-memory")).unwrap();
            life.session
                .discard_session_barrier(&mut life.machine)
                .unwrap();
            assert_eq!(&authority.epochs, &life.session.context().epochs);
            assert_eq!(authority.epochs.reset, Counter(1));
            assert!(!life.dom.admits(&old, Counter(0)));
            assert_eq!(life.dom.intended(), None);
            assert_eq!(life.dom.checkpoint(), None);
            assert_eq!(life.machine.active(), None);
            assert_eq!(life.dom.storage(), &observed);
            assert_eq!(
                life.machine.recovery_input().active,
                ActiveSelection::EmptySession
            );
            assert!(life.dom.cancel().is_err());
            if !crash_before_ack {
                let request =
                    gate_request(&life.session, "discard-memory", Request::DiscardSession);
                life.session.register(&request).unwrap();
                let b = ActionBinding {
                    authority,
                    step: Counter(0),
                    operation: request.fingerprint.clone(),
                };
                life.dom.bind_activation(b.clone()).unwrap();
                assert!(life.session.accept(&reply(
                    &request,
                    Event::SessionDiscarded {
                        previous_generation: Counter(1)
                    },
                    Counter(2)
                )));
                life.dom
                    .activation_acknowledged(&ack(&life.dom, &b))
                    .unwrap();
            }
            let restored = life.reconstruct();
            assert_eq!(
                restored.dom.generation(),
                Counter(if crash_before_ack { 2 } else { 3 })
            );
            assert_eq!(restored.dom.intended(), None);
            assert_eq!(restored.dom.checkpoint(), None);
            assert_eq!(restored.dom.storage(), &observed);
        }
    }
}

#[test]
fn discard_crash_before_supervisor_dispatch_reconstructs_from_the_dom_memory_commit() {
    let retained = private_backup("discard-before-dispatch").0;
    let mut life = Lifecycle::bootstrap(
        RecoveryInput {
            storage: unread().storage,
            active: ActiveSelection::RetainedSession { backup: retained },
        },
        context().epochs,
        Counter(0),
        None,
    );
    life.dom.discard_session(id("discard-before-send")).unwrap();
    assert!(life.machine.active().is_some());
    let input = RecoveryInput {
        storage: life.machine.recovery_input().storage,
        active: ActiveSelection::EmptySession,
    };
    let generation = life.dom.generation();
    let epochs = life.dom.crash_after_settlement().unwrap().next_epochs;
    assert_eq!(life.dom.intended(), None);
    assert_eq!(life.dom.checkpoint(), None);
    drop(life);
    let restored = Lifecycle::bootstrap(input, epochs, generation, None);
    assert_eq!(restored.machine.active(), None);
    assert_eq!(restored.dom.generation(), Counter(2));
    assert_eq!(restored.dom.epochs().reset, Counter(1));
}

#[test]
fn failed_read_preserves_head_but_cannot_authorize_logical_or_dom_saving() {
    let saved = Arc::new(DurableRecord {
        revision: Counter(7),
        backup: Some(private_backup("saved-for-failure").0),
    });
    let known = DurableHead::from_record(&saved).unwrap();
    let mut life = Lifecycle::bootstrap(
        RecoveryInput {
            storage: StorageObservation::Readable { record: saved },
            active: ActiveSelection::RetainedSession {
                backup: private_backup("newer-session").0,
            },
        },
        context().epochs,
        Counter(0),
        None,
    );
    let authority = life.dom.begin_storage_refresh(id("failed-read")).unwrap();
    let failed = HeadObservation::Unavailable {
        last_readable: Some(known.clone()),
    };
    let mut fabricated = known.clone();
    fabricated.revision = Counter(8);
    assert!(
        life.dom
            .finish_storage_refresh(
                &authority,
                HeadObservation::Unavailable {
                    last_readable: Some(fabricated)
                }
            )
            .is_err()
    );
    life.dom.finish_storage_refresh(&authority, failed).unwrap();
    life.machine
        .refresh_storage(
            &life.session.context().epochs,
            StorageObservation::Unavailable {
                last_observed_head: Some(known.clone()),
            },
        )
        .unwrap();
    let (prepared, _) = life.prepare(
        private_backup("unsaved-candidate").0,
        "prepare-after-failure",
        false,
    );
    life.dom
        .stage_account_transition(transition(&life.dom, &prepared, ACCEPTED_UI))
        .unwrap();
    assert!(
        life.session
            .propose_prepared_save(&mut life.machine, &prepared.token)
            .is_err()
    );
    let record = DurableRecord {
        revision: Counter(8),
        backup: life
            .machine
            .recovery_input()
            .selected_backup()
            .unwrap()
            .cloned(),
    };
    let mut proposed = DurableHead::from_record(&record).unwrap();
    proposed.content = DurableContent::Account {
        account: prepared.account.clone(),
    };
    let bytes = b"precomputed candidate record";
    proposed.record_digest = Digest::of(bytes);
    let reservation = StorageReservation {
        binding: prepared.binding.clone(),
        token: prepared.token.clone(),
        expected: known,
        proposed,
        record: descriptor(&prepared.binding, BulkPurpose::DurableRecord, bytes),
    };
    reservation.validate(&prepared).unwrap();
    assert!(life.dom.reserve(reservation).is_err());
    assert_eq!(life.machine.phase(), &CommitPhase::Prepared);
}

#[test]
fn target_checkpoint_is_required_and_activation_binding_is_exact_once_for_both_commit_modes() {
    for durable in [false, true] {
        let (mut dom, prepared, reservation) = saving_case();
        if durable {
            dom.reserve(reservation).unwrap();
            dom.transaction_complete(&prepared.binding).unwrap();
        } else {
            dom.promote_session_anchor(&prepared).unwrap();
        }
        let mut b = prepared.binding.clone();
        b.step = Counter(1);
        b.operation = Digest::of(b"original activation");
        assert!(dom.bind(prepared.binding.clone()).is_err());
        dom.bind_activation(b.clone()).unwrap();
        dom.bind_activation(b.clone()).unwrap();
        let mut changed = b.clone();
        changed.operation = Digest::of(b"queued different activation");
        assert!(dom.bind_activation(changed.clone()).is_err());
        assert!(dom.activation_acknowledged(&ack(&dom, &changed)).is_err());
        for field in 0..6 {
            let mut bad = ack(&dom, &b);
            match field {
                0 => bad.checkpoint = None,
                1 => bad.checkpoint.as_mut().unwrap().revision = Counter(0),
                2 => bad.checkpoint.as_mut().unwrap().schema = id("wrong-schema"),
                3 => bad.checkpoint.as_mut().unwrap().sha256 = Digest::of(b"wrong-ui"),
                4 => bad.checkpoint.as_mut().unwrap().account_semantics = None,
                _ => bad.checkpoint.as_mut().unwrap().byte_length = Counter(1),
            }
            assert!(dom.activation_acknowledged(&bad).is_err());
            let frame = ControlFrame {
                version: ProtocolVersion,
                from: Endpoint::UiSupervisor,
                to: Endpoint::Dom,
                authority: b.authority.clone(),
                step: b.step,
                payload: Control::Activated {
                    acknowledgement: bad,
                },
            };
            let decoded = decode_control(&encode_control(&frame).unwrap()).unwrap();
            let Control::Activated { acknowledgement } = decoded.payload else {
                panic!("wrong acknowledgement");
            };
            assert!(dom.activation_acknowledged(&acknowledgement).is_err());
        }
        let original = ack(&dom, &b);
        dom.activation_acknowledged(&original).unwrap();
        assert!(dom.activation_acknowledged(&original).is_err());
        assert!(dom.bind_activation(changed).is_err());
    }
}

#[test]
fn preference_transition_preserves_accepted_settings_and_replacement_requires_exact_confirmation() {
    let active = private_backup("same-account").0;
    let mut life = Lifecycle::bootstrap(
        RecoveryInput {
            storage: unread().storage,
            active: ActiveSelection::RetainedSession {
                backup: active.clone(),
            },
        },
        context().epochs,
        Counter(0),
        None,
    );
    life.edit(
        "objective",
        FormEdit::ObjectiveWeight {
            dimension: Dimension::EnergyStability,
            value: Fixed::ONE,
        },
        b"objective=energy",
    );
    life.edit(
        "targets",
        FormEdit::EncounterTargets {
            encounter: 0,
            count: 2,
        },
        ACCEPTED_UI,
    );
    let changed = active
        .restore(None)
        .unwrap()
        .with_preferences(AllocationPreferences {
            game_lock_policy: GameLockPolicy::AllowReassignment,
            ..AllocationPreferences::default()
        })
        .unwrap();
    let backup = Arc::new(Backup::from_validated(&changed).unwrap());
    let (head, mut registry) = life.prepare(backup, "preferences", true);
    assert!(life.dom.promote_session_anchor(&head).is_err());
    let target = transition(&life.dom, &head, ACCEPTED_UI);
    assert_eq!(target.policy, CheckpointPolicy::PreserveAccepted);
    let mut wrong = target.clone();
    wrong.previous_checkpoint = None;
    assert!(life.dom.stage_account_transition(wrong).is_err());
    life.dom.stage_account_transition(target.clone()).unwrap();
    assert!(
        life.dom
            .retain_checkpoint(target.checkpoint.clone())
            .is_err()
    );
    assert!(life.dom.stage_account_transition(target.clone()).is_err());
    life.promote(&head);
    assert_eq!(life.dom.checkpoint(), Some(&target.checkpoint));
    life.activate(&head, &mut registry, StorageDisposition::SessionOnly);
    let mut life = life.reconstruct();
    assert_eq!(
        life.dom.checkpoint().unwrap().payload.sha256,
        Digest::of(ACCEPTED_UI)
    );
    assert_eq!(life.checkpoints.revision(), Counter(3));
    let (replacement, _) =
        life.prepare(private_backup("different-account").0, "replacement", false);
    let mut target = transition(&life.dom, &replacement, ACCEPTED_UI);
    let correct = target.clone();
    target.policy = CheckpointPolicy::PreserveAccepted;
    assert!(life.dom.stage_account_transition(target).is_err());
    let mut wrong = correct.clone();
    if let CheckpointPolicy::ConfirmedReplacement { confirmation } = &mut wrong.policy {
        confirmation.checkpoint.sha256 = Digest::of(b"unconfirmed reset");
    }
    assert!(life.dom.stage_account_transition(wrong).is_err());
    life.dom.stage_account_transition(correct).unwrap();
}

#[test]
fn actual_ordered_refresh_is_not_absorbing_and_failed_stale_or_unsettled_reads_cannot_authorize_cas()
 {
    let old = Arc::new(DurableRecord {
        revision: Counter(7),
        backup: Some(private_backup("saved-a").0),
    });
    let known = DurableHead::from_record(&old).unwrap();
    for storage in [
        StorageObservation::Unavailable {
            last_observed_head: None,
        },
        StorageObservation::Unreadable {
            last_known_head: None,
        },
        StorageObservation::Readable {
            record: old.clone(),
        },
    ] {
        let retained = private_backup("active-b").0;
        let mut life = Lifecycle::bootstrap(
            RecoveryInput {
                storage,
                active: ActiveSelection::RetainedSession {
                    backup: retained.clone(),
                },
            },
            context().epochs,
            Counter(0),
            None,
        );
        let refresh = life.dom.begin_storage_refresh(id("failed-read")).unwrap();
        let failed = HeadObservation::Unavailable {
            last_readable: life.dom.storage().head().cloned(),
        };
        let mut stale = refresh.clone();
        stale.epochs.worker = Counter(99);
        assert!(
            life.dom
                .finish_storage_refresh(&stale, failed.clone())
                .is_err()
        );
        life.dom
            .finish_storage_refresh(&refresh, failed.clone())
            .unwrap();
        life.machine
            .refresh_storage(
                &life.session.context().epochs,
                StorageObservation::Unavailable {
                    last_observed_head: failed.head().cloned(),
                },
            )
            .unwrap();
        assert_eq!(life.dom.storage(), &failed);
        let refresh = life.dom.begin_storage_refresh(id("actual-read")).unwrap();
        let success = StorageObservation::Readable {
            record: old.clone(),
        };
        life.machine
            .refresh_storage(&life.session.context().epochs, success.clone())
            .unwrap();
        life.dom
            .finish_storage_refresh(&refresh, success.head_observation().unwrap())
            .unwrap();
        assert_eq!(life.machine.active(), Some(&retained));
        assert_eq!(life.machine.durable_head().unwrap(), Some(known.clone()));
        let refresh = life.dom.begin_storage_refresh(id("stale-read")).unwrap();
        for record in [
            DurableRecord {
                revision: Counter(6),
                backup: old.backup.clone(),
            },
            DurableRecord {
                revision: Counter(7),
                backup: None,
            },
        ] {
            let observation = StorageObservation::Readable {
                record: Arc::new(record),
            };
            assert!(
                life.dom
                    .finish_storage_refresh(&refresh, observation.head_observation().unwrap())
                    .is_err()
            );
            assert!(
                life.machine
                    .refresh_storage(&life.session.context().epochs, observation)
                    .is_err()
            );
        }
        life.dom
            .finish_storage_refresh(
                &refresh,
                HeadObservation::Readable {
                    head: known.clone(),
                },
            )
            .unwrap();
        life.session.restart_worker(&life.machine).unwrap();
        let input = life.machine.recovery_input();
        let bootstrap = gate_request(
            &life.session,
            "ordered-logical-bootstrap",
            Request::Bootstrap {
                input: input.clone(),
            },
        );
        life.session.register(&bootstrap).unwrap();
        assert_eq!(
            input.active,
            ActiveSelection::RetainedSession { backup: retained }
        );
        let mut life = life.reconstruct();
        let (head, _) = life.prepare(private_backup("candidate-c").0, "save-after-read", false);
        life.dom
            .stage_account_transition(transition(&life.dom, &head, ACCEPTED_UI))
            .unwrap();
        let (reservation, _) = life.reserve(&head);
        assert_eq!(reservation.expected, known);
        assert_eq!(reservation.proposed.revision, Counter(8));
        assert!(life.dom.begin_storage_refresh(id("write-bypass")).is_err());
        assert!(
            life.machine
                .refresh_storage(&life.session.context().epochs, success)
                .is_err()
        );
        assert_eq!(life.dom.phase(), &DomPhase::Reserved);
    }
}

#[test]
fn session_preparation_at_max_revision_is_independent_from_save_and_guarded_reinitialization() {
    let saved = Arc::new(DurableRecord {
        revision: Counter(u64::MAX),
        backup: Some(private_backup("saved-max").0),
    });
    let head = DurableHead::from_record(&saved).unwrap();
    let mut life = Lifecycle::bootstrap(observed(saved), context().epochs, Counter(0), None);
    let (prepared, _) = life.prepare(private_backup("session-at-max").0, "max-session", false);
    assert_eq!(
        life.session
            .propose_prepared_save(&mut life.machine, &prepared.token)
            .unwrap_err()
            .code,
        IssueCode::ResourceLimit
    );
    life.dom
        .stage_account_transition(transition(&life.dom, &prepared, ACCEPTED_UI))
        .unwrap();
    life.promote(&prepared);
    let mut life = life.reconstruct();
    assert_eq!(life.machine.durable_head().unwrap(), Some(head));
    assert_eq!(life.dom.intended().unwrap().snapshot, id("session-at-max"));
    let authority = life.dom.begin_clear(id("explicit-reset")).unwrap();
    let b = binding(authority.clone());
    life.dom.bind(b.clone()).unwrap();
    let observation = life.dom.storage().clone();
    let plan = life
        .dom
        .reserve_privacy_clear(&b, id("fresh-store"))
        .unwrap();
    assert!(matches!(plan, ClearPlan::Reinitialize { .. }));
    let mut clear =
        PrivacyClearMachine::reserve(authority, observation, id("fresh-store")).unwrap();
    let clear_authority = clear.authority().clone();
    clear.transaction_complete(&clear_authority, &plan).unwrap();
    life.dom.privacy_clear_complete(&b, &plan).unwrap();
    life.machine.apply_privacy_clear(&clear).unwrap();
    life.session.install_privacy_clear(&clear).unwrap();
    let restored = life.reconstruct();
    assert_eq!(
        restored.machine.durable_head().unwrap().unwrap().revision,
        Counter(1)
    );
    assert_eq!(restored.dom.intended(), None);
    assert_eq!(restored.dom.checkpoint(), None);
    assert_eq!(restored.dom.epochs().reset, Counter(1));
}

#[test]
fn retained_source_budget_counts_older_saved_bodies_and_compacts_them_without_losing_cas() {
    for shared_candidate in [false, true] {
        let (saved, saved_source) = private_backup("saved-a");
        let (active, active_source) = private_backup("active-b");
        let record = Arc::new(DurableRecord {
            revision: Counter(7),
            backup: Some(saved),
        });
        let head = DurableHead::from_record(&record).unwrap();
        let caller_held_saved_body = record.clone();
        let mut machine = CommitMachine::new(
            context().epochs,
            true,
            RecoveryInput {
                storage: StorageObservation::Readable { record },
                active: ActiveSelection::RetainedSession { backup: active },
            },
        )
        .unwrap();
        assert_eq!(saved_source.strong_count(), 1);
        assert!(active_source.strong_count() > 0);
        drop(caller_held_saved_body);
        assert_eq!(saved_source.strong_count(), 0);
        let (candidate, candidate_source) = if shared_candidate {
            (machine.active().unwrap().clone(), active_source.clone())
        } else {
            private_backup("candidate-c")
        };
        machine.prepare(Some(candidate), &context().epochs).unwrap();
        let observers = [&saved_source, &active_source, &candidate_source];
        let distinct = observers
            .iter()
            .enumerate()
            .filter(|(i, source)| {
                source.strong_count() > 0
                    && !observers[..*i]
                        .iter()
                        .any(|previous| Weak::ptr_eq(previous, source))
            })
            .count();
        assert_eq!(distinct, if shared_candidate { 1 } else { 2 });
        assert!(distinct <= work::MAX_RETAINED_SOURCES);
        assert_eq!(machine.durable_head().unwrap(), Some(head.clone()));
        assert!(matches!(
            machine.recovery_input().storage,
            StorageObservation::RetainedHead { .. }
        ));
        let record = machine.reserve_save(&context().epochs, &head).unwrap();
        assert_eq!(record.revision, Counter(8));
        machine.storage_failed(&context().epochs).unwrap();
        assert_eq!(machine.durable_head().unwrap(), Some(head.clone()));
        machine.reserve_save(&context().epochs, &head).unwrap();
        machine
            .storage_committed(&receipt(&record, &context().epochs))
            .unwrap();
        let input = machine.recovery_input();
        drop(machine);
        let recovered = CommitMachine::new(context().epochs, true, input).unwrap();
        assert_eq!(
            recovered.active().unwrap().account.id,
            id(if shared_candidate {
                "active-b"
            } else {
                "candidate-c"
            })
        );
        let unread_body = RecoveryInput {
            storage: StorageObservation::RetainedHead { head },
            active: ActiveSelection::ObservedDurable,
        };
        assert!(unread_body.restore(None).is_err());
        assert!(CommitMachine::new(context().epochs, true, unread_body).is_err());
    }
}

#[test]
fn catalog_asset_components_are_checked_independently_of_matching_paths_and_semantic_ids() {
    for revision in ["revision-1", "v1.2.3", "build_2026-10"] {
        live_catalog(revision).validate().unwrap();
    }
    for revision in [
        ".",
        "..",
        "../escape",
        "a/../b",
        "/absolute",
        "https://elsewhere/path",
        "a:b",
        "a..b",
        "//host/path",
        "a%2fb",
        "a%252fb",
        "a\\b",
        "a?query",
        "a#fragment",
    ] {
        if RevisionId::try_from(revision.to_owned()).is_ok() {
            assert!(live_catalog(revision).validate().is_err(), "{revision}");
        } else {
            assert!(wire::decode::<RevisionId>(&format!("\"{revision}\"")).is_err());
        }
    }
    assert!(DefinitionId::try_from("semantic:namespace/path".to_owned()).is_ok());
}
fn live_catalog(revision: &str) -> Catalog {
    let mut catalog = catalog();
    let mechanics = MechanicsRef::Live {
        game_version: "unverified-contract-case".into(),
        revision: id(revision),
        sha256: Digest::of(b"no live mechanics"),
    };
    catalog.mechanics[0].mechanics = mechanics.clone();
    catalog.mechanics[0].presets[0].spec.mechanics = mechanics;
    catalog.mechanics[0].path = format!("data/mechanics/live/{revision}.json");
    catalog
}

#[test]
fn current_unreadable_head_preserves_past_authority_and_reaches_guarded_clear_with_retained_pair() {
    for schedule in 0..5 {
        let (saved, saved_source) = private_backup("saved-a7");
        let record = Arc::new(DurableRecord {
            revision: Counter(7),
            backup: Some(saved),
        });
        let known = DurableHead::from_record(&record).unwrap();
        let (active, active_source) = private_backup("retained-b");
        let intended = account_head(&active);
        let mut life = Lifecycle::bootstrap(
            RecoveryInput {
                storage: StorageObservation::Readable { record },
                active: ActiveSelection::RetainedSession { backup: active },
            },
            context().epochs,
            Counter(0),
            None,
        );
        assert_eq!(saved_source.strong_count(), 0);
        life.edit(
            "accepted-ui",
            FormEdit::EncounterTargets {
                encounter: 0,
                count: 2,
            },
            ACCEPTED_UI,
        );
        let accepted = life.dom.checkpoint().cloned().unwrap();
        let authority = life
            .dom
            .begin_storage_refresh(id("actual-unreadable-header"))
            .unwrap();
        let observation = HeadObservation::Unreadable {
            last_known_head: Some(known.clone()),
        };
        let mut stale = authority.clone();
        stale.epochs.worker = Counter(99);
        assert!(
            life.dom
                .finish_storage_refresh(&stale, observation.clone())
                .is_err()
        );
        assert_eq!(life.dom.phase(), &DomPhase::Refreshing);
        assert!(life.dom.begin_clear(id("read-not-settled")).is_err());
        for wrong in [
            HeadObservation::Unreadable {
                last_known_head: None,
            },
            HeadObservation::Unavailable {
                last_readable: None,
            },
        ] {
            assert!(life.dom.finish_storage_refresh(&authority, wrong).is_err());
        }
        life.machine
            .refresh_storage(
                &life.session.context().epochs,
                StorageObservation::Unreadable {
                    last_known_head: Some(known.clone()),
                },
            )
            .unwrap();
        life.dom
            .finish_storage_refresh(&authority, observation.clone())
            .unwrap();
        assert_eq!(life.dom.storage(), &observation);
        assert_eq!(life.machine.durable_head().unwrap(), Some(known.clone()));
        assert_eq!(life.dom.intended(), Some(&intended));
        assert_eq!(life.dom.checkpoint(), Some(&accepted));
        let mut life = life.reconstruct();
        assert_eq!(life.dom.storage(), &observation);
        let read = life
            .dom
            .begin_storage_refresh(id("regression-after-unreadable"))
            .unwrap();
        for bad in [
            DurableRecord {
                revision: Counter(6),
                backup: None,
            },
            DurableRecord {
                revision: Counter(7),
                backup: None,
            },
        ] {
            let bad = StorageObservation::Readable {
                record: Arc::new(bad),
            };
            assert!(
                life.dom
                    .finish_storage_refresh(&read, bad.head_observation().unwrap())
                    .is_err()
            );
            let mut recovery = life.machine.recovery_input();
            recovery.storage = bad.clone();
            assert!(
                life.machine
                    .bootstrap(life.session.context().epochs.clone(), recovery)
                    .is_err()
            );
            assert!(
                life.machine
                    .refresh_storage(&life.session.context().epochs, bad)
                    .is_err()
            );
        }
        let mut older = known.clone();
        older.revision = Counter(6);
        assert!(
            life.dom
                .finish_storage_refresh(
                    &read,
                    HeadObservation::Corrupt {
                        readable_head: older
                    }
                )
                .is_err()
        );
        let mut invented = known.clone();
        invented.revision = Counter(8);
        assert!(
            life.dom
                .finish_storage_refresh(
                    &read,
                    HeadObservation::Unreadable {
                        last_known_head: Some(invented)
                    }
                )
                .is_err()
        );
        life.dom
            .finish_storage_refresh(&read, observation.clone())
            .unwrap();
        let authority = life.dom.begin_clear(id("confirmed-clear")).unwrap();
        life.session.reset_barrier().unwrap();
        assert_eq!(&authority.epochs, &life.session.context().epochs);
        let b = binding(authority.clone());
        life.dom.bind(b.clone()).unwrap();
        assert_eq!(life.dom.intended(), Some(&intended));
        assert_eq!(life.dom.checkpoint(), Some(&accepted));
        let plan = life
            .dom
            .reserve_privacy_clear(&b, id("fresh-exact-store"))
            .unwrap();
        let ClearPlan::Reinitialize {
            new_store,
            tombstone,
        } = &plan
        else {
            panic!("unreadable CURRENT header cannot use old-head CAS");
        };
        assert_eq!(new_store, &id("fresh-exact-store"));
        assert_eq!(tombstone.revision, Counter(1));
        assert_eq!(life.dom.phase(), &DomPhase::Reserved);
        assert!(life.dom.cancel().is_err());
        assert!(life.dom.crash_after_settlement().is_err());
        let mut clear = PrivacyClearMachine::reserve(
            authority.clone(),
            observation.clone(),
            id("fresh-exact-store"),
        )
        .unwrap();
        assert!(!clear.erased());
        assert!(clear.cleared_bootstrap().is_err());
        if schedule == 0 {
            life.dom.privacy_clear_aborted(&b).unwrap();
            clear.transaction_aborted(&authority).unwrap();
            life.session
                .abort_reset_before_save(&mut life.machine)
                .unwrap();
            assert!(!clear.erased());
            let restored = life.reconstruct();
            assert_eq!(restored.dom.storage(), &observation);
            assert_eq!(restored.machine.durable_head().unwrap(), Some(known));
            assert_eq!(restored.dom.intended(), Some(&intended));
            assert_eq!(restored.dom.checkpoint(), Some(&accepted));
            assert!(active_source.strong_count() > 0);
            continue;
        }
        if schedule >= 3 {
            let mut observed = StoredHead {
                store: id("wrong-old-store"),
                head: tombstone.clone(),
            };
            assert!(life.dom.settle_privacy_clear(&b, &observed).is_err());
            assert!(clear.settle(&authority, &observed).is_err());
            assert_eq!(life.dom.phase(), &DomPhase::Reserved);
            assert_eq!(life.dom.intended(), Some(&intended));
            observed.store = id("fresh-exact-store");
            life.dom
                .settle_privacy_clear(&b, &observed)
                .unwrap()
                .unwrap();
            clear.settle(&authority, &observed).unwrap();
        } else {
            life.dom.privacy_clear_complete(&b, &plan).unwrap();
            clear.transaction_complete(&authority, &plan).unwrap();
        }
        assert!(clear.erased());
        life.machine.apply_privacy_clear(&clear).unwrap();
        life.session.install_privacy_clear(&clear).unwrap();
        assert_eq!(life.dom.intended(), None);
        assert_eq!(life.dom.checkpoint(), None);
        if schedule == 2 || schedule == 4 {
            let input = life.machine.recovery_input();
            let req = gate_request(
                &life.session,
                "clear-ack",
                Request::Bootstrap {
                    input: input.clone(),
                },
            );
            life.session.register(&req).unwrap();
            let activation = ActionBinding {
                authority,
                step: Counter(1),
                operation: req.fingerprint.clone(),
            };
            life.dom.bind_activation(activation.clone()).unwrap();
            life.machine
                .bootstrap(life.session.context().epochs.clone(), input)
                .unwrap();
            assert!(life.session.accept(&reply(
                &req,
                Event::ResetAcknowledged {
                    previous_generation: Counter(2)
                },
                Counter(3)
            )));
            life.dom
                .activation_acknowledged(&RestorationAck {
                    binding: activation,
                    account: None,
                    generation: Counter(3),
                    checkpoint: None,
                })
                .unwrap();
        }
        let restored = life.reconstruct();
        assert_eq!(
            restored.dom.generation(),
            Counter(if schedule == 2 || schedule == 4 { 4 } else { 3 })
        );
        assert_eq!(restored.machine.active(), None);
        assert_eq!(restored.dom.intended(), None);
        assert_eq!(restored.dom.checkpoint(), None);
        assert_eq!(
            restored.machine.durable_head().unwrap().unwrap().revision,
            Counter(1)
        );
        assert_eq!(active_source.strong_count(), 0);
    }
}

#[test]
fn unavailable_after_current_unreadability_never_authorizes_erasure_or_forgets_history() {
    let known = DurableHead::from_record(&DurableRecord {
        revision: Counter(7),
        backup: None,
    })
    .unwrap();
    let mut dom = ready_dom(
        HeadObservation::Readable {
            head: known.clone(),
        },
        None,
    );
    let read = dom.begin_storage_refresh(id("unreadable")).unwrap();
    dom.finish_storage_refresh(
        &read,
        HeadObservation::Unreadable {
            last_known_head: Some(known.clone()),
        },
    )
    .unwrap();
    let read = dom.begin_storage_refresh(id("read-failed")).unwrap();
    dom.finish_storage_refresh(
        &read,
        HeadObservation::Unavailable {
            last_readable: Some(known.clone()),
        },
    )
    .unwrap();
    let b = binding(dom.begin_clear(id("cannot-clear-unavailable")).unwrap());
    dom.bind(b.clone()).unwrap();
    assert!(dom.reserve_privacy_clear(&b, id("not-authorized")).is_err());
    assert_eq!(dom.phase(), &DomPhase::Bulk);
    assert_eq!(dom.storage().head(), Some(&known));
}

#[derive(Debug, Clone, Copy)]
enum Completion {
    CancelProposal,
    AbortThenCancel,
    CommitBeforeAck,
    CommitAfterAck,
    MemoryBeforeAck,
}

#[test]
fn bounded_composed_lifecycle_matrix_reconstructs_exact_old_or_new_pairs() {
    for initial in 0..4 {
        for completion in [
            Completion::CancelProposal,
            Completion::AbortThenCancel,
            Completion::CommitBeforeAck,
            Completion::CommitAfterAck,
            Completion::MemoryBeforeAck,
        ] {
            let (saved, saved_source) = private_backup("saved-a");
            let saved_record = Arc::new(DurableRecord {
                revision: Counter(7),
                backup: Some(saved),
            });
            let saved_head = DurableHead::from_record(&saved_record).unwrap();
            let input = match initial {
                0 => observed(DurableRecord {
                    revision: Counter(0),
                    backup: None,
                }),
                1 => observed(saved_record.clone()),
                2 => unread(),
                _ => RecoveryInput {
                    storage: StorageObservation::Corrupt {
                        head: saved_head.clone(),
                    },
                    active: ActiveSelection::EmptySession,
                },
            };
            let mut life = Lifecycle::bootstrap(input, context().epochs, Counter(0), None);
            assert_eq!(life.dom.generation(), Counter(1));
            life.edit(
                "objective",
                FormEdit::ObjectiveWeight {
                    dimension: Dimension::EnergyStability,
                    value: Fixed::ONE,
                },
                b"objective=energy",
            );
            life.edit(
                "target-form",
                FormEdit::EncounterTargets {
                    encounter: 0,
                    count: 2,
                },
                ACCEPTED_UI,
            );
            let (session_b, source_b) = private_backup("active-b");
            let head_b = account_head(&session_b);
            let (prepared_b, mut registry_b) = life.prepare(session_b, "import-b", false);
            let target_b = transition(&life.dom, &prepared_b, ACCEPTED_UI);
            life.dom.stage_account_transition(target_b.clone()).unwrap();
            life.promote(&prepared_b);
            life.activate(
                &prepared_b,
                &mut registry_b,
                StorageDisposition::SessionOnly,
            );
            drop(registry_b);
            assert_eq!(life.dom.generation(), Counter(2));
            assert_eq!(life.dom.checkpoint().unwrap().revision, Counter(3));
            let refresh = life
                .dom
                .begin_storage_refresh(id("actual-saved-read"))
                .unwrap();
            life.machine
                .refresh_storage(
                    &life.session.context().epochs,
                    StorageObservation::Readable {
                        record: saved_record.clone(),
                    },
                )
                .unwrap();
            life.dom
                .finish_storage_refresh(
                    &refresh,
                    HeadObservation::Readable {
                        head: saved_head.clone(),
                    },
                )
                .unwrap();
            drop(saved_record);
            assert_eq!(saved_source.strong_count(), 0);
            let mut life = life.reconstruct();
            assert_eq!(life.dom.generation(), Counter(3));
            assert_eq!(life.dom.epochs().worker, Counter(1));
            let (candidate_c, source_c) = private_backup("candidate-c");
            let head_c = account_head(&candidate_c);
            let (prepared_c, mut registry_c) = life.prepare(candidate_c, "import-c", false);
            let target_c = transition(&life.dom, &prepared_c, ACCEPTED_UI);
            assert!(life.dom.promote_session_anchor(&prepared_c).is_err());
            let proposal = life
                .session
                .propose_prepared_save(&mut life.machine, &prepared_c.token)
                .unwrap();
            let bytes = wire::encode(&proposal).unwrap();
            let reservation = StorageReservation {
                binding: prepared_c.binding.clone(),
                token: prepared_c.token.clone(),
                expected: saved_head.clone(),
                proposed: DurableHead::from_record(&proposal).unwrap(),
                record: descriptor(
                    &prepared_c.binding,
                    BulkPurpose::DurableRecord,
                    bytes.as_bytes(),
                ),
            };
            assert!(life.dom.reserve(reservation.clone()).is_err());
            life.dom.stage_account_transition(target_c.clone()).unwrap();
            assert_eq!(life.dom.intended(), Some(&head_b));
            assert_eq!(life.dom.checkpoint(), Some(&target_b.checkpoint));
            assert_eq!(life.machine.phase(), &CommitPhase::Prepared);
            assert_eq!(
                [&saved_source, &source_b, &source_c]
                    .iter()
                    .filter(|s| s.strong_count() > 0)
                    .count(),
                2
            );
            let (expected_account, expected_ui, expected_revision, expected_generation) =
                match completion {
                    Completion::CancelProposal => {
                        life.session
                            .cancel_prepared_save(&mut life.machine)
                            .unwrap();
                        life.dom.cancel().unwrap();
                        (head_b, target_b.checkpoint, 7, 4)
                    }
                    Completion::AbortThenCancel => {
                        life.dom.reserve(reservation.clone()).unwrap();
                        life.session
                            .reserve_prepared_save(&mut life.machine, &saved_head)
                            .unwrap();
                        assert!(life.dom.cancel().is_err());
                        life.dom.transaction_aborted(&prepared_c.binding).unwrap();
                        life.session.storage_failed(&mut life.machine).unwrap();
                        assert_eq!(life.dom.intended(), Some(&head_b));
                        assert_eq!(life.dom.checkpoint(), Some(&target_b.checkpoint));
                        life.dom.cancel().unwrap();
                        (head_b, target_b.checkpoint, 7, 4)
                    }
                    Completion::CommitBeforeAck | Completion::CommitAfterAck => {
                        life.dom.reserve(reservation.clone()).unwrap();
                        life.session
                            .reserve_prepared_save(&mut life.machine, &saved_head)
                            .unwrap();
                        let receipt = life.dom.transaction_complete(&prepared_c.binding).unwrap();
                        life.session
                            .storage_committed(&mut life.machine, &receipt)
                            .unwrap();
                        assert_eq!(life.dom.intended(), Some(&head_c));
                        assert_eq!(life.dom.checkpoint(), Some(&target_c.checkpoint));
                        assert!(life.dom.cancel().is_err());
                        if matches!(completion, Completion::CommitAfterAck) {
                            life.activate(
                                &prepared_c,
                                &mut registry_c,
                                StorageDisposition::Persisted { receipt },
                            );
                        }
                        (
                            head_c,
                            target_c.checkpoint,
                            8,
                            if matches!(completion, Completion::CommitAfterAck) {
                                5
                            } else {
                                4
                            },
                        )
                    }
                    Completion::MemoryBeforeAck => {
                        life.dom.reserve(reservation.clone()).unwrap();
                        life.session
                            .reserve_prepared_save(&mut life.machine, &saved_head)
                            .unwrap();
                        life.dom.transaction_aborted(&prepared_c.binding).unwrap();
                        life.session.storage_failed(&mut life.machine).unwrap();
                        life.promote(&prepared_c);
                        (head_c, target_c.checkpoint, 7, 4)
                    }
                };
            drop(registry_c);
            drop(proposal);
            let restored = life.reconstruct();
            assert_eq!(
                restored.dom.intended(),
                Some(&expected_account),
                "{initial}/{completion:?}"
            );
            assert_eq!(
                restored.dom.checkpoint(),
                Some(&expected_ui),
                "{initial}/{completion:?}"
            );
            assert_eq!(
                restored.machine.durable_head().unwrap().unwrap().revision,
                Counter(expected_revision)
            );
            assert_eq!(
                restored.machine.durable_head().unwrap(),
                Some(if expected_revision == 8 {
                    reservation.proposed.clone()
                } else {
                    saved_head.clone()
                })
            );
            assert_eq!(restored.dom.generation(), Counter(expected_generation));
            assert_eq!(restored.dom.epochs().worker, Counter(2));
            assert_eq!(restored.dom.epochs().reset, Counter(0));
            assert_eq!(
                restored.machine.active().map(|b| account_head(b)),
                Some(expected_account)
            );
            assert!(
                [&saved_source, &source_b, &source_c]
                    .iter()
                    .filter(|s| s.strong_count() > 0)
                    .count()
                    <= work::MAX_RETAINED_SOURCES
            );
        }
    }
}
