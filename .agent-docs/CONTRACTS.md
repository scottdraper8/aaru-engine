# Shared schema 2 — accepted material native subgate

`crates/aaru-contract` owns the shared Rust types, validators and pure state
machines. This document defines adapter obligations. **Native G1 was accepted at
1b66491/run40582912311. H0-M material/strict-shape native subgate is accepted at
50119ab/run40609987311 after independent/staged/hosted gates. Broader P1a/H0
public-interface/runtime gates remain; engine/UI consumers must not start yet.** The sibling-worker design
is approved; its implementation, timing, platform feasibility and product are not.
No engine, UI builder source or WorkerEvent implementation is supplied here.

## Ownership, assets and privacy

| Context / owner | Responsibility |
| --- | --- |
| Contract coordinator | Contract crate, shared schemas, this document, necessary root dependency integration |
| DOM controller (`aaru-web`) | Both sibling module Worker handles, their private MessageChannel, compact authority/cancellation gate, Web Lock, actual IndexedDB callbacks, opaque immutable account/UI anchors, bounded rendering and download URL |
| UI supervisor (`aaru-web`, worker mode) | Full private UI model, setup edits, logical JSON codecs/fingerprints, SessionGate, result validation, save preparation, checkpoint encoding, bounded presentation pages |
| Engine (`aaru-wasm` -> engine + contract) | Imports, fresh trusted sample verification, validated active/candidate accounts, mechanics/search, contextual legality/results, prepared registry and activation |
| Parent | Build/bootstrap/server/CI/security/policy and acceptance/release coverage |

Web depends only on contract, never engine-private datasets or a duplicate scoring
ontology. Engine owns public dataset/catalog content, provenance, hashes and fixture
oracles; contract owns their schemas. UI must not fabricate engine responses.
All first-party application/UI/tests/tooling are Rust. Static HTML/CSS and generated
initialization-only WASM glue/loaders are allowed. Follow `PRODUCT_LANGUAGE.md`.

DOM creates exactly TWO directly owned sibling module Workers and one MessageChannel,
transferring one distinct endpoint to each in its Bootstrap control. No nested-worker,
SharedArrayBuffer, BroadcastChannel leader election or service-worker assumption.
Each worker installs Rust bootstrap/MessagePort listeners; UI startup selects Window
versus supervisor behavior. Loaders only initialize colocated generated bindings/WASM.
They contain no routing, account parsing, storage or cancellation logic.

| Asset | Base-relative path / shared constant |
| --- | --- |
| Engine loader / bindings | `pkg/aaru-wasm/worker.js` / `pkg/aaru-wasm/aaru_wasm.js` (`WORKER_MODULE_PATH`, `WORKER_BINDINGS_PATH`) |
| DOM loader | `pkg/aaru-web/main.js` (`WEB_MODULE_PATH`) |
| Supervisor loader / UI bindings | `pkg/aaru-web/supervisor.js` / `pkg/aaru-web/aaru_web.js` (`SUPERVISOR_MODULE_PATH`, `WEB_BINDINGS_PATH`) |
| Catalog / mechanics manifest | `fixtures/catalog.json` / `data/mechanics/manifest.json` |

Both UI loaders initialize the same UI package; generated bindings load their
colocated `_bg.wasm`. Resolve all assets against one configured same-origin base
ending in `/`, including `/aaru-engine/`, never a mutable route/imported URL.
Sample IDs are `syn-welcome`, `syn-scarcity`, `syn-stress`, `good-unsupported-preview`.
Synthetic accounts use `fixtures/canonical/<id>.json` and mechanics
`data/mechanics/synthetic/<id>.json`. Preview uses
`fixtures/good/good-unsupported-preview.json` with no synthetic mechanics. Live
mechanics paths are `data/mechanics/live/<revision>.json`. Catalog validation checks
live revision filename components independently of semantic ID validity: ASCII
letters/digits at both ends, only letters/digits/`-_.` inside, no `..`. Separators,
URL syntax and encoded separators/traversal are not filenames. Matching a path
interpolated from an unsafe revision is insufficient. Semantic IDs retain their
namespace/path grammar; this does not verify live mechanics. Manifest is
`Vec<MechanicsEntry>`; validate it against the Catalog before use.

Hold exclusive Web Lock `aaru-primary-v1` before any private storage/anchor/work;
hold it across worker restarts, settlement and session-only operation. Secondary
tabs or browsers without Web Locks provide public help only. Ownership loss stops
both workers and invalidates authority, but does not prove an outstanding write
aborted. A successor primary performs ordered storage recovery before writing.

Dedicated trustworthy origin required: path/IDB prefixes are not same-origin access
isolation. No private account/preferences/source/download data in network requests,
logs or telemetry; no credentials, account upload or game automation. CSP restricts
scripts/workers/fetches to same-origin, permits WASM compilation via
`script-src 'self' 'wasm-unsafe-eval'`, forbids arbitrary inline/eval application
code, framing, objects, blob worker loaders and external fonts/scripts/assets.
Parent's loopback Rust server serves only an allowlisted public asset root, never
the repository; traversal, encoded traversal, backslashes, symlink escapes,
directory listing, correct WASM MIME and subpath hosting require actual tests.

## Two protocols, one authority

### Logical worker-to-worker API

`WorkerRequest = Envelope<Request>` and `WorkerEvent = Envelope<Event>` travel
DIRECTLY over the private supervisor–engine ports as verified UTF-8 JSON byte
attachments. Rust `wire::decode_request` is authoritative for structural decoding
and operation fingerprints; contextual account/spec/result checks are additional.
Never JSON.parse imports, convert accounts through JS objects/Number, replace
malformed UTF-8, regenerate original source bytes or interpret numeric lexemes
through floating point. Source identity includes original whitespace/CRLF/lexemes.

