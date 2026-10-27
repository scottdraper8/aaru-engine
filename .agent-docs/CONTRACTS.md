# Shared schema 1 — engine/browser handoff

`crates/aaru-contract` defines the shared API; this document defines its semantics.
G1 remains subject to parent acceptance. This is not a product/release claim.
All first-party application, UI, tests and repository tooling are Rust. Static
HTML/CSS and generated initialization-only WASM bindings/loaders are permitted.
Follow `PRODUCT_LANGUAGE.md`: Aaru/Sumeru identity, literal novice-readable
operations, original accessible styling, no spoilers or invented mechanics.

## Ownership and deployment

| Owner | Responsibility |
| --- | --- |
| Contract coordinator | `crates/aaru-contract/**`, shared schema and this document; root dependency/lock integration |
| Engine thread | `aaru-engine`, `aaru-wasm`, Rust worker runtime/scheduler, `fixtures/**`, `data/**` |
| UI thread | `aaru-web`, browser worker construction/lifecycle, `web/**`, `tools/browser-tests/**`, player help |
| Parent coordinator | Build/server/bootstrap tooling, CI/security/policy, acceptance/release coverage matrix |

Both builders may own crate-local manifests and regenerate worktree lockfiles;
parent resolves integration. Preserve exact pins and workspace `crates/*` glob.
Dependencies: web -> contract only; engine -> contract; wasm -> engine + contract.
No guessed interfaces, temporary workers, fabricated engine responses, UI scoring
or duplicate mechanics ontology. Both threads start only after parent accepts G1.
Catalog/manifest **schemas** remain contract-owned; after acceptance, their
**content**, hashes, attribution and fixture oracles belong to the engine thread.

All public assets resolve against one explicitly configured, same-origin base URL
ending in `/`, including project-subpath hosting (for example `/aaru-engine/`).
Never resolve assets against a route's current location or use root-absolute paths.
No imported data supplies asset URLs. Production uses a dedicated trustworthy
origin. IndexedDB namespacing prevents collisions, NOT access by other apps/scripts
on that origin; path prefixes do not isolate storage. Local imports/preferences,
source bytes and downloads must not leave the browser or enter logs/telemetry.
No credentials, remote inventory persistence or account uploads.

Parent's Rust server binds loopback only and serves a generated allowlisted public
asset root, **never the repository root**. Explicit paths, no directory listings,
no symlink escapes, traversal/encoded traversal/backslash rejection, correct WASM
MIME and non-root base-path/CSP tests are later real-server gates. CSP must permit
only same-origin scripts/workers and WASM compilation, with no arbitrary inline
application code, framing, objects or external assets.

| Asset | Base-relative path |
| --- | --- |
| Catalog | `fixtures/catalog.json` |
| Mechanics manifest | `data/mechanics/manifest.json` |
| Worker module loader | `pkg/aaru-wasm/worker.js` |
| Worker generated bindings | `pkg/aaru-wasm/aaru_wasm.js` |
| Main module loader | `pkg/aaru-web/main.js` |

Parent build tooling generates minimal loaders that default-initialize colocated
`aaru_wasm.js` / `aaru_web.js`, which load colocated `_bg.wasm` files. Loaders do
not validate/route messages or implement state. UI constructs an actual module
Worker at `WORKER_MODULE_PATH`, not uninitialized bindings. WASM adapter owns
`#[wasm_bindgen(start)] pub fn start_worker()` and Rust message listeners; UI owns
`#[wasm_bindgen(start)] pub fn start_web()`. Both produce cdylib/rlib. These are
handoff requirements, not existing product implementations.

Fixed sample IDs: `syn-welcome`, `syn-scarcity`, `syn-stress`, and
`good-unsupported-preview`. Account paths are respectively
`fixtures/canonical/<id>.json` for synthetic accounts and
`fixtures/good/good-unsupported-preview.json` for the preview. Synthetic mechanics
paths are `data/mechanics/synthetic/<id>.json`; preview has none. Catalog uses
`Catalog`/`SampleEntry`/`MechanicsEntry`; manifest is `Vec<MechanicsEntry>`.
Synthetic definitions use `syn:` prefixes, but prefixes never establish trust.

