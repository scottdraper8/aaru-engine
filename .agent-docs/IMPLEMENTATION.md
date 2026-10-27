# Aaru Engine — implementation contract

## Product

A local-first, beginner-friendly Genshin account decision website. First-party
application code, browser UI, tests and repository tooling are written in Rust.
Use WebAssembly with static HTML/CSS and generated bindings, not a Python/Node
application or custom Python workflow. `pre-commit` is retained as explicitly
requested; its implementation language does not authorize Python project code.
No backend or game credentials required. Windows and Linux use the same browser
UI. Authoritative intent is the full
`genshin_decision_engine_design_memo.md`. Read it for nuance, not just this note.

The user authorized initialization, remote creation, and PUBLIC visibility.
GitHub repository: `https://github.com/scottdraper8/aaru-engine` (public).
Never automatically merge dependency updates.

The acceptance map in `REQUIREMENTS.md` tracks each original owner requirement
and later clarification. Keep it current. It is not permission to scope the final
product down to an illustrative demo. Intermediate slices are implementation
milestones only; the requested endpoint remains a release-ready product.
`RELEASE_GATES.md` maps memo capabilities to observable behavior and independent
acceptance evidence, including roster/investment value beyond equipment moves.

## Durable requirements

1. PolyForm Strict 1.0.0, exact upstream license text and copyright notice.
2. Exact dependency/tool versions, Cargo.lock, hash-locked external pre-commit
   framework requirements, SHA-pinned actions/hooks. Locked installs; no unreviewed
   lifecycle scripts. No Node project stack.
3. pre-commit with Rust formatting, strict Clippy, behavioral tests, web lint and
   repo-specific policy checks. No bypasses, suppressions, or skipped tests.
4. Harden every hosted CI job with StepSecurity; least privilege, bounded time,
   untrusted PR code isolated from secrets, Linux + Windows native validation,
   production WASM build and real-browser checks. Rust advisories and dependency
   policy enforced in CI. No unreasonable waiting periods.
5. Dependabot opens security-fix PRs with exact direct pins, never auto-merges.
   Security updates require repository alerts/security-update settings as well
   as configuration. Keep required CI enforced through public main protection.
6. Data before recommendations: extensive realistic synthetic account inventories
   including unequipped weapons/artifacts, levels, ascensions, constellations,
   talents, refinements, duplicates, locks, sparse/large/incomplete accounts.
   Fixtures are fabricated, not real user data or verified live-game mechanics.
7. Roster ownership comes from validated imports, never a hard-coded owned list.
   GOOD is the first interoperability boundary. Battle Chronicle is future work:
   it cannot supply the full unequipped inventory and authorization must be safe.
8. Separate public mechanics/provenance from private account state. Import sizes,
   shapes, numeric ranges, identity/references, unsupported data, and allocation
   conflicts must be handled explicitly. Unknown data cannot silently become zero.
9. Deterministic formulas/typed effect predicates, team-level interactions, energy,
   field time, reaction/aura, sustain, special-system predicates, explicit objectives
   and encounters; distinguish exact/estimated/qualitative/unknown evidence.
10. Hard constraints before ranking; score vectors, objective weights, Pareto
    frontier, non-overlapping multi-team selection, scarce item allocation,
    artifact search, donor-aware counterfactuals and dependency invalidation.
    Reproducibility includes account/mechanics/engine/objective/encounter versions.
11. AI optional at the edges only; no model needed to use the engine. No invented
    current/future kits. Research assertions in the memo need independent checking.
12. Plain-English explanations from reason codes, visible assumptions, no fake DPS
    or precision. Accessible responsive UI, progress/error/empty states, private
    local import/export/reset, repeatable comparisons and understandable tradeoffs.
13. Performance comes from measured representative workloads, deterministic pruning
    and recomputation, not unexplained heuristic pair scores or weak test fixtures.
14. No legacy code/shims or stale docs. Correctness/thoroughness over delivery speed.
15. Self-explanatory code; no comments unless absolutely necessary. Organizational
    headers are allowed. Apply retroactively to existing source/tests/tooling and
    remove redundant narration and documentation duplicated by authoritative docs.

