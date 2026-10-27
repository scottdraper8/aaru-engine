# Shared contract freeze — v1

This is the builder contract, not a live-mechanics or release-readiness claim.
`crates/aaru-contract` is authoritative for field names, enum variants and bounds.
All first-party implementations and tests are Rust. Transport is UTF-8 JSON text,
not JavaScript objects; never parse account/protocol JSON through JavaScript.

## Ownership and fixed paths

| Owner | Packages and paths |
| --- | --- |
| Coordinator | `aaru-contract` (`crates/aaru-contract/**`), this document, shared root dependencies/contracts; root lockfile integration |
| Engine thread | `aaru-engine` (`crates/aaru-engine/**`), `aaru-wasm` (`crates/aaru-wasm/**`), `data/**`, `fixtures/**` |
| UI thread | `aaru-web` (`crates/aaru-web/**`), `web/**`, `tools/browser-tests/**` (package `aaru-browser-tests`), player help/README |
| Parent coordinator | Policy, CI, hooks, toolchain, `tools/xtask/**` (package `aaru-xtask`), acceptance/evidence docs |

Each implementation thread MAY own its crate-local Cargo manifest and regenerate
its worktree `Cargo.lock` as a generated integration artifact. Coordinator resolves
lockfile merges. Shared semantic contracts and root dependencies remain
coordinator-owned. The root member glob is `crates/*` alongside
`tools/repo-policy`; new tool packages outside that glob require coordinator
membership changes. Do not edit another thread's source/fixtures.

Dependency direction: web -> contract ONLY; engine -> contract; wasm -> engine
and contract. No UI dependency on an unfinished engine. No temporary protocol,
fake worker, duplicated scoring or mocked worker results in browser tests.

Fixed public assets: `fixtures/catalog.json`, `data/mechanics/manifest.json`.
Dedicated module-worker entry: `pkg/aaru-wasm/worker.js` (WORKER_MODULE_PATH).
Its generated loader imports/default-initializes `./aaru_wasm.js`; those bindings
load colocated `aaru_wasm_bg.wasm`. Main generated loader is
`pkg/aaru-web/main.js` (WEB_MODULE_PATH), importing/default-initializing
`./aaru_web.js` and its `aaru_web_bg.wasm`. `web/index.html` and `web/styles.css`
are source shell/style. Parent Rust build tooling generates the two minimal
loaders; there is no authored JS source. Loaders only initialize Rust modules,
never route messages or implement state/validation. UI creates a module Worker
at WORKER_MODULE_PATH; do not point Worker directly at uninitialized binding glue.
`aaru-wasm` owns `#[wasm_bindgen(start)] pub fn start_worker()` and Rust string
message listeners. `aaru-web` owns `#[wasm_bindgen(start)] pub fn start_web()`.
Both adapter packages produce `cdylib`/`rlib`; engine is a Rust library.
These paths/adapter entrypoints are integration requirements, not a claim the
files or adapter implementations already exist.

Fixed catalog IDs (the `SampleId` enum):

| ID | Account path | Mechanics |
| --- | --- | --- |
| `syn-welcome` | `fixtures/canonical/syn-welcome.json` | `data/mechanics/synthetic/syn-welcome.json` |
| `syn-scarcity` | `fixtures/canonical/syn-scarcity.json` | `data/mechanics/synthetic/syn-scarcity.json` |
| `syn-stress` | `fixtures/canonical/syn-stress.json` | `data/mechanics/synthetic/syn-stress.json` |
| `good-unsupported-preview` | `fixtures/good/good-unsupported-preview.json` | none; preview/export only |

Synthetic definitions use `syn:` prefixes. Catalog files use `Catalog` /
`SampleEntry` / `MechanicsEntry`; paths are bundled relative assets, never URLs
from account files. Mechanics manifest uses `Vec<MechanicsEntry>`. Engine owns
content hashes, fabrication labels, supported operations, diagnostic codes,
oracle-case references and public mechanics ontology. Do not duplicate effect,
formula, predicate, reaction or kit schemas in the web/contract crate.