## Transport, inventory and trust

UTF-8 JSON text only, never JS objects/Number conversion for account/protocol
values. `wire::decode` rejects duplicate properties (including escaped-equivalent
keys), BOMs, trailing data, excessive depth/strings/containers/numbers before
typed decode. Unknown fields are rejected. `wire::decode_request` also validates
the full operation fingerprint and request budgets. Structural decoding is NOT
contextual validation: worker must validate accounts, specs and results before
work/emission. All four independent version newtypes currently accept integer 1
only; no migrations or compatibility aliases. GOOD dialect is a separate upstream
version; profile 1 does not certify any unverified dialect semantics.

`Section` distinguishes omitted from supplied-empty. Completeness is exporter
assertion, not a scanner audit. Optional progression/lock/stat fields mean unknown,
not zero. u16 representation/positivity checks do not invent live progression caps.
Engine validates source-backed progression, identity catalogs, compatibility and
mechanics. Unsupported quantities retain inert lexemes; unknowns are not guessed.

Character ID selects one definition/form. `CharacterAvailability::Resolved` names
both canonical real identity and the one `AvailabilityId` for that owned resource.
Definitions cannot repeat under distinct IDs; one real identity maps bijectively
to one availability. Different supported forms may share that availability, never
use it simultaneously. Engine must corroborate identity/form mappings against its
supported catalog, not trust imported labels. Unknown relationships remain
`Unknown`: affected whole-inventory or subset evaluation is blocked. The affected
closure includes character owners of included weapons/artifacts, even when those
owners are not selected. Shared movement validation also requires resolved owner
and recipient availability before use. Excluding an owner cannot free its equipment.
An excluded unknown character and its excluded equipment do not block unrelated
supported subset evaluation. Equipment owner-slot conflicts, current loadouts, game-lock checks and pins also
use availability, so selecting a form neither manufactures slots nor transfers
gear between real owners. Comparison rejects a purported transfer between mutually
exclusive forms; changing form is not an equipment move. `ResourceFootprint` must exactly match selected
character IDs, availability IDs and physical item IDs across all simultaneous teams.

All canonical character/item IDs are globally unique; item definitions are not
copies. GOOD without stable upstream IDs uses original-byte SHA-256 snapshot ID
`snapshot-<digest>` and `<digest>/characters/<index>`, `/weapons/<index>`,
`/artifacts/<index>`. Identical weapons/artifacts remain distinct copies; repeated
characters do not become additional ownership. Imports replace after confirmation;
no guessed cross-import reconciliation. Preserve canonical IDs on restore.

Unresolved external owners reserve equipment and occupy slots; omitted rosters do
not create owners. Game lock is preserved separately from Aaru movement policy.
Default policy protects known locked items; opt-in reassignment changes policy,
not game data. Explicit protections, exclusions and pins stay binding. Pins reserve
an item to its owner if used; they do not force participation. Engine additionally
validates weapon legality, artifact rules and supported mechanical predicates.

`SourceArchive` preserves exact original valid UTF-8 bytes, hash, format and source
references. Source pointers are RFC 6901 (empty = document); engine verifies escaping,
existence and record provenance. Source export returns identical bytes, never
regenerated GOOD. Canonical export is `Backup`, not GOOD. Restore preserves the
original archive, never recursively archives backup wrappers. Backup formatting
need not survive; archive bytes, IDs, preferences and provenance must. No source
archive means source export is explicitly unavailable.

`ValidatedAccount` cannot deserialize its private trust marker. External raw
accounts always use `imported`. Only the engine's bundled, hash-checked catalog
loader uses `bundled`. GOOD cannot acquire synthetic trust. Synthetic mechanics
require the exact trusted synthetic SampleId; synthetic accounts cannot use live
mechanics. Backup sample metadata is NOT a trust assertion: `Backup::restore`
requires a freshly trusted bundled reference of that SampleId and exact inventory
hash, allowing only preferences to differ. Removing metadata yields an imported
account, never synthetic permission. Modified sample inventory is rejected rather
than silently relabelled. `with_preferences` validates and preserves existing trust.