Envelope binds protocol version, RequestId, generation, RequestContext (session,
worker incarnation, reset epoch, active semantic digest, exact engine identity)
and SHA-256 of compact Rust JSON tuple
`["aaru-operation-v1", context, generation, request]`. Events echo context and
fingerprint. Account transitions echo OLD context and carry generation exactly +1;
matching snapshot/semantic acknowledgement gates usability. Engine build identity
comes from parent build metadata, not guessed package versions. Digests identify
integrity/context, not signatures or proof of database completion.

Requests: GetCatalog, PrepareImport (File with explicit declarations / Sample /
Restore), PreparePreferences, ActivatePrepared, Bootstrap { input: RecoveryInput },
Evaluate, Compare, Sensitivity, Export, cooperative Cancel, Reset, DiscardSession.
There is no Recover alias. Events carry real catalog/preview/results/downloads,
progress/errors and exact transition acknowledgements. Cancellation by physical
termination is not a fabricated logical CancelAcknowledged event.

`SessionGate` lives ONLY in the supervisor, never DOM. `wire::decode` establishes
structural shape, not contextual validity. `accept` admits only non-result events
by context/order; it deliberately refuses Evaluated, Compared and SensitivityCompleted.
For those three terminals the supervisor MUST call `accept_result(event, account)`
with its actual validated active account BEFORE forwarding, paginating, rendering
or caching authoritative output. It calls the corresponding full `validate_for`
(including nested results/references), then context/order admission. A validation
error leaves the request pending. Engine also validates before sending. These
whole-result operations remain off DOM in terminable bulk work; no DOM account
codec/hash is authorized. On initial bootstrap call
`begin_bootstrap` with the actual expected RecoveryInput before registering the
Bootstrap envelope. A physically replaced supervisor reconstructs its gate and
CommitMachine from DOM's compact context and verified opaque anchors; Rust objects
do not survive termination. For continuous pure-state reasoning, `restart_worker`
increments incarnation, clears
issued IDs/previews/work, and admits only the exact retained bootstrap input.
Writing must settle first. A matching bootstrap also advances generation +1.
Reset armed/proposed work needs its explicit abort before ordinary restart;
committed reset recovery needs no second clear. IDs/tokens cannot repeat within
an incarnation (4,096 limit); checked counters never wrap. One authoritative
foreground request; late/reversed/cancelled/mismatched/duplicate terminals fail.

### Compact physical API (`transport.rs`)

Only `ControlFrame` uses `encode_control` / `decode_control`; maximum **16 KiB
encoded UTF-8**, including pages. Strict JSON guards reject unknown fields,
duplicate/escaped-equivalent keys, BOMs, trailing data and resource abuse. No
arbitrary JSON value/payload escape hatch. Endpoint routing is typed: DOM owns
Bootstrap; logical requests go supervisor->engine and events engine->supervisor;
DOM edits/permissions go to supervisor; prepared/storage/presentation controls go
supervisor->DOM. WorkerReady/WorkerFailed identify their actual sibling role.

`TransportAuthority` = session/worker/reset Epochs + account generation + action ID.
`ActionBinding` adds step and full logical operation fingerprint. DOM allocates
action authority BEFORE any full-input hash exists. Bind once per step; advancing
a step invalidates old bindings. Each receiving adapter checks endpoint, full
current authority and step before accepting metadata or attachments. DOM must
apply `admits` to progress/errors/acks too, not just presentation/save proposals.

`SourceOffer` describes an UNVERIFIED File: format, attachment ID and byte length,
not a digest or authorization to preview/save/activate. DOM checks size and passes
the opaque File without text()/array_buffer(). Supervisor reads exact bytes and
validates/hashes off-thread before establishing an ActionBinding.

`BulkDescriptor` binds authority, step, unique attachment ID, purpose, encoding,
exact byte length, SHA-256 and operation fingerprint. Every receiving worker
checks the expected purpose, all bindings, byte length/digest and strict UTF-8
before consuming it. `verify_bytes` is a worker helper, NOT a DOM helper. Bootstrap
account/UI anchor bytes (RecoveryBackup, RecoveryRecord or UiCheckpoint) use
`verify_anchor` against retained commitments and fresh
bootstrap authority; no full operation fingerprint is required before bootstrap
input exists. Rebind only physical authority; never change anchor bytes/digest.

The physical postMessage data is exactly a two-element native array:
`[control_json_string, attachment_or_null]`. No additional object properties or
out-of-band payload list. Receiver checks array length2, string slot0, and slot1's
expected native handle or null before decoding. For Bootstrap, transfer list contains
exactly its MessagePort; File/Blob are structured-cloned opaque handles, not parsed
objects or transferable buffers. This layout is identical on Worker and private
MessagePort channels; route validation still distinguishes their permitted senders.
Native File/Blob/MessagePort values are opaque adapter attachments, never serde
account values or web-sys dependencies here. `ControlFrame::attachment` specifies
exactly zero or ONE required handle; adapter rejects extra/missing/wrong-kind,
wrong-ID/length or unexpected-channel attachments (`check_attachment`). Each
Bootstrap transfers exactly one port. Account/checkpoint anchors follow as separate
Bulk controls, not extra hidden Bootstrap attachments. Previously retained
checkpoint descriptors in Bootstrap are commitments, not reuse authorization for
an old port/attachment. Repeated offers/attachment IDs fail closed. A descriptor
is not a capability, trust assertion or permission to fetch arbitrary data.