## Entry points and versioning

- `wire::decode_request(&str) -> ContractResult<WorkerRequest>`: strict structural
  parsing plus request-level resource validation.
- `wire::decode_account(&str) -> ContractResult<ValidatedAccount>`: canonical
  import; always imported trust, even if it was a sample backup.
- `wire::decode::<WorkerEvent>` and `wire::encode`: strict wire codec.
- `AccountSnapshot::validate`, `ValidatedAccount::{imported,bundled}`,
  `DecisionSpec::validate_for`, `Allocation::validate`,
  `DecisionResult::validate_for`, `ComparisonResult::validate_for`,
  `SensitivityResult::validate_for`: shared
  identity, origin, assignment, numeric and result-context invariants.
- `PreparedRegistry::{prepare,activate,clear}`: digest-bound, single-use prepared
  candidate lifecycle. `SessionGate::{register,accept,generation}`: UI stale
  event filtering, including objective changes within one generation.
- `Fixed::{from_decimal,from_percentage_points,checked_add,checked_sub,
  checked_mul,checked_div}`, `Objective::{validate,score}`,
  `ScoreVector::{validate,pareto_against}`: shared numeric semantics.
- `wire::{canonical_backup_digest,account_semantic_digest,decision_spec_digest,
  result_fingerprint}`: reproducibility and storage acknowledgement hashing.
- `ownership`: full physical-item ownership map for comparison worlds.

AccountVersion, ProtocolVersion, CatalogVersion and ImportProfileVersion are
independent newtypes, each currently serializing as JSON integer **1** and
rejecting every other version. No migration/fallback. GOOD dialect version is a
separate upstream number; profile revision 1 does NOT assert versions 1–3 all
have verified semantics. Engine must reject unsupported dialects and unresolved
profile semantics explicitly before normalization. Mechanics revision/hash/game
version, engine version/revision, and objective/encounter/assumptions/search
revisions are separate required fields. A changed semantic contract requires a
coordinated version update and both consumers, not unilateral new fields.

All structs/tagged enums reject unknown fields; optional scalar fields serialize
as `null` and mean unsupplied, not zero/default. Unknown import extensions stay
inert in `SourceArchive` with source references/diagnostics, not executable rules
or ad-hoc web fields. `wire::decode` alone is structural; it is NOT a substitute
for contextual validation. Worker calls the contextual validators before work
and before emitting results. UI uses SessionGate before rendering events.

## Inventory, preservation and trust

`Section<T>` is either `{"coverage":"omitted"}` or supplied with completeness
and records. Supplied empty is not omitted. Completeness is exporter-declared
(`unspecified`, `declared_partial`, `declared_complete`), never a scanner audit.
Missing progression remains `null`. Integer fields have u16 representation
bounds and basic positivity checks only; engine owns source-backed progression
relationships and eligibility. Do not invent live caps from representation bounds.

Definition IDs are NOT physical IDs. All canonical character/item IDs must be
globally unique. Exact canonical IDs survive backup restore. GOOD records without
suitable upstream identity use snapshot-scoped IDs:
`snapshot-<lowercase-source-sha256>` for snapshot identity and
`<sha256>/characters/<zero-based-index>`, `<sha256>/weapons/<index>`,
`<sha256>/artifacts/<index>` for instance IDs. The digest is the original UTF-8
bytes, not normalized JSON. Identical records remain different copies. Imports
replace snapshots after confirmation; no guessed cross-import reconciliation.

OwnerRef distinguishes unequipped, resolved character and unresolved external
owner. Unresolved owners reserve their items and occupy equipped slots just like
known owners; they are never free search resources. Two equipped weapons or two
artifacts in the same owner/slot are invalid, including unresolved owners.
Missing owned-character sections must not manufacture characters to resolve keys.