## Numeric and scoring semantics

`Fixed` is signed i64 millionths. Wire values are canonical scaled-integer strings:
`"39000"` means 0.039 units. No leading plus/zeroes or negative zero. Counter and
epoch values are canonical u64 decimal strings; small counts/versions are numbers.
Decimal/scientific imports are exact using checked i128 intermediates; inexact,
overflow and malformed inputs are errors, not rounded/clamped zeros.
`from_percentage_points("3.9")` is 0.039, only for verified percentage-point fields.
Multiply/divide round once, nearest integer ties-to-even, symmetric across signs.
Division by zero and narrowing overflow fail. Objective dot product accumulates
all weighted products and rounds once at the end. Nonnegative weights need at
least one positive term; weights are not automatically normalized. Positive-weight
unavailable dimensions block scoring; zero-weight unavailable terms do not.

Raw build metrics use engine-defined semantic keys/units under the mechanics
revision. `Normalization` independently revisions all nine dimension ranges, units
and higher/lower direction; engine verifies these against the dataset, not invented
live math. `MetricRange::normalize` maps the declared low/high range affinely into
[0,1], reversing lower-is-better metrics, with one final half-even division.
Out-of-range raw values fail, never clamp. `ScoreVector` values are dimensionless
normalized utilities in [0,1], all maximize, never DPS. Fidelity/provenance remain
explicit. Global/character metric differences may be negative and are not utility
vectors. Resource efficiency is explicit modeled resource cost, not a donor fudge.

Each team contains its per-encounter ScoreVector. One to three explicit encounters
represent simultaneous four-character teams. Allocation vector is their equal-weight
arithmetic mean per dimension, rounded once; any unavailable term propagates.
Aggregation uses worst fidelity and sorted union of provenance IDs. Objective is
applied to the aggregate, not independently rounded scores summed afterwards.

Pareto compares every dimension exactly; any unavailable dimension makes vectors
incomparable, including identical unknown vectors. Never discard resource-distinct
builds solely for local vector equality/dominance. `SearchStatus` describes search:
Complete exhausts the declared feasible domain; Infeasible exhausts an empty one;
Bounded has declared reached limits; Cancelled proves no optimum; unsupported is
blocked, not infeasible. Build/team/allocation counters count admitted nodes, including
nodes subsequently pruned. Check the budget before admitting the next node: every
claimed reached limit must occur exactly at its corresponding maximum, never below
or above it, with no duplicate claims. Exhaustion exactly at a budget can still be
Complete if the domain is finished without cutting off another node; it then claims
no reached limit. Presentation limit is not a search cutoff. `FrontierPresentation`
separately says Exhaustive, Truncated with positive omitted frontier count, or
AmongReturned. Global frontier claims require complete/infeasible search and
comparable classified returned vectors. Unknown vectors use AmongReturned/Unknown.
Dominated labels need returned witnesses; Frontier cannot have a returned dominator.
Validation checks consistency, not the truth of exhaustive-search/pruning proofs.

## Decisions and comparisons

`DecisionSpec` makes assignment domain, mechanics, normalization, scope, objective,
encounters, assumptions, hard constraints and search policy explicit. Engine
rejects unsupported keys/rules/weighted dimensions, missing mechanics/progression
and invalid combinations. No default combat assumptions. Shared validation checks
exact footprints, slots, exclusions, pins, allowed artifact sets/main stats, metric
minima, satisfied required-rule outcomes and objective/aggregate consistency.
Engine still proves mechanics legality and safe pruning. Missing assigned weapon
or artifacts means actually unassigned, not unknown equipped inventory.

Rank descending presentation score, then ascending AllocationId bytewise. Engine
IDs derive deterministically from canonical assignments, not timing. Sort keys
bytewise, preserve encounter order. Causes precede effects; references must resolve.
UI renders factual reason codes, localized uncertainty and assumptions without
post-hoc invented explanations. Exact arithmetic is not verified game truth.

`ComparisonSpec` has two distinct modes, never a silently changed world:

- **CurrentLoadout**: one explicit validated party allocation with currently equipped
  gear, including recipient. Intervention is `MoveOnly`: same parties/encounters and
  unrelated gear, moved item on recipient, recipient's displaced gear unassigned,
  donor loses item. Optional donor fallback must be a different unequipped legal,
  permitted item of the same type/slot, used by a participating distinct donor.
  No fallback means unassigned donor slot. No unrelated reoptimization is attributed
  to this move. Fixed allocation and fallback are included in the after spec hash.
- **UnrestrictedOptimum**: baseline Unrestricted; after ReservedMove under identical
  account resources, policies, mechanics, normalization, objective and assumptions.
  Recipient must participate and use the item. Donor/other builds may reoptimize and
  their changes must be reported. This measures restriction/opportunity cost, not
  ownership creating resources. A complete unrestricted optimum cannot improve by
  restricting assignments, even if the restricted search is bounded.

Both decisions validate against the SAME original trusted account; original source,
ownership, locks and resources are unchanged. Exact before/after assignment footprints
identify hypothetical use; no duplicate full ownership-map scaffolding. All selected
characters (including unchanged controls), donor and recipient have explicit build
references. `original_owner` preserves the imported owner ID. `DonorChange` binds its
physical availability and all forms participating in either selected before/after
allocation, each with its own build references. An alternate participating form must
not be relegated to unrelated changes while loss is attributed to an absent form.
If no donor form participates on either side, the imported owner has explicit empty
build references instead. Fallback legality uses the participating before-form's
availability, not exact imported form-ID membership. Global delta is after minus before best-returned score. ExactDifference
requires both searches Complete; BestFoundDifference is not proven improvement;
missing feasible sides require NotComparable and unavailable delta. Character raw
metric deltas and causal/mechanical completeness remain engine proof obligations.

Sensitivity varies only explicit assumptions/encounters, preserving all other fields,
including normalization and assignments. At most 16 unique scenarios. Each full
rerun, rank-change summary and subtraction is validated; no probabilistic confidence
is invented. Marginal roster/investment operations are NOT implemented by equipment
comparison; parent must track them explicitly in the release coverage matrix.

## Authority, persistence and recovery

Every envelope carries protocol version, request ID, account generation, RequestContext
(session ID, worker incarnation, reset epoch, active account semantic digest, engine
version/revision), operation fingerprint and payload. Fingerprint is SHA-256 of the
compact Rust JSON tuple `["aaru-operation-v1", context, generation, request]`.
Parent build metadata supplies identical engine version/revision to UI and worker;
neither consumer guesses the other package's identity. Responses echo the operation
fingerprint/context; transitions echo old context but carry new generation exactly
old+1. UI updates active account semantics only on a matching
transition with the expected snapshot/hash. All results additionally bind their full
spec and account/engine reproducibility fingerprint. Hashes are integrity/context
identifiers, not security signatures or evidence that a database transaction occurred.

Exactly ONE authoritative foreground operation; no independent concurrent UI regions.
New ordinary operations locally supersede old work. Cancel explicitly targets the
current cancellable foreground request: import/preference preparation, export,
catalog, evaluation, comparison or sensitivity. Unknown/noncurrent targets fail
without displacing current work. Late preparation success cannot restore preview
authority. Saving, activation, reset, recovery and session discard cannot be cancelled
or ordinarily superseded. Worker retains at most one pending replacement, disposing
superseded work, not accumulating requests. IDs/tokens cannot
repeat within an incarnation (4,096 maximum); overflow/exhaustion is explicit.
SessionGate rejects reversed completion, changed objectives/targets/specs, cancelled
work, mismatched families/generations/fingerprints and duplicate terminal events.
Worker independently enforces authority and cancels obsolete work. No stale preview
or save can become authoritative just because its account generation still matches.

Worker restart increments worker epoch and invalidates previews/work. Call
`restart_worker` with the matching CommitMachine only after storage has settled;
Writing blocks restart. The gate then accepts only Recover with the exact expected
`RecoveryState`, not a replacement Reset or a caller-invented account. A new worker
accepts that recovery context and retained UI generation as its incarnation bootstrap,
validates/restores its payload, and acknowledges generation+1. Storage revision is
NOT account generation; old-worker messages never bootstrap a new incarnation.
New sessions use fresh session IDs. Generation/epoch increments cannot wrap.