The PDF is reference material only. No PDF-specific tooling in pre-commit or CI;
its text extraction is a one-time reading aid, never a project workflow.

## Evidence discipline

Follow `behavioral_testing_overhaul_agent_protocol.md` and
`Evidence_Governed_Agentic_Reverse_Engineering.pdf` (both read in full at intake).
During orchestration, the PDF's complete extracted text is available at ignored
`.agent-docs/tmp/evidence-methodology.txt` for agents without PDF-reading tools.
Mechanism, observed execution, intent, and correctness are distinct. Use normative
hand-calculated cases, invariants/metamorphic checks, differential small-domain
search, real native/browser boundaries and targeted mutation/fault challenges.
Claims must name domain, oracle, reality/substitution, provenance and uncertainty.
Keep a compact current claim-evidence matrix, not a ledger of every action.

## Player-facing identity

Follow `PRODUCT_LANGUAGE.md`: Genshin/Sumeru/Aaru-themed user-facing features and
tools, accurate quest terminology, novice-readable labels, original accessible
styling, no unverified lore claims or spoilers by default. Preserve Aaru Engine.

## Architecture and delivery

Rust engine library with strict typed state and data-driven mechanics; WASM
adapter exposes structured errors/results. Rust browser UI (e.g. web-sys or a
justified Rust UI framework), static HTML/CSS, generated JS glue only. Responsive
bounded/asynchronous computation; no server-side inventory storage. Rust tooling
owns builds/serving/security checks. Real browser tests also use Rust tooling,
not a second Python/Node stack. Synthetic fixtures use the same import/evaluation
path as user data. Public mechanics independently versioned; unsupported facts
remain visible. Workspace foundation and Rust `aaru-repo-policy` already exist.

Use a thin Rust DOM controller and two directly owned sibling module Workers:
private Rust UI supervisor and Rust engine. Large logical JSON, fingerprints and
validation stay off the DOM thread; compact controls/pages are at most 16 KiB.
The DOM owns Web Lock/IndexedDB settlement and opaque recovery/UI-state anchors.
Cancellable bulk work can terminate both workers immediately; durable reservations
remain non-cancellable. Keep 50 ms visible cancellation acceptance. Cooperative
search retains 256-unit/4 ms slices; isolated terminable codecs are not falsely
classified as short primitives. This design addresses observed >50 ms WASM calls;
its implementation, memory and platform behavior still require real-browser proof.

## Current state / next action

Worktree branch: `feat/rust-decision-engine`. Rust 1.98.1 is installed under
`$HOME/.cargo/bin`; prepend it to PATH for cargo/pre-commit. Toolchain, wasm32,
rustfmt and Clippy are pinned. External pre-commit framework is 4.6.2 with exact
hashed requirements in `.github/`; there is no first-party Python/Node source.

PUBLIC remote, main protection, vulnerability alerts, automated security fixes,
secret scanning/push protection verified via API; auto-merge disabled. Feature
updates must go through the protected PR, not a direct push to main.

Current handoff status: **G1 ACCEPTED AT THE NATIVE CONTRACT BOUNDARY**, commit
`1b664913a3d25f86e97b18b1069a442276e9b760`, run40582912311attempt1. Not production
or release acceptance. P1/H0 shared-schema/build/lease prerequisites remain before
engine/UI source launch; no automatic orchestrator resume.
- Orchestrator **450363 remains intentionally SIGSTOP-paused**. Phase 0/1 artifacts
  are under `tmp/big-build/aaru-engine`; read complete phase1 QA, not just headings.
- Independent review `tmp/g1-review.md` found five blockers despite passing tests.
  `tmp/g1-repairs.md` records their repairs, 34 passing contract tests and twelve
  isolated detected/reverted faults. Native FSM evidence is not browser proof.
- Real optimized-WASM probes observed allowed 16 MiB fingerprints at 56–66 ms and
  decoding at 77–92 ms. `tmp/g1-budget-evidence.md` records exact scope/provenance.
  The approved sibling-worker design is in `tmp/g1-bulk-architecture.md`, with
  binding integration amendments in `tmp/g1-bulk-acceptance.md`.