Game lock (`Option<bool>`) is preserved account data, distinct from Aaru
preferences. Default `AllocationPreferences` protects known game-locked items
from reassignment. User permission to allow reassignment changes only the Aaru
policy, never `game_lock`. Explicit protections/pins/exclusions remain binding.
A pinned item need not participate in a selected party; if used it must have the
pinned owner. Unresolved external reservations cannot be overridden by the
imported-lock policy. Engine additionally validates weapon compatibility,
set/main-stat rules, derived-stat minima and mechanics constraints.

`SourceArchive` contains the exact valid original UTF-8 source and SHA-256;
whitespace, numeric spelling, extension fields and final newlines are preserved.
It is private state and must never be logged/uploaded or included in public
artifacts. SourceRef points into that archive. Shared checks verify digest and
pointer shape; engine verifies RFC 6901 escaping, pointer existence and correct
record provenance against the original strict parse. Original source export
returns archive bytes unchanged, never regenerated GOOD. Canonical export is an
Aaru backup, not a GOOD export, and includes preferences, IDs and the archive.
On restore use the existing preserved source archive; do not nest a backup as a
new archive recursively. Preserved source means the earliest archived inventory
source, not the formatting of each subsequent Aaru backup wrapper. Aaru backups
promise canonical ID/policy/archive preservation, not byte-identical wrapper
formatting. If no archive exists, source export is blocked explicitly.

Raw input is bounded before parse: 16 MiB UTF-8 file, depth 64, 256 characters,
4,096 weapons, 16,384 artifacts, 4,096-byte ordinary strings and 128 rendered
issues. Wire/backup transport is bounded at 128 MiB (escaping source text expands
it); the only long-string field names are `text` and `utf8`, still capped at
128 MiB at the codec; contextual validation caps archived original source and
GOOD file text at 16 MiB. Containers cap at 65,536 entries; no trailing data/BOM, duplicate object
properties (including escaped-equivalent names), coercion or silent clamping.
GOOD import text requires its own strict parse after outer-message parsing.
Canonical backup restore is a structured backup decode (`wire::decode_account`),
not re-archiving it as a raw GOOD file; `PrepareImport/File` with format `aaru`
and storage restore use the 128 MiB wire/backup limit. That expansion allowance
is for complete backups containing escaped original source, not a larger GOOD
raw-input allowance. The outer message must also fit the wire budget; oversize
inputs fail explicitly and never truncate.

Invalid input never replaces active state. Unsupported but preserved inventory
is distinct from evaluable inventory. `EvaluationCapability` names operations,
scope and blockers. A missing/unavailable stat is not an evaluable zero.
`ImportedQuantity::Unsupported` retains its original lexeme inertly.

**Synthetic isolation:** ValidatedAccount is non-deserializable and its trust
marker is private. Every external GOOD/canonical import uses `imported`.
Only the engine's bundled, hash-checked catalog loader calls `bundled` after
`ImportInput::Sample`; import text cannot name a trust marker. Synthetic mechanics
must match that exact synthetic SampleId. GOOD archives cannot acquire synthetic
trust even through `bundled`. Synthetic samples cannot use live mechanics.
`good-unsupported-preview` never permits synthetic mechanics. Reloading an
exported synthetic backup does not restore synthetic trust: ask the player to
load the bundled sample again. Imported definitions resembling synthetic IDs
remain imported and blocked; never infer trust from IDs/provenance labels.

## Shared numerical rules

Fixed represents signed i64 millionths: range
-9,223,372,036,854.775808 through 9,223,372,036,854.775807 units. The wire is the
canonical signed **scaled-integer decimal string**, e.g. `"39000"` means 0.039,
not 39,000 units. No leading plus/zeroes or negative zero. Counter/generation
uses canonical u64 decimal strings. Small bounded counts/version integers are
JSON numbers; scaled quantities never pass through JS Number.