`RecoveryState::Durable` represents a successful durable read, including a revisioned
tombstone or initial empty record. When a durable record is known, restart restores
that record, not a session-only override; restoring the retained override is a later
explicit prepare/restore operation. `CommitMachine::session_only` instead represents
initially unavailable/unread IDB with no asserted durable record or revision.
Preparation and explicit SessionOnly activation remain usable. Restart carries the
retained backup as `RecoveryState::SessionOnly`, or None for an empty session; this
is not a successful durable read. A recovered backup acknowledges Activated; an
empty session acknowledges SessionDiscarded, never a durable ResetAcknowledged.
Fresh trust validation still applies to recovered sample backups.

`DiscardSession` is a separate memory-only action when storage was unavailable:
advance the reset epoch through `discard_session_barrier`, drop retained private
memory/candidates, then acknowledge SessionDiscarded at generation+1. Label it
**Discard this session**, never **Clear local data**. Previously saved data might
still exist in unread storage; do not claim it was removed. Durable clear requires
an actual committed tombstone. No gate reconstruction is a recovery/reset operation.

Persistence and worker memory are NOT one transaction. PreparedRegistry holds one
validated inert candidate and complete `ImportPreview.backup`; invalid preparation
preserves active state. Preparation binds source, account, preferences and trust.
PreparePreferences uses `with_preferences` then the same preview/commit flow;
PrepareImport/Restore and canonical Aaru files use `Backup::restore`, not raw flags.

Primary tab must hold exclusive Web Lock `aaru-primary-v1` for its lifetime; secondary
tabs and browsers without locks may show public help only, with no private account
read/import/evaluate/export/write/reset. All private operations, including session-only,
require primary ownership. Loss of ownership stops work. Browser proof remains open.

Primary UI/worker adapters follow these ordered boundaries:

1. Validate a prepared preview and get explicit replacement/preference confirmation.
   `SessionGate::begin_prepared_save` binds CommitMachine to that exact authoritative
   preview/epochs and reserves the foreground. No new import, objective, reset,
   cancellation or other ordinary operation can start while saving/activating.
   Activation is blocked while Writing, including SessionOnly or a prematurely
   fabricated receipt. After gate/storage_committed only the exact matching
   persisted activation is admitted, or a settled restart followed by recovery.
2. In ONE IndexedDB transaction, compare the observed revision, replace the entire
   DurableRecord (backup includes archive/preferences), and increment revision.
   Transaction **complete** is the durable commit point, not request success.
3. On confirmed abort/failure before commit, call gate/storage_failed; old durable
   and active account remain usable. Offer retry, cancel, or explicit SessionOnly
   activation. SessionOnly changes memory, never claims the durable save succeeded.
   Reload restores old durable state; disclose this. Clear-local-data is never a
   session-only operation: it needs a committed reset tombstone.
4. Only after completion build the exact CommitReceipt and send ActivatePrepared /
   Persisted. Registry verifies backup/record digest, revision, epochs and token,
   then installs account and increments generation. Repeating the most recent same
   token/context/disposition at the resulting worker generation is idempotent; stale
   or changed acknowledgements fail. On lost acknowledgement, resend the identical
   original envelope without replacing the registered transition. Worker checks this
   exact replay before its ordinary stale-generation guard; registry `current` is
   worker-owned current generation, not a caller-supplied claim. CommitMachine
   activates committed memory separately.
5. Interruption AFTER commit recovers the new durable account, even if worker
   activation never acknowledged. While Writing, await transaction completion/abort
   and reread IDB before recovery: a lost callback cannot tell memory what committed.
   Settle storage before resetting/restarting/advancing commit epochs; never let an
   old asynchronous transaction run across a new writable session. With a lost
   callback, gate/settle_storage requires a successful read after the transaction
   settles, exactly matching either the prior record (abort) or candidate (commit).
   Another revision/payload is not proof of either outcome. Recovery while Writing
   is rejected. Invalidate old candidates through CommitMachine as well as SessionGate.
   Recovery rejects regressed revisions/epochs and different data at the same revision.
