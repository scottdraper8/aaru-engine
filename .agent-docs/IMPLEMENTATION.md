# Aaru Engine — implementation contract

## Product and authority

Build the FULL local-first, beginner-friendly Genshin account decision website for
Windows and Linux. The complete `genshin_decision_engine_design_memo.md`, original
16 owner requirements in `REQUIREMENTS.md`, and all19 functional/12 browser rows in
`RELEASE_GATES.md` remain binding. Intermediate contracts, synthetic models and
research are NOT a release-ready product or verified live-account DPS.

Read AGENTS and REQUIREMENTS before substantive work; read the full memo for
architecture/scope. Both testing references govern evidence. The PDF is reference
only: no PDF-specific hooks/CI/tooling or reference edits. Its ignored complete
extraction is a reading aid, not an application workflow.

First-party application/UI/tests/repository tools use Rust. Static HTML/CSS and
generated INITIALIZATION-ONLY WASM bindings/loaders are allowed, not handwritten
JS/execute_script, Node/Python application/tool stacks or project shell scripts.
Keep required external pre-commit4.6.2; its Python implementation is not permission
to add first-party Python. Minimal code comments applies retroactively; retain only
organizational headers/absolutely necessary explanation or tooling metadata.

Exact upstream PolyForm Strict1.0.0, exact tools/direct dependencies/immutable
actions/framework hashes, Cargo.lock and locked installs remain enforced. No legacy
aliases/shims/dead scaffolding, suppressions/ignored or focused tests, bypasses,
skipped required jobs, weakened gates or automatic merging. All-files pre-commit
BEFORE every normal hooked commit; complete staged blobs INCLUDING new files checked.

## Runtime and data decisions

Thin Rust DOM directly owns TWO sibling module Workers (Rust UI supervisor/engine)
and their private MessageChannel. No nested workers/SharedArrayBuffer/giant DOM
JSON/hash or account integers through JSNumber. DOM owns WebLock before private
reads, actual IndexedDB lifecycle/settlement, compact authority, opaque immutable
account+UI anchors, bounded pages and URLs/Blob ledger. Supervisor owns full UI/
spec/codecs/contextual admission/checkpoints; engine owns import/catalog/mechanics/
search. UI consumes public metadata, never engine-private scoring/guessed IDs.

Preparation precedes durable reservation/Writing. Transaction COMPLETE commits,
not request success/worker death/timeout. Install the exact account/UI pair in DOM
before notification, exact ack/bind-once/ordered refresh, no preference resets on
same-account edits. Replacement carries/resets settings only by explicit exact-target
choice. Retained sessionB must not silently become savedA. Current corrupt/unreadable
headers, historical head and unavailable reads are distinct. Memory discard is not
durable erase; guarded fresh-store clearing is not authorized by failed reads alone.

Budgets: raw GOOD/source archive16MiB; canonical Aaru offer/logical wire128MiB.
EACH DecisionResult16MiB including nested decisions; complete comparison/sensitivity
bodies128MiB INCLUDING own metadata; FINAL enclosing request/event independently
128MiB INCLUDING wrapper. An exactly128MiB body need not fit after wrapping.
Controls/pages16KiB, page24 rows maximum, source2 distinct across ALL holders,
candidate/queued replacement1, extra caches32MiB, each worker linear max512MiB,
Blob ledger256MiB. Account for copies/backings/callers/acks/exports across holders;
compact unselected saved bodies to truthful heads. Native maxima aren't feasibility
for every simultaneous workload. Search slices<=256 admitted units/4ms with real task
yields; visible cancellation50ms. Whole codecs are terminable bulk, NOT4ms primitives.
DOM invalidates authority and directly terminates BOTH siblings for cancellable work;
durable reservation barriers remain non-cancellable and explicit.

Preserve physical copies/owners/progression/locks/provenance, incomplete sections,
mutually exclusive forms and omitted/partial/unknown/supplied-empty distinctions.
GOOD presence isn't completeness, absent materials aren't zero/infinite, parser
ranges aren't game legality, source positions aren't guaranteed cross-export identity.
Use GOOD_REFERENCE's pinned documentation and exact ratio/unboosted talent semantics.
Unknowns block only dependent claims, not unrelated supported inventory/equipment.

Deterministic checked arithmetic, explicit objectives/assumptions/encounters, unique
resources, joint team/build/global Pareto search, donor/form opportunity costs and
cold-vs-incremental recomputation must be real engine behavior. Roster marginal
value and budgeted investment are DISTINCT operations, not equipment comparisons.
Reasons resolve through SubjectContext and mandatory SessionGate.accept_result;
ordinary accept rejects result terminals. Source/profile/engine/input versions and
causal/sensitivity/limits remain visible; synthetic utility isn't verified DPS.

## Current evidence and blockers

Branch `feat/rust-decision-engine`; PUBLIC https://github.com/scottdraper8/aaru-engine.
Main requires PR/admin enforcement/strict latest-base Quality gate (Actions app15368),
resolved conversations, no force/deletion. Alerts/security fixes/secret scanning/push
protection enabled, auto-merge disabled; actual pin-preserving Dependabot fix behavior
still unverified. No direct-main bypass/auto-merge. Linux BLOCK/Windows AUDIT hardening
first in EVERY job, least privilege/endpoints unchanged. No paid subscriptions.