Decimal lexemes are converted exactly, including scientific notation, using
checked i128 intermediates; inexact millionths, overflow and malformed input are
errors, never rounded to zero. Preserve unsupported values via ImportedQuantity.
`from_percentage_points("3.9")` is 0.039 (`"39000"` scaled). The GOOD importer
may use that conversion only after verifying that field's percentage convention.

Add/subtract are exact with range checks. Multiply divides the i128 product by
1,000,000; divide multiplies the numerator by 1,000,000. Each rounds once, nearest
integer, ties to even, symmetric for negative values. Division by zero and
narrowing overflow are errors. No saturation, epsilon, floating ranking or
platform-dependent rounding. The objective dot product accumulates all weighted
products with checked i128 addition and rounds **once at the end**. Weights are
nonnegative, at least one positive; they are not implicitly normalized.
Unavailable dimensions with positive weight block scoring; zero weights do not.
Engine-specific formula rounding boundaries belong to the engine's versioned
mechanics model, using these primitives only at declared boundaries.

All nine ScoreVector keys must exist. All maximize; engine defines dimension
normalization/provenance under its dataset revision. Pareto comparisons use exact
integers; unknown dimensions make vectors incomparable. Frontier/Dominated status
is among the returned allocations, not proof about unsearched space. Global
search completeness is separate. A comparative presentation score is not DPS.
Resource efficiency is explicit modeled cost, not a guessed donor penalty.

## Decisions, assignments, comparison and evidence

DecisionSpec makes mechanics, scope, objective, encounters, assumptions,
constraints and deterministic search limits explicit. One to three encounters
means one to three simultaneous four-character teams. Engine rejects unknown
assumption/encounter keys, unsupported weighted metrics, unmet required rules,
invalid progression and missing mechanics. No default combat assumptions.

BuildAssignment identifies character, optional weapon, exact artifact slots,
raw metrics and causal reasons. Optional weapon/missing artifacts means actually
unassigned, NOT unknown equipped gear; engine may evaluate such a build only if
its declared scope/mechanics support it. Each Allocation lists exact teams and
ResourceFootprint. Shared checks reject duplicate physical resources, wrong
artifact slots, false footprints, wrong objective scores and failed hard
constraints. Mechanics legality, completeness of constraints and safe pruning
remain engine proof obligations, not properties Serde can establish.

Rank by descending presentation score, then ascending AllocationId bytewise.
Engine IDs must be deterministic functions of sorted canonical assignments,
not enumeration timing. Sort character/item/build/team keys bytewise; preserve
encounter order. CausalReason causes precede effects (no cycles/dangling causes),
and reason IDs attached to results must resolve. ReasonCode is stable and factual;
UI renders plain-English consequences, not invented post-hoc explanations.
LocalizedUncertainty attaches fidelity/assumptions/sources to an actual component.
Origin and fidelity are independent; exact arithmetic is not verified game truth.

Complete means exhaustive within declared scope/constraints, including globally
necessary alternate builds and all unique resource conflicts. Infeasible means
exhaustive with no feasible allocation. Bounded means best found, with limits;
Cancelled may retain partial allocations but proves no optimum. Unsupported
mechanics is a separate blocked status. Counters are actual work, not invented
percentages. Bounds require rule provenance. Result validation checks consistency,
not the truth of an engine's claimed search proof. No top-N/greedy shortcut may
be labeled complete. Max results is a presentation bound, not a search cutoff.

ComparisonResult includes complete before/after decisions AND full ownership
maps for all physical items, original owner, recipient, donor (when resolved),
other affected characters, replacements through build IDs, global delta and
causal paths. Baseline spec is unchanged. The engine applies the requested move,
reserves it to the recipient, reoptimizes donor replacements/artifacts/third-party
competition and evaluates the hypothetical account. Original source and game
locks are unchanged. After fingerprint hashes that hypothetical ownership.
Use `ComparisonResult::validate_for` against the original trusted account. It
checks complete ownership maps, movement protections, explicit donor identity,
contextual before/after validation and the global score subtraction.
A ProvenWithinDeclaredDomain claim requires both searches Complete; Bounded
comparisons must say BestFoundDifference, not proven account improvement.
Character metric deltas and causal completeness still require engine evidence.