6. Reset uses an explicit lifecycle:
   - `reset_barrier` advances reset epoch immediately and invalidates preview/work
     authority. `begin_reset_save` invalidates the storage candidate at that epoch
     and begins one revisioned tombstone write. Armed/Writing block foreground work.
   - Before any write starts, `abort_reset_before_save` releases Armed, including
     unavailable-IDB failure. Confirmed transaction abort uses gate/storage_failed;
     settled reread of the old record uses gate/settle_storage. Both release Writing,
     preserve old durable/active state and restore evaluation/export at the advanced
     reset epoch. Old previews never regain authority; no epoch rollback.
   - Uncertain outcome stays Writing: no restart, reset retry, recovery, cancellation
     or ordinary work until transaction settlement. A failed read is not settlement.
   - gate/storage_committed or settled reread of the candidate records the tombstone.
     `Request::Reset` then requires the exact committed record's matching receipt.
     Clear worker memory/private caches/download URLs and acknowledge generation+1.
   - If acknowledgement is lost, restart with the settled CommitMachine and Recover
     that exact tombstone. Old-account and SessionOnly recovery are rejected; no
     second clear transaction is required. ResetAcknowledged clears the barrier.
     A committed tombstone must never resurrect the prior account.

`CommitMachine` is a pure state-machine boundary. Its success does not establish
IndexedDB/CAS/Web Lock/worker correctness. Real adapters must enforce the sequence,
including never inventing receipts, settle writes before restart, and prohibit user
actions during durable transitions. Activation failure after commit requires retry
or recovery of the committed state, not silently restoring the prior account.

## Bounded work and reproducibility

| Surface | Bound |
| --- | --- |
| Original GOOD/source bytes | 16 MiB |
| Escaped backup/wire transport | 128 MiB |
| Characters / weapons / artifacts | 256 / 4,096 / 16,384 |
| JSON depth / container entries / ordinary string bytes | 64 / 65,536 / 4,096 |
| Numeric lexeme bytes / absolute exponent | 128 / 128 |
| Prepared candidates / queued replacement | 1 / 1 |
| Retained distinct private sources / extra caches | 2 (active + candidate) / 32 MiB |
| Returned allocations / decision bytes | 128 (or lower max_results) / 16 MiB |
| Reasons / issues, uncertainty, provenance entries | 1,024 / 128 each |
| Scenarios | 16 |
| Primitive work per slice / elapsed slice budget | 256 (or lower requested) / 4 ms |
| Cancellation acceptance target | 50 ms |

Only `text`/`utf8` allow wire-length strings; source/input/download contextual bounds
still apply. No nested archive histories. Evict caches instead of exceeding budgets;
cache entries include full semantic fingerprints. Import/serialization must also be
bounded and cancellation-aware, not just combinatorial enumeration. A primitive
cannot be an entire unbounded build/team expansion: split scans into resumable steps.
Use SliceBudget before each primitive; yield through a real worker task (not a
self-replenishing microtask), check cancellation before work and emission. A primitive
must finish within 4 ms on declared supported reference workloads. Measure optimized
production workers, enforce 50 ms cancel acceptance and no late authoritative result.
These are acceptance requirements, NOT native timing evidence. SliceBudget only
checks admission; it cannot split a synchronous helper. At the allowed 16 MiB source
size, the actual current optimized WASM operation fingerprint took 56.1–66.5 ms and
decode_request 77.2–91.9 ms in three sandboxed Chrome 156.0.8078.47 module-worker
trials. Native release decoding took 53.497–55.766 ms. These separately measured
synthetic-whitespace probes used the owned codec, not production UI/storage; exact
provenance is in temporary `g1-budget-evidence.md`. Current hashing, codec, cloning
and validation must not be assumed to be 4 ms primitives or safe DOM-thread work.
The separate architecture review must freeze a realizable bulk/control transport and
cancellation seam before consumer handoff. This repair does not redesign that seam,
lower limits, weaken the 50 ms target, or establish production responsiveness.

