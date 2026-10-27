# Product coverage and release gates

This maps the complete design memo to observable product behavior. It supplements
`REQUIREMENTS.md`; it does not replace or narrow the owner's requirements.
A passing foundation, synthetic milestone or orchestration report is not release.
Do not substitute equipment comparison for roster or investment analysis.

## Current evidence boundary

Commit `72f2048300006ec68b05b110ceb78b31756a72ca` passed all-file pre-commit,
commit-time hooks, 34 native contract tests, 20 policy unit tests and four real
Git/CLI tests. Contract wasm32 strict Clippy, denied-warning cargo-audit,
cargo-deny and live external-framework advisory checks passed. Hosted run
40530179308, attempt 1, passed Linux native, Windows native, dependency security
and aggregate Quality gate, including the new framework query.

These are validator/state-machine and repository enforcement claims. Engine,
production UI, realistic corpus, verified live mechanics and release artifacts
are not implemented. The uncommitted G1 bulk/control/setup integration passed 51
contract tests before seven material interface defects were found. Their composed
repairs closed at native scope; later subject-resolution/current-unreadable-header
repairs passed71 contract/20 policy/4 Git cases, independently repeated by parent.
Independent stable review supports those native seams but accepted a wrong lower
comparison cap. Parent executed and corrected that regression, retaining original
16MiB per-decision/128MiB aggregate-final-wire scopes. Current73/20/4, strict checks,
all-files hooks and security checks pass; three restored full-suite budget faults
fail assertions. See `tmp/g1-budget-scope-evidence.md`. Narrow independent follow-up
passed21 controls and73/20/4 twice; its documentation-only contradiction is corrected.
Complete staged/new-file checks, normal hooks and parent21-probe repeat passed.
Native G1 ACCEPTED at1b664913a3d25f86e97b18b1069a442276e9b760, run40582912311attempt1:
Linux native, Windows native, security and Quality gate all successful. Verified
StepSecurityv2.22.3 retained blocked Linux/audited Windows egress. Prior40579865822
Linux checkout failure/stuck workflow was cancelled, not relabeled green; new success
is not universal resolver proof. Full product plan adopted; P1/H0 integration next.
No real application consumers or production-browser acceptance yet.
Two disposable Linux sibling-browser runs and three runtime sensor challenges support
only their observed primitives, not painted-pixel, production, long-run memory
or Windows browser claims. Final integration/staged/hosted executions remain open.

## Functional coverage

All rows below remain product implementation/evidence gates. Shared types and
native validators alone do not close them. References name design-memo sections.