- Independent review `tmp/g1-independent-review-evidence.md` returned **G1 FIX
  REQUIRED** despite independently passing 51 contract/20 policy/4 Git tests twice.
  Six issues: DOM memory discard, account+UI-checkpoint transition ordering,
  actual storage-refresh installation, activation rebinding, memory-only revision
  exhaustion and matching traversal-shaped catalog paths. Four normative probes
  failed; two additional transition gaps have executed witnesses/source arguments.
  Parent additionally executed `tmp/g1-retention-finding.md`: saved A + session B
  + candidate C retains three distinct source bodies despite bound two.
- Composed repairs closed the original seven at native-contract scope; later
  dangling-subject and unreadable-current-header probes still failed. Final-seam
  writer finished both repairs: `tmp/g1-final-seam-repairs-evidence.md`, 71/20/4
  cases and18 restored behavioral faults. Parent matched source fingerprint
  `81bea9f7a9f453bfc380562e7a2cdaca5ce964c6a03cb513a8f9e61d54ac39b6`
  and reproduced the complete tests/strict native/contract-WASM checks. Independent
  final review finished15 probes, supporting all nine native seams; its whole-
  comparison16MiB interpretation was wrong, reinforced by parent's review prompt.
- Parent executed the compound-budget regression then corrected it after review:
  `tmp/g1-compound-budget-finding.md` and `tmp/g1-budget-scope-evidence.md`.
  Original profile is16MiB PER DECISION and128MiB TOTAL compound/final wire,
  including envelope overhead. No lower whole-comparison cap. Corrected source
  `bf5cb79a84ca4759369436e8e9c669a9241e73ea9eed24970b98ea3e5766cab9`
  passes73/20/4, native/WASM strict checks, all-files pre-commit and security checks.
  Three isolated full-suite budget faults fail assertions; exact restoration.
  Original failing probes stay unchanged; new adapted whole harness74passes.
  Narrow independent follow-up finished21 owned controls and73/20/4 twice, no
  remaining budget code defect. It found one stale contradictory contract sentence;
  parent corrected that documentation-only defect. Review ownership is released.
  Complete reviewed snapshot/staged/all-files/native/WASM/security checks and normal
  commit hooks passed. Parent repeated all21 probes. Foundation5b9c64f and reviewed
  hardening update1b66491 are pushed. Fresh40582912311 passed Linux native, Windows
  native, dependency security and Quality gate. Parent decision tmp/G1_ACCEPTANCE.md.
  Native maximum tests are not browser memory/paint/Worker proof.
- Full-product replacement plan ADOPTED under `tmp/big-build/aaru-engine/phase1/`:
  master-plan, architecture, delivery, acceptance and threads.json; read ALL parts.
  Parent assessment amendments applied: correct byte scopes/current APIs, H0 material/
  public schemas first, genuine initial source before full workspace/build validation,
  two disjoint leases, all16 requirements and19functional/12browser rows. Old plans
  archived as pre-replan, failed giant-write recovery unadopted. P1/H0 not launched.
- Parent owns policy/hygiene, hooks/workflows/framework locks and acceptance docs.
  Contract coordinator must not stage or commit. Parent stages the complete reviewed
  snapshot only after ownership settles; staged-blob checks must match that snapshot.
- Locked local Rust hook equivalents retain pre-commit without separately resolved
  generic-hook environments. Policy passed 20 unit + 4 real temporary-Git/CLI tests,
  strict Clippy, formatting and actual staged-repository inspection. This includes
  staged/worktree separation, raw adversarial cases, key detection, portability,
  exact 2 MiB/mode boundaries and mandatory Linux/Windows/advisory CI enforcement.
- Rust OSV checks cover the exact hash-locked external framework dependency graph,
  reject advisories regardless of severity and fail closed on errors. Live query,
  denied-warning cargo-audit and cargo-deny passed the checked snapshot. Rerun all
  on final integration; do not claim zero vulnerabilities or universal database coverage.
- Historical foundation `5b9c64f3c36e7146735cdff1f0c57767b3c2b018`,
  run40579865822: Windows native and dependency security succeeded; Linux checkout
  failed three times connecting to allowed github.com under blocked egress. Raw
  API/logs prove that failure even while run/job summaries still say in_progress.
  Do not treat delayed summary status or later green as erasing this failure.