`PreparedHead` contains token, binding, account snapshot/semantic/backup commitments
and bounded counts/issues. Full ImportPreview.backup remains off DOM.
`DurableHead` commits revision, record digest and AccountHead or tombstone.
`StorageReservation` binds exact prepared token, expected head, next revision,
proposed account/head and DurableRecord Blob descriptor. It is a proposal until
DOM accepts it. DOM constructs receipts from compact precomputed commitments,
not DurableRecord::digest. `PresentationPage` binds operation/view digest/page;
maximum 24 typed rows and 16 KiB total, carrying labels, preset/parameter metadata,
scores/vectors/fidelity, frontier/search distinctions or causal references.
Pagination does not alter alternatives, search completeness or reason graph claims.
Only one page may await acknowledgement. Progress coalesces to the latest value;
it is never an accumulated event queue. At most one current download URL; revoke
on replacement/reset/shutdown. Canonical export is Backup JSON, source export
is the exact preserved source bytes; no archive means source export unavailable.

## Cancellation, checkpoints and backpressure

`DomGate::new` takes the exact intended AccountHead AND optional UiCheckpointHead,
validates their account binding and starts Recovering. `begin_bootstrap`, binding
and matching RestorationAck must establish the intended pair and generation
exactly +1 before foreground work. RestorationAck carries an optional complete
UiCheckpointCommitment: account semantics, private schema, UI revision, byte length
and SHA-256. Both `restored` and `activation_acknowledged` compare that exact
commitment to DOM-retained state; neither accepts a caller-supplied alternative
CheckpointGate as the restoration authority. `Control::Activated` carries this
same acknowledgement, not an ignored revision-only field. Initial unavailable storage remains
unavailable, never an invented revision-zero read. Bootstrap references retained
anchors only on the same primary page; reload cannot inherit ephemeral memory.

Bulk/precommit phases are cancellable even BEFORE binding. `cancel()` immediately
invalidates authority, increments incarnation and returns `TerminatePair`: explicitly
terminate BOTH UiSupervisor and Engine handles and close their old ports. Drop
abandoned candidate/results/pages/download references, render Cancelled/recovering,
then create a new pair. No acknowledgement, timeout, heartbeat or codec completion
is required. Queued old messages fail incarnation/action/step checks. The Rust plan
is not evidence that browser handles were actually terminated.

One replacement intent is retained during recovery; replacement returns/disposes
the previous intent, not a queue. A queued file handle needs a new SourceOffer
under the newly issued action; old authority must never be silently accepted.
While Reserved/Activating/Recovering/Refreshing, user cancellation/reset/replacement is blocked;
worker failure may restart only after actual storage settlement. `finish` releases
ordinary work after page acknowledgement. Reset advances authority before staging;
prewrite abort keeps the advanced epoch and old usable state, never old previews.

Retain one opaque `UiCheckpointHead` + Blob, bound to active account semantics,
private UI schema revision, SHA-256/length and acknowledged UI-state revision.
`CheckpointGate` admits ONE edit at the current base revision. Further independent
edits get backpressure and are NOT accepted yet. Supervisor applies the edit to
its full state, proposes a checkpoint at exactly revision+1; DOM retains that Blob
before acknowledging acceptance to supervisor/player. Retaining metadata alone
is not the adapter's memory commit. No coalescing of accepted edits; no replacing
independent pending edits. Only not-yet-submitted same-control input may coalesce
locally. Cancel abandons only unacknowledged edits and restores the last accepted
checkpoint. After the DOM retains the opaque Blob and successfully calls
`DomGate::retain_checkpoint`, acknowledge acceptance and advance the supervisor's
CheckpointGate. Rejected DOM retention is not an accepted edit. Restart usability
waits for the exact retained pair, never default objectives/targets/form state.

Every prepared account transition, including preferences and same-account changes,
requires `StagedAccountTransition` through `stage_account_transition` BEFORE
`reserve` or `promote_session_anchor`. The StageAccountTransition control carries
one opaque target-checkpoint Blob. Hash/encode/migrate it off DOM; retain that Blob
and candidate account Blob before admitting their metadata. Stage exactly the
previous accepted checkpoint commitment, target account semantics, next UI revision,
private schema, length/digest and current action/step/fingerprint. Once staged,
accepted UI edits and replacement prepared heads are blocked; cancel/finish/next
step discards staging. The accepted old pair remains unchanged until commit.

For the same SnapshotId, including preference changes, `PreserveAccepted` obliges
the UI supervisor to carry ALL previously accepted objective/target/form settings
through any private-schema migration. It is not permission to encode blank defaults.
For actual replacement (different SnapshotId, including the first import), require
`ConfirmedReplacement` with AccountChangeConfirmation naming the exact current
binding, previous/target AccountHeads, target checkpoint commitment and explicit
`CarryAcceptedSettings` OR `ResetAfterConfirmation`. There is NO default reset.
The adapter must obtain an actual player confirmation of that exact policy/target;
changing the target requires a new confirmation. Unresolvable account-specific
settings need explicit resolution during that confirmation, never silent loss.

Transaction complete or session memory promotion installs account AND staged UI
checkpoint together before dispatching activation. Confirmed abort keeps the old
complete pair; a later cancel drops the staged pair. Crash after commit, before or
after acknowledgement, reconstructs the new pair. Reconstruct CheckpointGate from
the retained target head, not an old supervisor Rust object. Activation binds only
the next step, once: an identical binding may replay while pending; a different
fingerprint cannot replace it or displace its authoritative acknowledgement.

Private UI schema, migration and accumulated-setting semantics remain UI-owned.
The contract checks commitments/order, not the meaning of opaque bytes. Production
UI tests must independently demonstrate preservation and actual confirmation.

## Storage, memory commit and recovery

DOM owns IndexedDB `aaru-private-v1`, version1, store `current-record`, key `current`.
Store head and complete DurableRecord Blob together, with a persisted store marker
(`StoredHead.store`). The wrapper is constructed via Rust browser bindings; account
numbers never become JS properties. Hold the Web Lock throughout settlement.