Native G1 baseline accepted at1b664913a3d25f86e97b18b1069a442276e9b760,
run40582912311attempt1 all Linux/Windows/security/Quality successful. Material native
subgate H0-M accepted at50119ab8868ece6feaf8f0ff0ef5fa3953fa3a4e,
run40609987311attempt1 all Linux/Windows/security/Quality successful. Parent decision
tmp/H0_MATERIAL_ACCEPTANCE.md; NOT full P1a/H0/runtime or product acceptance. Historical5b9c64f Linux checkout failed;
stuck40579865822 was cancelled, not relabeled green. Reviewed StepSecurityv2.22.3
46b06cea07f9e64621fdf066ba77ac6915163d43 succeeded without reduced hardening; not
universal resolver/root-cause proof. Prior failures/probes remain unchanged.

H0 adds mandatory account schema2 material knowledge/full-u64 decimal strings,
source/provenance/bounds/declarations/public labels/preview counts, protocol2 and
whole-pair recovery/cache commitments. No importer/cost engine/investment API yet.
Initial82/20/4 green +8 restored faults STILL missed strict-field defects. Independent
review and parent reproduced ignored claims on tagged empty variants/completeness
payloads. Parent repaired those with one strict empty-map decoder across28 unit
variants and strict completeness payload decoding, retaining canonical encodings/
Rust variants (no compatibility alias). Permanent malformed/valid controls, actual
request/control/recovery heads and3 restored behavioral faults support the repair.
Full87/20/4/strict native+contractwasm and preserved18+2 owned probes now pass.
Independent repair review repeated those and4 own controls with unchanged source:
ACCEPTABLE for native staged/hosted handoff WITH runtime gates. Complete staged16
files/new tests/all-files/normal hooks/security passed, then exact hosted50119ab
passed. **H0-M material native accepted; broader P1a/H0 interfaces still open.**

Current reports: tmp/G1_ACCEPTANCE.md; h0-material-contract-evidence.md;
h0-material-review-evidence.md (historical FIX REQUIRED); h0-parent-availability-finding.md;
h0-shape-repair-evidence.md and h0-shape-repair-review-evidence.md. INITIAL reviewer
final summary was empty after provider timeouts;
detailed findings were saved and parent repeated ALL18 probes before/after repairs.
Original failing reports/harnesses/logs unchanged. Local OSV DNS timeout was preserved;
normal lookup/HTTPS recovered, then actual fail-closed framework query passed without
policy changes. This is scoped advisory evidence, never zero-vulnerability proof.

## Delivery and next actions

Full replacement plan adopted under tmp/big-build/aaru-engine/phase1/: read ALL
master-plan/architecture/delivery/acceptance/threads.json/qa-review. Parent amendments
applied, old artifacts archived. Orchestrator450363 stays paused; do NOT resume stale
builders. At most TWO nonoverlapping E/U source leases. Parent owns ALL shared
schemas/contracts/manifests/locks/pins/build/bootstrap/server/security/CI/authority;
E engine/WASM runtime src/tests+fixture/mechanics CONTENT; U DOM/supervisor/browser
src/tests/static original HTMLCSS/player help. Park both for shared changes, one
reviewed fingerprint and explicit acknowledgments before resume.

Next: read new p1-{engine,ui}-handoff-plan-evidence proposals; parent must reconcile
actual inventory-presentation/persisted-UI/import
metadata gaps before source launch. Freeze public import/catalog/material and exact
manifest/feature/entrypoint proposals. Allocate GENUINE initial E/U source,
then PARK both for parent manifest/lock/build/loader/server integration; validate ALL
actual native/WASM targets. No stub libs/workers/fake events/missing-target hiding.
P1 proposal tmp/p1-build-runtime-interface-proposal.md isn't executed/frozen.

Require early actual optimized file/sample/catalog/preview/explicit settings choice/
activation/export/checkpoint/storage/cancellation/reload on Windows AND Linux. G0
Linux research used older code/protocol; codec56–96ms, DOM/rAF cancellation isn't
paint, listener/Blob/RSS/revocation/production/Windows behavior remains OPEN.
Then extensive REALISTIC synthetic corpus BEFORE recommendations, broad source-
backed kernel/rights/costs, full search/allocation/equipment/recompute, distinct
roster/investment, causal/sensitivity/accessibility and operational/deepQA/package/
help. Every release row still requires relevant actual evidence, not counts/mocks/
snapshots/coverage/mutation theater. Follow PRODUCT_LANGUAGE's accurate original
Aaru/Sumeru identity/plain English, no spoilers/assets/implied HoYoverse integration.

Source research dossiers are untrusted references. Current gcsim AGPL, historical
MIT not blanket permission; GO software/prose/assets/numbers differ; KQM unresolved.
Progression-cost dossier has useful conditional rows/original arithmetic and source
contradictions, NOT rights/current-game/broad coverage clearance. Community token-
based90/95/100 documentation isn't a universal legality/profile decision. Missing
partial EXP/AR/unlocks/rounding/RNG/history/costs remain unknown, not invented.

PATH=$HOME/.cargo/bin:/tmp/aaru-audit-tools/bin:$PATH; Rust1.98.1/audit0.22.2/deny0.20.2/
wasm-bindgen0.2.129. Follow .agent-deny, never inspect credentials/private accounts/
dependencies/generated output. tmp/CURRENT_HANDOFF.md tracks live tasks; ignored tmp
artifacts are coordination/evidence, not release deliverables or requirement waivers.