- Official StepSecurityv2.22.3 released2026-10-27 explicitly addresses GitHub DNS/
  outbound-proxy false blocks. Parent verified tag46b06cea07f9e64621fdf066ba77ac6915163d43,
  source/checksums (Linux agent0.18.0), and updated all three immutable action pins;
  Linux block/Windows audit/privilege/endpoint policy unchanged. Fresh40582912311
  all required jobs succeeded; Linux log confirms block and the expected0.18.0 checksum.
  Stuck failed40579865822 was force-cancelled after normal cancellation failed, NOT
  relabeled green. Success is consistent with the vendor fix, not universal resolver
  proof. Historical72f2048 green and40493810875 failure remain scoped evidence.
- Tools: PATH includes `$HOME/.cargo/bin`; audit0.22.2, deny0.20.2 and
  wasm-bindgen0.2.129 under `/tmp/aaru-audit-tools/bin`. Sandbox Chrome/driver
  156.0.8078.47 paths/provenance are in `tmp/RUNTIME_PROBE.md`. Completed disposable
  topology probe: `tmp/g0-topology-probe-evidence.md`, immutable 72f2048 dependency,
  two real Linux Chrome runs and three detected runtime faults. Actual 16 MiB bytes,
  sibling target disappearance, IDB abort/complete/CAS and retained-Blob behavior
  observed. Local input-to-DOM cancellation 0.2–0.6 ms is NOT painted-pixel proof;
  long-run Blob/RSS/revocation/production/Windows behavior remains open. Parent
  independently verified no probe Chrome/driver processes or server ports remain.
- Public-source research finished: `tmp/live-source-research-evidence.md`; parent
  review `tmp/live-source-research-parent-review.md`. GOOD docs confirm optional
  material balances in v2/v3 (absent remains unknown), unboosted talents and
  percentage-point semantics; GOOD_REFERENCE.md now records documentary evidence.
  Current gcsim notice is AGPLv3-or-later since 2026-09-19, not wholly MIT; no current
  code reuse authorized. KQM/data/prose/artwork rights and actual numeric/cost tables
  remain unresolved. Directory counts include custom definitions, not live coverage.
  Parent flagged the proposed ICD timeline as insensitive to its stated fault;
  strengthen and execute future oracles, never inherit a green assertion blindly.
  Research is not mechanics implementation or waiver of substantial fixtures first.
- Unnecessary first-party source/test/config comments were removed. Necessary
  action-version metadata remains. Keep the rule in all subsequent implementations.
- Next P1/H0: parent coordinates canonical material knowledge/counts/provenance and
  public import/catalog/labels, exact dependency/manifest/entrypoint/build interfaces.
  Then allocated E/U GENUINE initial source, park both for parent-only integration,
  full all-target validation and real optimized inventory/IDB/settings/cancellation/
  export vertical on Windows/Linux. No fake libs/workers or missing-target masking.
  Extensive realistic corpus BEFORE recommendations; source-backed mechanics/search/
  allocation/donor/recompute, DISTINCT roster/investment and full release evidence.
  Protected PR/no auto-merge, complete staged/all-files/hooks and fresh CI each commit.

Artifacts in ignored `.agent-docs/tmp/` are temporary evidence/coordination, not
release deliverables or binding product requirements. Keep this note current and
remove obsolete process details. No release claim until actual product evidence.

Hosted enforcement contract:
- Remote main must require pull requests and the GitHub Actions check named
  `Quality gate` with strict up-to-date checks, including administrators. The CI
  workflow MUST provide this aggregate job and fail it on failed/skipped required
  jobs. No automated merging. Work on `feat/rust-decision-engine`, not remote main.
- StepSecurity's public GitHub-hosted tier is free. Harden every job as its first
  step. Windows supports audit mode only; Linux supports block mode. Verify actual
  behavior in hosted runs, not just action presence. Upstream reference:
  https://github.com/step-security/harden-runner . No paid subscriptions authorized.

Release is blocked until verified mechanics coverage, full behavior/UI/security
checks and honest scope documentation exist. A synthetic-data demonstration is
not a validated live-account recommendation release.