Sensitivity passes explicit scenario assumptions and encounter replacements.
The baseline objective, mechanics, scope, constraints and search policy stay
fixed. Each ScenarioResult contains the full rerun and ranking-change/global-delta
summary; shared validation checks context, ranked IDs and score subtraction.
Engine must establish causal/mechanics validity of the reruns. No probabilistic confidence is
invented from scenarios. At most 16 uniquely identified scenarios per request.

Reproducibility excludes timing. account_semantic_digest sorts record IDs and
pins, clears source references/archive/provenance, sorts artifact substats and
retains progression, ownership, section coverage and policy. Spec digest hashes
compact Rust JSON (BTreeMap/BTreeSet ordered, vector order meaningful). Result
fingerprint hashes the compact JSON tuple
`["aaru-result-v1", account_semantic_digest, decision_spec_digest,
engine_version, engine_revision]`. canonical_backup_digest instead hashes exact
compact AccountSnapshot serialization, including archive/provenance, for the
storage acknowledgement. Hashes are lowercase SHA-256, not security signatures.

## Protocol lifecycle

Every envelope has protocol_version, request_id, account_generation and payload.
Initial generation is `"0"`, with no active account. Requests for inventory work
without an active account receive a structured InvalidInput error. IDs are
unique per session (maximum 4,096); reload creates a new worker/session.

GetCatalog does not activate a sample. PrepareImport validates into a preview
with token, full canonical account, digest, issues, capability and trusted sample
indicator. It does not mutate the active account. New successful prepare replaces
the prior candidate; invalid preparation leaves the prior usable state intact.
Token binds current generation and exact canonical backup digest. Persisted
activation requires UI's successful namespaced IndexedDB transaction for that
same backup; storage failure instead offers explicit SessionOnly or abort.
Never send Persisted before the transaction completes. PreparedRegistry consumes
only a successful matching activation. Reset clears prepared candidates.

All ordinary responses echo request generation. Activated/ResetAcknowledged
carry **new generation = previous + 1**, plus previous_generation; the UI accepts
that transition only for its pending matching request. Reset cannot wrap a u64.
Activation/reset cancels old computations and invalidates old tokens. A failed
transition emits Error at the unchanged generation. Clear only Aaru storage;
engine worker reset and UI download/object-URL cleanup are real responsibilities.

Evaluate/Compare/Sensitivity emit Progress followed by their terminal family or
Error. Cancellation targets a request ID and acknowledges whether it was running;
unknown targets report was_running=false. A cancellation acknowledgement does
not certify that work was exhaustive. UI immediately invalidates the target.
SessionGate also rejects results from superseded objectives in the SAME generation,
unsolicited transitions, stale generations, wrong response families and duplicate
terminals. Once a transition is pending, UI must not offer new decision actions
until it succeeds/fails. Engine must independently reject stale requests/tokens.

Export returns Download (UTF-8 text, literal filename, MIME type, digest). UI
creates a local download and verifies export/reset boundaries in real-browser
Rust tests. Never treat filename as a path or imported strings as HTML. Errors
have stable IssueCode, bounded structured issues and recoverability; player copy
must explain the problem and recovery. Do not expose internal account text in
errors. ContractError is for native validation; ProtocolError is the transport.

## Serialized examples (compiled by contract tests)

Minimal partial canonical backup (not an evaluable account):

```json
{"schema_version":1,"id":"empty-account","characters":{"coverage":"omitted"},"weapons":{"coverage":"supplied","completeness":"declared_partial","records":[]},"artifacts":{"coverage":"omitted"},"preferences":{"game_lock_policy":"protect_from_reassignment","protected_items":[],"pinned":[],"excluded_characters":[],"excluded_items":[]},"source_archive":null,"provenance":[]}
```