1. Supervisor/engine validate a complete inert prepared Backup, including source,
   preferences and trust; explicit user confirmation precedes replacement/save.
   `CommitMachine::prepare` prepares memory independently of durable revision.
   `proposed_record` lazily constructs/caches the successor record for a SAVE;
   exhaustion rejects that save without wrapping, not session-only activation.
   `propose_prepared_save` / `propose_reset_save` leave CommitMachine Prepared,
   NOT Writing. Prepare and retain the target UI checkpoint as specified above. Whole-input encoding/hashing remains cancellable isolated work.
   `cancel_prepared_save`, or reset's `abort_reset_before_save`, drops staging.
2. DOM requires the retained exact target account/UI pair, validates authoritative
   proposal and expected head, enters Reserved BEFORE
   acknowledging permission or starting any write. Only then supervisor calls
   `reserve_prepared_save` / `reserve_reset_save` to mark logical Writing.
   User actions cannot abandon either component during this barrier.
3. ONE readwrite transaction compares the actual expected revision/head, replaces
   head and Blob together and increments revision exactly once. Enqueue required
   requests from transaction callbacks; no async hashing/unrelated awaits inside.
   Request success is NOT commit. Transaction **complete** is durable commit.
4. Confirmed abort releases reservation; old intended/active/durable state remains.
   Candidate may be retried or explicitly promoted session-only. Unknown outcome
   remains Reserved/Writing: settle transaction, then ordered reread. Compact/full
   settle APIs accept only the exact prior head/record or proposed candidate.
   Unrelated/newer/revised data, a failed read or timeout proves neither outcome.
5. Complete promotes the intended account AND target UI anchors before activation.
   Abort leaves BOTH old anchors intact. Send exact
   receipt; bind ActivatePrepared as the NEXT logical step. Registry checks token,
   epochs, backup/record digests and revision before installing generation+1.
   Usability waits for matching acknowledgement. Registry permits only exact most
   recent activation replay; replay identical original envelope after lost ack,
   before ordinary stale-generation rejection. Never invent a new activation.
6. If crash/lost ack follows commit, recover the committed candidate/tombstone,
   never silently old memory. DOM callbacks remain alive while workers fail. Lost
   callbacks require settlement/reread before restart or another writable session.

`RecoveryInput` separates `StorageObservation` (Readable full record, RetainedHead,
Corrupt with a CURRENT readable head but unreadable payload, Unreadable with an
optional last-known head, Unavailable with an optional last-observed head) from `ActiveSelection` (ObservedDurable, EmptySession, RetainedSession backup).
RetainedHead truthfully preserves the successfully observed revision, record digest
and account/tombstone commitments AFTER releasing an unselected saved body. It is
neither unavailable nor corrupt. It authorizes ordinary exact-head CAS, NOT activation
of an unread/unvalidated body. ObservedDurable requires a complete Readable record.
`durable()` exposes a full retained record only when one exists; use `durable_head()`
for authority, not body availability. Construction, memory promotion/discard,
refresh and bootstrap compact unselected durable bodies; committed candidate
recovery retains its full validated body. Preserve the selected active Backup.
Unavailable is neither empty storage nor revision zero. Persisting after unavailable
storage requires a successful actual reread and new CAS preparation.

The DOM's `begin_storage_refresh` admits a uniquely authorized read only from Idle,
with no outstanding write/activation. `finish_storage_refresh` checks that exact
authority, nonregression and exact same-revision consistency, then installs the
actual observation without changing intended account/UI or generation. A failed
read installs Unavailable with EXACTLY the prior last-observed head (or None if
none was known), never a fabricated readable result; CAS stays blocked. A Corrupt
observation supplies only the actually readable CURRENT head. Unreadable means an
actual successful observation of current header corruption, NOT a failed read;
its last_known_head must equal exactly the prior retained commitment, or None
only if none was known. Both DOM and CommitMachine install this authorized current
unreadability without changing the selected account/UI pair. Their head() /
durable_head() expose retained historical authority, NOT a claim the current header
is readable; ordinary CAS still requires the Readable variant. Later readable or
corrupt-readable observations and ordinary bootstrap must not regress that history
or change the same revision. Unavailable preserves that same history but does NOT
authorize erasure/reinitialization. Rejected stale/inconsistent results do not
release the refresh guard. After the read settles, explicit begin_clear + binding
+ reserve_privacy_clear can consume the installed Unreadable observation and plan
Reinitialize. Before reservation there is no write permission; from reservation
through actual transaction settlement cancellation/restart are forbidden. Abort
retains the selected pair AND current-unreadability/history; completion or ordered
settlement of the exact new-store marker/tombstone installs the empty pair.
Reconstruction retains both readability and historical authority. No stale-head
CAS, fabricated Corrupt(old head), reload or ad hoc new gate is a clearing path.

For continuous logical gates, first call `CommitMachine::refresh_storage` while
Idle with the same actual read; it preserves selected memory and compacts storage.
Then `SessionGate::restart_worker` freezes its exact recovery input; register that
Bootstrap, not a changed input after registration. This logical order already
worked in review; the repaired seam is DOM installation. Replace the physical pair
only after the DOM read settles, synchronizing both incarnations, and reconstruct
from its unchanged intended pair plus the new storage head. All large read-body
validation/hash work is terminable off-DOM bulk; no write can start meanwhile.
Ordinary bootstrap rejects regressed revisions or changed data at the same revision,
and cannot switch either the intended selection or selected account. Every backup restores through validation;
samples require freshly trusted bundled inventory even when retained in memory.