`account_semantic_digest` sorts records/pins/substats, removes source references,
archive and provenance, retains IDs, availability, coverage, progression, ownership
and policy. `snapshot_digest` hashes exact compact AccountSnapshot serialization;
Backup::digest also includes sample binding. Spec hash includes all spec fields.
Result hash is compact tuple `["aaru-result-v1", account_semantics, spec_digest,
engine_version, engine_revision]`. BTreeMaps/Sets are ordered; vector order matters.
No timestamps or timing in fingerprints.

## Executable wire examples

A partial backup, NOT an evaluable account:

```json
{"account":{"schema_version":1,"id":"empty-account","characters":{"coverage":"omitted"},"weapons":{"coverage":"supplied","completeness":"declared_partial","records":[]},"artifacts":{"coverage":"omitted"},"preferences":{"game_lock_policy":"protect_from_reassignment","protected_items":[],"pinned":[],"excluded_characters":[],"excluded_items":[]},"source_archive":null,"provenance":[]},"sample":null}
```

Catalog and bundled-sample requests, with independently computed tuple hashes.
They do not claim fixture content or a worker already exists:

```json
{"protocol_version":1,"context":{"epochs":{"session":"example-session","worker":"0","reset":"0"},"account_semantics":null,"engine":{"version":"0.1.0","revision":"example-engine"}},"fingerprint":"19c72a551bea153aaf0484f564067565dbe2f6346f29b20cafbbd1914a14a792","request_id":"catalog-1","account_generation":"0","payload":{"type":"get_catalog"}}
```

```json
{"protocol_version":1,"context":{"epochs":{"session":"example-session","worker":"0","reset":"0"},"account_semantics":null,"engine":{"version":"0.1.0","revision":"example-engine"}},"fingerprint":"4ad2f15d8d8e53bf17d3eea0d36ba55c40b48f5b8fd6f03375ba1797779f491e","request_id":"sample-1","account_generation":"0","payload":{"type":"prepare_import","input":{"type":"sample","id":"syn-welcome"}}}
```

## Evidence boundary and remaining gates

Native contract tests exercise actual Rust validators, JSON/SHA-256 and pure state
machines. They do not implement or mock an engine/browser to claim integration.
Normative assignment/score records are contract inputs, not live game predictions.
Independent arithmetic distance enumeration, signed hand cases, restriction relations,
adversarial identities and commit-point failure schedules provide scoped oracles.
Complete search and causal metric claims still need independent engine evidence.

Repair evidence: 2026-10-27 UTC, Linux, Rust/Cargo 1.98.1; base HEAD
`fd40cb828669236bd4097a96b68c310096173a9b` plus uncommitted contract work. Source/test
manifest fingerprint is `de95d392cb66be1694d76d00605a6c268ed878434710fc13527fef35862d32c4`,
using `sha256sum crates/aaru-contract/Cargo.toml crates/aaru-contract/src/*.rs
crates/aaru-contract/tests/*.rs | sha256sum` from the repository root. No source
changes followed the fault-restoration verification. Evidence below concerns this
Rust boundary, not production browser/storage or live-game correctness.