```json
{"protocol_version":1,"request_id":"catalog-1","account_generation":"0","payload":{"type":"get_catalog"}}
```

```json
{"protocol_version":1,"request_id":"sample-1","account_generation":"0","payload":{"type":"prepare_import","input":{"type":"sample","id":"syn-welcome"}}}
```

```json
{"protocol_version":1,"request_id":"import-1","account_generation":"0","payload":{"type":"prepare_import","input":{"type":"file","format":"good","text":"{\"format\":\"GOOD\",\"version\":3,\"source\":\"Example\"}"}}}
```

```json
{"protocol_version":1,"request_id":"activate-1","account_generation":"0","payload":{"type":"activate_prepared","token":"prepared-1","storage":{"type":"session_only"}}}
```

The digest below is syntactically valid example metadata, not an assertion that
a mechanics fixture already exists. The decision specification is explicit;
the actual engine may reject it for absent dataset/coverage.

```json
{"protocol_version":1,"request_id":"evaluate-1","account_generation":"1","payload":{"type":"evaluate","spec":{"mechanics":{"origin":"synthetic","sample":"syn-welcome","revision":"syn-v1","sha256":"0000000000000000000000000000000000000000000000000000000000000000"},"scope":{"type":"whole_supplied_inventory"},"objective":{"revision":"objective-v1","label":"Balanced","weights":{"output_potential":"1000000"}},"encounters":[{"revision":"encounter-v1","definition":"syn:encounter/one","label":"Sample target","target_count":1,"parameters":{},"provenance":[]}],"assumptions":{"revision":"assumptions-v1","values":{}},"constraints":{"required_characters":[],"excluded_characters":[],"excluded_items":[],"pinned_assignments":[],"allowed_sets":{},"allowed_main_stats":{},"metric_minima":{},"required_rules":[]},"search":{"revision":"search-v1","max_build_nodes":"100","max_team_nodes":"100","max_allocation_nodes":"100","max_results":10,"work_units_per_slice":10}}}}
```

```json
{"protocol_version":1,"request_id":"compare-1","account_generation":"1","payload":{"type":"compare","baseline":{"mechanics":{"origin":"synthetic","sample":"syn-welcome","revision":"syn-v1","sha256":"0000000000000000000000000000000000000000000000000000000000000000"},"scope":{"type":"whole_supplied_inventory"},"objective":{"revision":"objective-v1","label":"Balanced","weights":{"output_potential":"1000000"}},"encounters":[{"revision":"encounter-v1","definition":"syn:encounter/one","label":"Sample target","target_count":1,"parameters":{},"provenance":[]}],"assumptions":{"revision":"assumptions-v1","values":{}},"constraints":{"required_characters":[],"excluded_characters":[],"excluded_items":[],"pinned_assignments":[],"allowed_sets":{},"allowed_main_stats":{},"metric_minima":{},"required_rules":[]},"search":{"revision":"search-v1","max_build_nodes":"100","max_team_nodes":"100","max_allocation_nodes":"100","max_results":10,"work_units_per_slice":10}},"movement":{"item":"w0","recipient":"c0"}}}
```