For explicit session-only activation, `promote_prepared_session` /
`CommitMachine::promote_session_only` commit the intended retained memory anchor
BEFORE dispatching activation. CommitPhase ActivatingSession blocks ordinary work;
matching acknowledgement releases usability. Crash before ack bootstraps that intended
session account. It survives hard cancellation even with a KNOWN OLDER durable
account. A fresh page instead reads actual durable storage. Session-only saves do
not claim persistence; disclose reload loss. `DiscardSession` is an explicit memory
transition, labelled **Discard this session**, preserving known/unread durable data.
`DomGate::discard_session` is its dedicated authority/memory commit: advance reset
exactly once, invalidate old work, drop intended account AND UI anchors, enter
Activating, THEN dispatch DiscardSession. It does not reserve/write storage and
works when IDB is unavailable. Synchronize `SessionGate::discard_session_barrier`
and CommitMachine to that same next reset epoch/generation before the logical
request. Bind its step0 once; the empty-pair acknowledgement advances generation.
Even a crash BEFORE supervisor dispatch reconstructs EmptySession from the DOM
memory commitment, not the former account or known older saved body. There is no
ad hoc new-gate replacement for this transition. It never claims erasure. Durable reset uses a committed revisioned tombstone;
confirmed abort restores usability at the advanced reset epoch, uncertain outcome
blocks, and postcommit restart recovers tombstone without another transaction.

Corrupt payload with a readable current head needs no valid old Backup to erase.
`begin_clear` advances reset authority; `PrivacyClearMachine` reserves either CAS
of that head to next-revision tombstone or explicit store reinitialization when
head is unreadable/revision exhausted. Reinitialization atomically replaces the
single current record and its fresh store marker with revision1 tombstone; it is
NOT ordinary regressed-revision recovery or guessed successful deletion.
Unavailable IDB cannot report clear success. Until transaction complete or an
ordered read of the exact committed target, `erased` is false. Reinitialization
settlement additionally requires its exact new store marker, not an old revision1
tombstone. Confirmed abort remains not-erased. `apply_privacy_clear` and
`install_privacy_clear` are the guarded committed transition for existing machine/
gate; they do not require reconstructing gates or decoding the corrupt old archive.
All actual IDB transaction/CAS/crash semantics remain real-browser obligations.

## Inventory, trust, numeric and decision semantics

Account schema2 requires `materials: Section<MaterialBalance>` on EVERY canonical
snapshot. Schema1 and missing-material snapshots are rejected, not defaulted or
migrated. Logical/compact protocol2 accompanies the new mandatory
`PreparedHead.material_kinds` count and Materials import declarations; old protocol1
is rejected. ImportProfileVersion1 is unchanged: no GOOD importer/conversion is
implemented here. CatalogVersion1 retains the existing generic public-label shape
and meaning, now also accepting `DefinitionKind::Material`; no ontology or content
revision is invented. Consumers must use the same reviewed contract fingerprint.

`Section` preserves Omitted versus supplied-empty. Array presence/GOOD fields do
NOT certify completeness. Completeness is an assertion, not scanner verification.
CurrentLoadout and MoveOnly require every selected physical owner's weapon and
five artifact slots resolved: known occupied slots can resolve partial sections;
an absent slot requires DeclaredComplete. Omitted/Unspecified/Partial absence is
unknown, never empty. Unrestricted known-pool evaluation remains separately scoped.
`ImportDeclaration` explicitly names a supplied section and completeness; complete
assertions carry User statement or Exporter name + original SourceRef. Applying
declarations never converts Omitted to empty, defaults completeness or upgrades it
silently; duplicates/invalid provenance fail. Engine verifies source pointers and
exporter claims against actual bytes, without presenting assertions as audits.

Materials are balances by `DefinitionId`, NOT owned ItemId/CharacterId copies.
`MaterialBalance` contains definition, quantity, optional SourceRef and bounded
Provenance records. Definitions must be unique within the material section (at most
16,384 kinds); each balance permits at most128 provenance entries. Public labels
use the existing PublicDefinition/Catalog bounds. They verify neither costs nor kits.
Materials never enter gear footprints, pins, movement or character availability.
No material causal subjects or cost/budget/investment operation APIs exist yet (H3).
Missing/unknown materials do not disable unrelated non-cost team/build/equipment
validation; future cost claims require explicit evidenced costs and budget inputs.

MaterialQuantity is exactly `{"status":"unknown"}` OR
`{"status":"known","amount":"18446744073709551615"}`. Known amounts use Counter's
canonical nonnegative u64 decimal STRING, including zero and counts above2^53; no
JSON numbers, sign, leading zeroes, fractions, exponents, whitespace, rounding,
clamping or overflow. Unknown has no amount, not zero/infinity. Empty variants use
strict empty-map decoding, including internally tagged unit variants in requests,
recovery, controls, decision metadata and parameter domains. Unexpected payloads
are rejected without changing canonical encodings or maintaining old-shape aliases.
Declared-complete payloads also reject unrecognized verification/provenance fields;
a valid completeness assertion remains an assertion, not an audit. Raw GOOD numeric
lexeme conversion is the forthcoming importer's responsibility.

`ValidatedAccount::material_balance(&DefinitionId)` returns MaterialQuantity.
An explicit record wins, even Unknown under complete coverage. Absent is Known(0)
ONLY under valid DeclaredComplete; Omitted, Unspecified-empty and partial-empty
are Unknown. These sections, explicit Unknown, Known(0), known amounts and declared
complete-empty remain distinct canonical states even when two lookups agree.
No records().is_empty knowledge inference. User/exporter completeness assertions
remain attributable assertions, never audits. SourceRefs require the retained archive,
matching digest and syntactically valid RFC6901 pointers (including ~0/~1 escapes).
Actual pointer targets/GOOD values/exporter claims require importer verification;
shared validation does not parse source bytes as mechanics or manufacture balances.

All canonical IDs globally unique; identical definitions of equipment are distinct
physical copies. Character definitions/forms cannot duplicate ownership. Resolved
identity maps bijectively to owned AvailabilityId; mutually exclusive forms share
one resource. Owner-slot conflicts, current gear, locks and pins use availability.
Unknown availability blocks affected evaluation, including owners of included items;
unrelated excluded unknown resources do not block a supported subset. Unresolved
external owners reserve gear. Game lock remains distinct from movement policy;
protections/pins/exclusions remain binding. Engine checks source-backed type/slot/
progression/mechanics legality beyond shared generic assignment checks.