| Scoped claim | Independent oracle and actual boundary | Targeted sensitivity |
| --- | --- | --- |
| Included equipment cannot bypass unknown owner availability | Weapon and artifact owner c7 excluded from c0–c3 subset: reject included/used gear; unrelated unequipped-copy control passes | Invert affected-owner closure; separately check recipient instead of donor availability: each fails `included_equipment_closes_unknown_owner_availability_without_blocking_unrelated_subsets` |
| Donor fallback/loss use physical availability, imported owner remains provenance | Original c1 versus selected alternate c5; legal fallback and no-fallback controls, different before/after forms, misattribution and pseudo-transfer rejection | Exact-ID fallback; exact-ID donor report; omit same-availability transfer guard: each fails `donor_fallback_and_loss_follow_participating_forms_but_preserve_imported_owner` |
| Reset abort restores usability at the advanced epoch | Confirmed abort keeps old active/durable account; Evaluate and Export then succeed; late preview stays rejected | Omit reset disarming: `confirmed_reset_abort_restores_usability_without_rolling_back_the_reset_epoch` fails |
| Uncertain/committed reset cannot resurrect prior state | Writing blocks work/restart/recovery; exact settled old/candidate records resolve abort/commit; lost callback/ack then restart recovers tombstone without another write | Accept arbitrary newer settled record: `uncertain_reset_storage_stays_blocked_until_a_settled_read_proves_abort` fails; reject matching tombstone recovery: `committed_reset_survives_lost_acknowledgement_and_restart_without_another_clear` fails |
| Unavailable IDB does not require a fictitious durable read | Initial SessionOnly activation, retained-backup restart, unavailable clear failure exit, explicit memory discard and empty-session restart | Invent revision-zero Durable recovery: `unavailable_idb_allows_initial_session_activation_restart_and_explicit_memory_discard` fails |
| Foreground preparation/export are cancellable, durable transitions protected | Import/preferences/export/catalog passing-success controls, explicit Cancel and late-success rejection; saving/activation/reset/recovery reject supersession | Remove preparation from cancellable set: `foreground_preparation_export_and_catalog_are_explicitly_cancellable` fails |
| Each reached search limit matches its counter | Each node family at 1/99/100/101 against 100; boundary passes, under/over fail, multiple/duplicate claims checked; presentation truncation separate | Replace equality with upper-bound-only check: `reached_search_limits_require_the_exact_admitted_node_boundary` fails at count 1 |
| Cross-team uniqueness assertion is nonvacuous | Exact characters/availability/items, distinct physical-copy control passes; only reuse plus exact derived item-set change rejects | Permit already-seen weapons while retaining exact footprint collection: `cross_team_weapon_reuse_changes_only_the_physical_copy` fails at duplicate acceptance |
| Numeric/scoring and assignment restrictions retain their scoped evidence | Signed hand arithmetic, independent distance oracle, finite restriction enumeration, normalization/unknown/frontier cases | Existing independent oracles retained; optimizer/mechanics proofs remain open |

Each listed fault was applied alone, its named locked integration test executed
with `cargo test -p aaru-contract --locked --test invariants <name> -- --exact`,
and produced a behavioral assertion failure (exit 101), not a compile failure.
All twelve faults were reverted exactly: SHA-256 of the five changed source files
and invariant test file matched their pre-challenge values. Disposable raw outputs
are `/tmp/aaru-g1-fault-{owner-scope,owner-move,donor-fallback,donor-report,
pseudo-transfer,reset-abort,reset-recovery,reset-settlement,session-read,
cancel-preparation,search-boundary,cross-team-copy}.log`.

After restoration, executed successfully:
- `cargo fmt --all -- --check`
- `cargo test --workspace --locked`: 34 contract, 18 parent-policy unit and
  4 parent staged-boundary tests passed; none failed or were ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo clippy -p aaru-contract --target wasm32-unknown-unknown --all-targets --locked -- -D warnings`
- `cargo check -p aaru-contract --target wasm32-unknown-unknown --locked`
- `pre-commit run --all-files` (4.6.2): all four required hooks passed.
- `cargo audit --deny warnings` (0.22.2): current 52-dependency lock graph passed.
- `cargo deny --locked check` (0.20.2): advisories, bans, licenses and sources passed.
- `cargo run -p aaru-repo-policy --locked -- framework-advisories`: passed.

These local executions are not hosted CI or G1 acceptance. Nothing has been staged
or committed by this repair. Parent's staged diff fingerprint remained
`a23b901b9df55b45f4a48bb66093e8622c25683e0f7c7d1dab5f5f6f099521ff`
(`git diff --cached | sha256sum`) before/after the repair checks.

Remaining gates: bulk codec/control-plane architecture and measured optimized
production responsiveness; native/WASM runtime parity; real worker/CSP/subpath/server
privacy; IndexedDB completion/abort/CAS/reload; Web Locks/two tabs; files/downloads/reset;
accessibility; hosted Linux/Windows/security enforcement; verified mechanics/release
coverage matrix. Compilation for wasm32 is not runtime parity or browser evidence.
G1 is not accepted and engine/UI implementation must not start until parent review.