```json
{"protocol_version":1,"request_id":"sensitivity-1","account_generation":"1","payload":{"type":"sensitivity","baseline":{"mechanics":{"origin":"synthetic","sample":"syn-welcome","revision":"syn-v1","sha256":"0000000000000000000000000000000000000000000000000000000000000000"},"scope":{"type":"whole_supplied_inventory"},"objective":{"revision":"objective-v1","label":"Balanced","weights":{"output_potential":"1000000"}},"encounters":[{"revision":"encounter-v1","definition":"syn:encounter/one","label":"Sample target","target_count":1,"parameters":{},"provenance":[]}],"assumptions":{"revision":"assumptions-v1","values":{}},"constraints":{"required_characters":[],"excluded_characters":[],"excluded_items":[],"pinned_assignments":[],"allowed_sets":{},"allowed_main_stats":{},"metric_minima":{},"required_rules":[]},"search":{"revision":"search-v1","max_build_nodes":"100","max_team_nodes":"100","max_allocation_nodes":"100","max_results":10,"work_units_per_slice":10}},"scenarios":[{"id":"scenario-1","label":"Alternative assumptions","assumptions":{"revision":"assumptions-v2","values":{"syn:rotation/seconds":{"type":"quantity","value":"24000000"}}},"encounters":[{"revision":"encounter-v1","definition":"syn:encounter/one","label":"Sample target","target_count":1,"parameters":{},"provenance":[]}]}]}}
```

```json
{"protocol_version":1,"request_id":"export-1","account_generation":"1","payload":{"type":"export","format":"canonical"}}
```

```json
{"protocol_version":1,"request_id":"cancel-1","account_generation":"1","payload":{"type":"cancel","target":"evaluate-1"}}
```

```json
{"protocol_version":1,"request_id":"reset-1","account_generation":"1","payload":{"type":"reset"}}
```

```json
{"protocol_version":1,"request_id":"evaluate-1","account_generation":"1","payload":{"type":"progress","progress":{"phase":"allocation","completed_units":"12","total_units":null,"counters":{"build_nodes":"80","team_nodes":"24","allocation_nodes":"12","pruned_nodes":"3"}}}}
```

```json
{"protocol_version":1,"request_id":"reset-1","account_generation":"2","payload":{"type":"reset_acknowledged","previous_generation":"1"}}
```

```json
{"protocol_version":1,"request_id":"evaluate-1","account_generation":"1","payload":{"type":"error","error":{"code":"unsupported_mechanics","message":"This account cannot be evaluated with the selected mechanics.","issues":[],"recoverable":true}}}
```

## Evidence and remaining gates

Contract tests use real Serde/JSON/SHA-256 and public validators, no engine or
browser substitutes. Independent numeric oracles include signed hand-calculated
half-even cases and nearest-candidate distance enumeration. Fault cases exercise
copy reuse across teams, false footprints, unknown dimensions, source changes,
stale objectives/generations, cancelled results, mismatched import digests,
synthetic trust, incomplete search claims and dangling/cyclic reasons. These
support only shared boundary claims, not importer/game/search/browser correctness.
Serialized examples above are executable tests, not fabricated worker results.

Executed sensitivity challenges (native Rust 1.98.1): changing ties-to-even parity
from odd to even made the hand-calculated boundary, independent distance oracle
and single-round weighting tests fail (3 failures); inverting the ordinary-event
generation guard made the UI stale-event test fail on an old-generation event.
Both deliberate faults were reverted immediately; no fault is part of the
contract. These establish sensor sensitivity to those two faults only.

Executed freeze gates: `cargo fmt --all`; `cargo test --workspace --locked`
(16 contract invariant tests and 10 concurrent repository-policy tests, no failed
or ignored tests); `cargo clippy --workspace --all-targets --locked -- -D warnings`;
`cargo check -p aaru-contract --target wasm32-unknown-unknown --locked`; and
`pre-commit run --all-files` all passed. Pre-commit's JSON file hook had no
applicable files; no tests/hooks were disabled or bypassed. The parent-owned
policy source changed concurrently; those test results describe the working tree
at execution, not a claim that parent changes are included in this contract
commit. Initial baseline at e8bff95 had seven policy tests passing.

Engine still owes pinned GOOD profile verification, mechanics coverage and
independent search/counterfactual oracles. UI still owes real production-worker,
file/storage/download/reset, accessibility and privacy executions. Native Windows,
production WASM/browser, hosted security/performance and release evidence remain
separate gates. The contract freeze is not a recommendation release.