GOOD generated IDs use exact original-byte SHA-256: `snapshot-<digest>` and
`<digest>/characters/<index>`, `/weapons/<index>`, `/artifacts/<index>`.
Imports replace after confirmation, with no guessed cross-import reconciliation.
`SourceArchive` preserves exact valid UTF-8/hash/format/RFC6901 provenance; backups
preserve canonical IDs/preferences/original archive, never recursively archive
backup wrappers. `ValidatedAccount` trust marker cannot deserialize. Bundled
synthetic trust requires engine hash-checked catalog loading, never a prefix/flag.
Restore of a sample compares fresh trusted inventory allowing only preferences to
differ. Imported accounts cannot acquire synthetic mechanics; unknowns stay unknown.

Fixed is signed i64 millionths, encoded canonical scaled-integer strings. Counter
is canonical u64 decimal string. Decimal/scientific imports use checked i128,
reject inexactness/overflow, never clamp/round to zero. Multiply/divide and objective
dot-product round once, nearest ties-to-even symmetrically. All nine ScoreVector
utilities are [0,1], maximize, never DPS. Explicit positive-weight unavailable
metrics block scoring. Normalization independently revisions units/ranges/direction;
out-of-range raw values fail. One to three explicit encounters correspond to
simultaneous four-person teams; allocation vector is equal-weight encounter mean,
unavailability propagates, fidelity is worst and provenance sorted union.

DecisionSpec explicitly includes mechanics, normalization, assignments, scope,
objective, assumptions, encounters, hard constraints and search policy. Catalog
MechanicsEntry provides complete versioned DecisionPreset specs and typed editable
metadata (flag, bounded quantity/unit, labelled choice, target count, objective
weight), exact mechanics/normalization, and bounded public definition labels.
Every preset assumption/encounter parameter/count/weight needs consistent metadata.
UI gets IDs/labels/domains from contract Catalog/pages, never private dataset or
hardcoded guessed live defaults. Shared validation checks consistency; engine
validates actual dataset support, legality, evidence and content independently.

Hard constraints precede ranking. Footprints exactly match characters,
availability and physical items across teams; no cross-team copy reuse. Rank by
descending score then ascending AllocationId bytewise; IDs derive from canonical
assignments, not timing. Reasons precede effects and all references resolve. Shared `SubjectContext::resolve`
returns the represented account/character/weapon/artifact/allocation/team, declared
assumption value or normalization dimension, not just an identifier-shaped string.
Account subjects match this snapshot; physical character/item subjects exist in it,
including known but unequipped/excluded resources. Team subjects name BOTH returned
allocation and team. Allocation/team subjects cannot name nonreturned candidates;
blocked/constraint explanations may instead name actual account resources, declared
assumptions and dimensions. There is no dangling-ID exemption or self-declared
unbounded entity registry. If a future feature needs nonreturned structures, its
bounded typed representation must be contracted before such references are emitted.
AssumptionRef explicitly distinguishes Global{key} from Encounter{index,key};
matching text in another context is not resolution. Dimension refers to one of the
nine declared normalization dimensions, including an unavailable/uncomputed one;
resolving it does NOT claim a measured value or positive objective weight.

DecisionResult reasons and LocalizedUncertainty subjects/assumptions use that result's
context. An uncertainty metric must be declared as a required metric in the spec or
represented in matching character/item/team/allocation builds (account/dimension/
assumption-wide uncertainty may use any represented build metric). It is not an
invented metric catalog. ComparisonResult's OWN CausalReason<ComparisonSubject>
subjects explicitly name Before or After, each resolved against its full decision
context, even when IDs coincide between worlds. Nested before/after decisions keep
their own local contexts. PresentationSubject preserves this scope when paging.
Typed Character/Item parameters in reasons and result/scope exclusions also resolve
against the same account. Cause IDs must precede effects; allocation/build/constraint
reason IDs resolve locally, and CharacterChange reason IDs resolve in the comparison
graph. Its before/after build lists identify the selected character's builds exactly;
metric-delta keys occur in that character's selected before/after metrics. Donor
availability and participating forms retain their physical-resource checks. Each
nested decision remains bounded at16MiB. The complete comparison, including its own
explanations/metadata, is bounded at128MiB by the strict wire codec. The final enclosing
logical message must independently fit128MiB INCLUDING envelope overhead.
External rule/definition/provenance truth and causal justification remain source-backed
engine duties; shared reference consistency does not verify a combat rule.

Pareto uses exact all-dimension comparisons; unknowns are incomparable. Preserve
resource-distinct alternatives despite locally equal/dominated vectors.
Complete exhausts feasible domain; Infeasible exhausts empty domain; Bounded names
reached limits; Cancelled proves no optimum; unsupported is blocked, not infeasible.
Counters count admitted nodes including later pruning. Each claimed reached limit
uniquely equals its exact maximum, not below/above. Finished domain exactly at
budget may still be Complete without a cutoff claim. FrontierPresentation is
separate (Exhaustive / positive Truncated / AmongReturned); returned dominance
needs witnesses. Validators check consistency, not engine exhaustive-search proofs.