| Domain | Required observable behavior | Independent acceptance evidence |
| --- | --- | --- |
| Dynamic account state (§2–5,14) | Owned roster/forms, progression, weapons, artifacts, distinct copies, ownership and preferences derive from imports; never a hard-coded owned roster. | Real GOOD/canonical import through the production worker; manually specified expected account facts and physical availability, including duplicate/alternate-form challenges. |
| Incomplete information (§3,14–15) | Omitted, partial, unknown and supplied-empty remain different. Current-loadout empty slots need knowledge; unsupported affected data blocks claims without disabling unrelated supported subsets. | Contrasting omitted/partial/complete-empty/known-occupied inputs; exact failure category and unchanged active account. No roundtrip-only oracle. |
| Input fidelity (§14–15) | Strict UTF-8/JSON, duplicate-key rejection, safe bounded numeric parsing, exact original bytes/source identities, canonical restoration and explicit user assertions. | Raw adversarial inputs and byte-exact downloads, large integer/CRLF/non-ASCII cases, independent source digests; malformed input cannot partially replace state. |
| Realistic synthetic corpus (owner16) | Substantial varied inventories before recommendations: unequipped gear, identical copies, locks/unknown locks, mixed progression/refinements, sparse through large accounts, incomplete sections, conflicts and scarcity. | Reproducible Rust generation plus hand-curated expected facts. Exercise actual import/evaluation paths. Cover realistic classes and stress boundaries, not a fixture-count target or four tiny happy paths. |
| Public mechanics evidence (§5,14–15,20) | Versioned character/weapon/artifact/reaction/encounter data with source provenance, attribution and honest verified/estimated/qualitative/unknown status. | Independently checked numerical and predicate cases, source/version/licensing review and explicit supported-definition/effect map. Memo examples and AI output are not verified facts. |
| Typed effects (§4–6) | Actual applicability, targets, stacks/non-stacking, conditions, on/off-field behavior and snapshot/dynamic distinctions; irrelevant buffs contribute nothing. | Hand-derived affected/unaffected controls, boundary conditions and deliberate predicate/stacking faults. No manually assigned pair scores in place of mechanics. |
| Team-level interactions (§5–6,18) | Roles, reactions/aura, field time, energy/ER, repeatable rotation, defensive utility, sourced special-system predicates and encounter dependence. | Higher-order counterexamples where a third/fourth member changes a pair's outcome; mono-element controls; source-backed calculations and localized unavailable dimensions. |
| Numerical model (§4,15,18–19) | Exact checked deterministic arithmetic; explicit assumptions/probabilities where modeled; raw units and independently revisioned normalization. Synthetic utility never presented as DPS. | Independent rational/hand oracles, signs/overflow/rounding and sensitivity cases; native/optimized-WASM parity on the same consequential workloads. |
| Objectives and constraints (§7,9,19) | Explicit user intent, positive-weight unknown blockers, hard exclusions/pins before ranking, visible soft trade-offs, no hidden combat defaults. | Small normative worlds whose best choice changes under objective/encounter constraints, including invalid/unavailable inputs and preset/data mismatches. |
| Systematic discovery (§3–4,7) | Generate legal four-character teams and evaluate genuine joint effects; deterministic candidate order and tie-breaking. | Independent small-domain enumeration, permutation/resource-label metamorphisms and targeted missing-candidate/pruning faults. |
| Artifact/build search (§4–5,8) | Main stats, sets, actual substats, weapons and model constraints produce legal builds; retain resource-distinct alternatives needed by global allocation. | Independent exhaustive finite inventory oracle, boundary constraints, duplicate-copy and locally dominated-but-globally-necessary cases. |
| Multi-team allocation (§7,24–25) | Best compatible pair/trio under explicit encounters; no reused physical character availability or item; scarce support/item opportunity cost. | Brute-force finite allocation oracle where individually strongest team is not in the globally strongest lineup; exact copy-reuse fault sensitivity. |
| Search/frontier honesty (§7,22) | Complete/infeasible/bounded/cancelled have defined evidence; reached counters match limits; display truncation is separate; Pareto claims have witnesses and unknown dimensions remain incomparable. | Independent complete/bounded edge worlds, actual cutoff runs and non-vacuous witness checks. Coverage or status serialization does not prove exhaustive search. |
| Equipment counterfactuals (§8,22–25) | Exact current-loadout intervention versus unrestricted reoptimization; recipient, donor/forms, displaced/fallback gear, changed builds/teams and global opportunity cost explained. | Hand finite worlds with recipient gain/global loss, legal involved swaps, blocked unrelated moves and the restriction-optimum relation; actual production comparison workflow. |
| Marginal roster value (§2,8,25) | Supported hypothetical acquisition shows new options, freed supports and account-wide gain under the same declared objective; no generic tier/pull prediction. | Independent finite before/after search and resource accounting, existing-form/no-new-availability controls, unsupported-kit rejection. Not satisfied by item moves. |
| Investment value (§2,9,25) | Explicit hypothetical progression/artifact improvements and declared costs/budgets compare account-wide marginal benefit. GOOD does not invent missing material balances. | Hand cost/benefit worlds, constrained-budget allocation, missing-cost blockers and source-backed progression legality. No zero-cost defaults or invented future rolls. |
| Dependency recomputation (§8) | Changed facts invalidate all dependent builds/teams/global allocations but reuse truly unaffected work, with complete semantic/version cache keys. | Incremental versus cold full evaluation agreement, observed affected/unaffected behavior and an omitted-dependency fault that produces a detected stale result. Counters alone are insufficient. |
| Causal explanations (§3,15–17,22) | Reasons arise during actual evaluation, distinguish archetype from quality, expose assumptions/uncertainty and explain donor costs and rejected alternatives. | Independently expected reasons in contrasting worlds; remove/change a causal rule and observe both result and explanation respond. No post-hoc fabricated prose. |
| Sensitivity (§18,23) | Bounded explicit assumption/encounter reruns reveal ranking reversals and unstable near-ties without invented confidence percentages. | Hand crossover cases and real reruns with exact scenario bindings; unrelated fields cannot silently change. |

## Browser, privacy and delivery