CurrentLoadout comparison fixes parties/encounters/current gear. MoveOnly changes
only the specified move and optional donor fallback: legal unequipped item OR the
recipient's EXACT displaced item of the moved slot/type. Donor must participate
by physical availability. No unrelated occupied third-party gear, same-availability
pseudo-transfer, pin/lock/protection violation or unrelated reoptimization. Exact
weapon/artifact swap controls are generic constrained interventions, not verified
live-game rules; engine still checks source-backed legality. No fallback leaves
donor slot empty; recipient displaced gear is otherwise unassigned.
UnrestrictedOptimum instead compares Unrestricted against ReservedMove under
identical resources/policies/objective/assumptions. Recipient participates/uses item;
other builds may reoptimize and must be reported. Restriction cannot improve a
Complete unrestricted optimum even if after search is bounded. Both sides use SAME
original trusted account; imported owner is preserved separately from participating
donor forms. All selected recipient/donor/other build references are explicit.
Delta is after minus before; ExactDifference requires both searches Complete,
BestFoundDifference is not proven improvement, absent feasible sides NotComparable.
Sensitivity changes only explicit assumptions/encounters, at most16 unique scenarios;
validate full reruns/rank summaries/deltas without invented probabilistic confidence.
Marginal roster/investment decisions are not implemented by equipment comparison.

Semantic digest sorts records/pins/substats (materials by definition) and removes
per-record source refs/archive/provenance, retaining material coverage, explicit
known/unknown quantities and exact counts along with IDs/availability/progression/
ownership/policy. Completeness assertions retain their existing attributed encoding.
Material changes invalidate snapshot/Backup commitments, sample inventory trust,
semantic result/cache keys and account-bound recovery/UI commitments. Backup,
prepared activation and durable/session recovery carry the whole mandatory section.
Snapshot digest hashes compact AccountSnapshot; Backup digest includes sample
binding. Spec hash includes all fields. Result fingerprint is compact tuple
`["aaru-result-v1", account_semantics, spec_digest, engine_version, engine_revision]`.
Exact arithmetic/fingerprints are reproducibility, not verified game truth.

## Bounds and scheduling

| Surface | Bound |
| --- | --- |
| Original source / escaped wire-backup | 16 MiB / 128 MiB |
| Characters / weapons / artifacts / material kinds | 256 / 4,096 / 16,384 / 16,384 |
| JSON depth / container entries / ordinary string bytes | 64 / 65,536 / 4,096 |
| Numeric lexeme bytes / absolute exponent | 128 / 128 |
| Control/page encoded bytes / page rows | 16 KiB / 24 |
| Candidate / replacement / unacknowledged edit / unacknowledged page | 1 each |
| Distinct retained private sources / extra caches | 2 across ALL retained bodies / 32 MiB |
| Returned allocations / per-decision bytes | 128 or lower max_results / 16 MiB |
| Reasons / diagnostic-uncertainty-provenance entries / scenarios | 1,024 / 128 / 16 |
| Per-worker linear-memory profile / owned opaque Blob ledger | 512 MiB / 256 MiB |
| Cooperative search slice | <=256 admitted primitives and <=4 ms, real task yield |
| Visible cancellation acceptance | 50 ms |

Large immutable accounts, archives, backups and logical specs use shared Arc
ownership inside each worker; gate/registry/commit retain compact commitments or
shared references, not avoidable full-source clones. Older saved A + session-only
active B + candidate C must NOT keep three sources: preserve A's compact head and
release its body/Blob when A is not selected. This applies across recovery inputs,
registries, supervisor staging and DOM Blob references, not only CommitMachine's
active/candidate fields. Drop abandoned candidates before unrelated work; do not
refuse routine later imports merely because saved and active accounts differ.
Arc does not share WASM memory
between siblings. Adapters release staging/candidates/results/URLs deliberately;
each nested DecisionResult is bounded at16MiB; comparison/sensitivity bodies,
including their own metadata, obey the128MiB wire bound, not an additional16MiB
whole-compound cap. Their contextual validators enforce that aggregate bound.
The FINAL logical WorkerEvent/request including envelope overhead must also pass
the128MiB wire codec; a near-limit body does not reserve room for its envelope.
BlobLedger charges
each distinct application-owned immutable payload's full byte length, including
source File retention; `share` counts known aliases and release uncharges only the
last alias. Unknown copies need separate entries; object URLs retain a ledger
reference. Charge before retaining/creating, reserve capacity for output, evict
extra caches and release abandoned payloads. Neither ledger nor Arc can promise
browser GC, Blob implementation deduplication or total RSS; actual worker peaks,
WASM build/runtime caps, native-handle lifetimes and delayed reclamation need proof.
Do not shrink the allowed 16 MiB input to hide a failure of this frozen profile.

256-unit/4 ms admission applies to cooperative search primitives, not whole-input
codecs/hashing/validation. Those are explicitly isolated TERMINABLE bulk phases.
They remain off DOM; a busy worker need not process Cancel because DOM owns physical
termination. This preserves the 50 ms visible acceptance requirement rather than
claiming large synchronous helpers became short. SliceBudget only tests admission,
not yielding or time. Actual owned optimized-WASM probes in Chrome156 observed
16 MiB fingerprints56–66 ms, encode23–28 ms, decode77–92 ms (synthetic valid GOOD
padded with whitespace). That observed failure justifies the isolation change;
it does not prove replacement cancellation, memory, storage or platform behavior.

## Executable logical examples

Partial backup, NOT an evaluable current-loadout account:

```json
{"account":{"schema_version":2,"id":"empty-account","characters":{"coverage":"omitted"},"weapons":{"coverage":"supplied","completeness":"declared_partial","records":[]},"artifacts":{"coverage":"omitted"},"materials":{"coverage":"omitted"},"preferences":{"game_lock_policy":"protect_from_reassignment","protected_items":[],"pinned":[],"excluded_characters":[],"excluded_items":[]},"source_archive":null,"provenance":[]},"sample":null}
```

```json
{"protocol_version":2,"context":{"epochs":{"session":"example-session","worker":"0","reset":"0"},"account_semantics":null,"engine":{"version":"0.1.0","revision":"example-engine"}},"fingerprint":"19c72a551bea153aaf0484f564067565dbe2f6346f29b20cafbbd1914a14a792","request_id":"catalog-1","account_generation":"0","payload":{"type":"get_catalog"}}
```