| Domain | Required observable behavior | Independent acceptance evidence |
| --- | --- | --- |
| Account workspace | Aaru Terminal handles sample/import/confirmation/preferences/export/reset and honest empty/error/loading/session states. | Production optimized WASM, real files/downloads and interrupted workflows; source/canonical byte checks and preservation of prior state on failure. |
| Decision workspace | Party Setup, Alternate Paths and Satyavada expose actual engine output, explicit objectives, trade-offs and model evidence. | End-to-end native oracle versus rendered account-specific outcome. UI cannot invent a score or hardcode an owned roster to satisfy a test. |
| Responsive isolation | Thin DOM and two directly owned sibling workers; compact messages, cooperative bounded search and isolated terminable bulk phases; visible cancellation within 50 ms. | Real large 16 MiB input and representative results, genuine input timing, blocked-worker termination, stale queued-message rejection and independent worker/resource observations. |
| Durable state | Real transaction complete, revision CAS, non-cancellable reservation, idempotent activation, crash/uncertain-outcome recovery and honest corrupted-store clearing. | Actual IndexedDB abort-after-request-success, completion-before-ack, quota/denial, restart/reload, corruption and old-write races. Native FSM tests are not these observations. |
| Session continuity | Retained session-only inventory and acknowledged objective/target/UI settings survive pair cancellation/restart, including a different older saved record. Reload loses ephemeral state honestly. | Denied/unread IDB from startup, old saved versus new active accounts, interrupted edits/checkpoints and actual reload. No invented successful read or empty durable record. |
| Single primary tab | Web Lock gates every private read/action; secondary/no-lock cases public-help only; ownership succession settles old storage first. | Two actual tabs, primary close/crash during writing and successful ordered successor recovery; no variable-only leader-election test. |
| Local privacy and assets | No credentials/scanner automation/account upload/logging; dedicated trustworthy origin, explicit non-root base, allowlisted loopback assets, strict CSP/MIME and no traversal. | Actual network/CSP/server probes across private workflows, hostile strings/paths, same-origin disclosure and no repository-root serving. Database names are not origin isolation. |
| Resource retention | Bounded candidates, messages, caches, WASM arenas, opaque attachments, ports and download URLs; disposal and backpressure work under repeated cancellation. | Repeated representative/max-size cycles, application ledgers plus actual browser evidence. Do not claim total RSS or immediate GC from dropping a reference. |
| Novice/accessibility/theme | Plain-English actions/errors and term explanations, accessible keyboard/focus/live states, responsive original Aaru/Sumeru styling, no spoilers/unlicensed assets or implied HoYoverse integration. | Real browser keyboard/error/contrast/viewport checks and novice workflow review; important actions remain understandable without Genshin lore. |
| Native/browser platforms | Supported Windows and Linux builds and actual optimized production browser workflows. | Hosted native jobs plus real browser/storage/cancellation/performance cases on both OSes; Linux compatibility alone does not establish Windows product behavior. |
| Supply chain and protected delivery | Exact dependencies/tools/locks, immutable actions, hash-locked framework, meaningful advisory/license/source checks, StepSecurity first, least privilege, protected aggregate gate, no auto-merge/bypass. | Fresh final-source local and hosted executions, deliberate enforcement faults and actual repository settings. Review build/runtime/deployment tool trust separately; no zero-risk claim. |
| Release package and help | Reproducible deployable website/build artifacts, source/third-party notices, clear supported mechanics/platforms and player-focused usage/privacy/recovery help. | Clean release-artifact build and independent installed/deployed smoke, matching fingerprints/hashes, license/attribution review and resolved material QA findings. |

## Scope rules

- AI at the edges is optional; the full deterministic product works without an
  external model. Do not add a covert account-data upload or placeholder AI feature.
- Battle Chronicle/scanners are not required sources of full inventory. GOOD is a
  validated data boundary, not endorsement or execution of an external scanner.
- A frame-perfect simulator is optional. Higher-level modeling must still expose
  its actual assumptions, supported mechanics and localized uncertainty.
- Named current/future systems and competitor claims in the memo are research
  assertions, not authority to fabricate kits, data, licenses or lore.
- Unsupported data paths must be honest and tested. They do not justify declaring
  a synthetic-only demonstration a release-ready live-account platform.

Update this map with scoped evidence as real behavior is implemented. Every
material row needs an adequate owned mechanism, independent oracle, relevant
boundary and challenged failure sensor before release acceptance.