```json
{"protocol_version":2,"context":{"epochs":{"session":"example-session","worker":"0","reset":"0"},"account_semantics":null,"engine":{"version":"0.1.0","revision":"example-engine"}},"fingerprint":"4ad2f15d8d8e53bf17d3eea0d36ba55c40b48f5b8fd6f03375ba1797779f491e","request_id":"sample-1","account_generation":"0","payload":{"type":"prepare_import","input":{"type":"sample","id":"syn-welcome"}}}
```

## Historical G1 native evidence and remaining gates

The executions below describe the accepted pre-extension snapshot, not schema2
acceptance. Current H0 evidence is separate: `tmp/h0-material-contract-evidence.md`,
`tmp/h0-shape-repair-evidence.md` and `tmp/h0-shape-repair-review-evidence.md`.
Repaired87 contract/20 policy/4 Git tests, strict native/contract-WASM and unchanged
18+2 original probes pass; independent repair review also executed4 own controls.
The historical16pass/2fail review remains unchanged. Parent inspected the complete
staged16 files/new tests, all-files/hooks/security passed, exact50119ab hosted
40609987311 passed Linux/Windows/security/Quality. `tmp/H0_MATERIAL_ACCEPTANCE.md`
closes only the material native subgate, not public-interface/importer/browser/release
acceptance. Further shared contracts must be frozen before consumer source leases.

Executed on Linux2026-10-27, Rust/Cargo1.98.1, base HEAD72f2048 plus unstaged
integration. Current source/test/dependency manifest, excluding this document:
`bf5cb79a84ca4759369436e8e9c669a9241e73ea9eed24970b98ea3e5766cab9`.

```text
sha256sum Cargo.toml Cargo.lock crates/aaru-contract/Cargo.toml crates/aaru-contract/src/*.rs crates/aaru-contract/tests/*.rs crates/aaru-contract/tests/cases/*.rs | sha256sum
```

Full locked workspace passed73 contract/20 policy/4 real temporary-Git tests,
none ignored/filtered; formatting, strict native/all-target and contract wasm32
Clippy/check passed. The preceding stable independent review passed15 owned probes
and supported the nine lifecycle/reference repairs, but accepted an incorrectly
reduced whole-comparison16MiB cap. Parent corrected its own review instruction:
original16MiB is per decision, compound total wire128MiB. That earlier comparison
byte-edge sensor was the WRONG policy oracle, not proof of unchanged limits.
Historical evidence remains intact in `tmp/g1-final-seam-{repairs,review}-evidence.md`.
Current correction evidence is `tmp/g1-budget-scope-evidence.md`; no pins changed.

| Native proposition | Oracle and scope |
| --- | --- |
| Current unreadability permits guarded clear without forgetting history | Saved A7/session B/UI, exact fresh store marker, abort/commit/uncertain reconstruction; unavailable, stale and regressed negatives |
| Explanations and mandatory result admission resolve actual context | Owned unused resources, allocation/team scope, global/encounter assumptions, before/after comparison, localized metrics, typed parameters, topological causes; invalid result leaves request pending |
| Account/UI and authority transitions compose | Exact staged target/ack, explicit replacement policy, DOM-first discard/commit, bind-once replay, ordered refresh, revisionMAX memory preparation; new-machine reconstruction |
| Distinct retained sources stay two | Arc/Weak controls including saved bodies/caller-held references; compact unselected durable body to truthful head, never activate unread head-only data |
| Byte-budget scopes remain original | Decision16MiB/+1 standalone and nested; valid comparison above16MiB; comparison and sensitivity bodies exactly128MiB/+1 including own metadata. Three full-suite isolated faults detected reduced comparison cap, omitted sensitivity aggregate check and wire+1 allowance; exact restoration manifests |
| Other bounds/semantics remain exercised | Reasons1024, uncertainties/allocations128 or lower max_results, opaque attachments/compact controls, exact cutoffs, availability/forms/donors/fallback and setup/preset controls |

The independent original failing harnesses were not rewritten. Parent's new owned
copy with the original compound-policy witness passes74 tests after correction;
fixture reuse is not an independent game oracle. Narrow independent budget review
passed21 owned controls and73/20/4 twice, including final event overhead/admission;
it found one stale contradictory16MiB policy sentence, now corrected above. No
budget code defect remained in those probes. Faults challenge selected sensors,
not universal safety or a mutation score. Complete staged/new-file/all-files/security
checks and normal hooks passed; parent repeated all21 probes. **Native G1 accepted**
at1b664913a3d25f86e97b18b1069a442276e9b760, hosted40582912311attempt1 all four required
jobs successful. Parent decision: tmp/G1_ACCEPTANCE.md. This is NOT production or
release acceptance. P1/H0 shared-schema/build/lease prerequisites still precede
engine/UI source launch; no automatic orchestrator resume.

These are native codec/FSM/reference and Arc/Weak observations, NOT production
Worker handlers, actual confirmations/settings migration, IDB transactions, Blob
lifetimes, painted cancellation or memory/RSS proof. Per-decision16MiB, source16MiB,
wire128MiB/control16KiB/source2/cache32MiB/worker512MiB/Blob256MiB and50ms visible/
256-unit4ms search profiles remain. Actual serialized envelope overhead still counts.

Mandatory optimized production gates on Windows/Linux include both-sibling
termination during large/uncooperative work, independently observed50ms visible
cancellation, stale messages, exact bytes, real attachment/URL retention, IDB abort/
complete/CAS/quota/denial/current-header classification/guarded fresh-store clear,
uncertain outcome/crash/reload/ack, Web Lock succession, session B over saved A and
accepted UI settings. Natural repeated max-size cycles and physical memory,
dedicated-origin/CSP/subpath/privacy, extensive realistic corpus, source-backed
mechanics, all real engine/search/roster/investment/UI operations, accessibility,
security and release artifacts remain open in RELEASE_GATES.md.
