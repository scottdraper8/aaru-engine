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

Current handoff status: **G1 NOT ACCEPTED**. Do not start engine/UI consumers.
- Orchestrator **450363 remains intentionally SIGSTOP-paused**. Phase 0/1 artifacts
  are under `tmp/big-build/aaru-engine`; read complete phase1 QA, not just headings.
- Independent review `tmp/g1-review.md` found five blockers despite passing tests.
  `tmp/g1-repairs.md` records their repairs, 34 passing contract tests and twelve
  isolated detected/reverted faults. Native FSM evidence is not browser proof.
- Real optimized-WASM probes observed allowed 16 MiB fingerprints at 56–66 ms and
  decoding at 77–92 ms. `tmp/g1-budget-evidence.md` records exact scope/provenance.
  The approved sibling-worker design is in `tmp/g1-bulk-architecture.md`, with
  binding integration amendments in `tmp/g1-bulk-acceptance.md`.
- Contract integration uses `tmp/g1-transport-integration-prompt.md`; output is
  `tmp/g1-transport-integration.md{,.json}`. Owns contract crate/CONTRACTS.md and
  genuinely needed Cargo/lock integration only; do not overwrite its work. It
  must freeze physical transport, safe reservations/bootstrap, retained session
  and UI-state checkpoints, public setup metadata and honest current-slot knowledge.
  Omitted inventory cannot become known-empty current gear.
- Parent owns policy/hygiene, hooks/workflows/framework locks and acceptance docs.
  Parent files are deliberately staged so checks inspect matching configuration;
  contract coordinator must not stage/commit them. Parent integrates after review.
- Locked local Rust hook equivalents retain pre-commit without separately resolved
  generic-hook environments. Policy passed 20 unit + 4 real temporary-Git/CLI tests,
  strict Clippy, formatting and actual staged-repository inspection. This includes
  staged/worktree separation, raw adversarial cases, key detection, portability,
  exact 2 MiB/mode boundaries and mandatory Linux/Windows/advisory CI enforcement.
- Rust OSV checks cover the exact hash-locked external framework dependency graph,
  reject advisories regardless of severity and fail closed on errors. Live query,
  denied-warning cargo-audit and cargo-deny passed the checked snapshot. Rerun all
  on final integration; do not claim zero vulnerabilities or universal database coverage.
- Hosted baseline `fd40cb8`, run40493810875 attempt2 passed Linux native, Windows
  native, dependency security and Quality gate. Attempt1 had real hardening/DNS
  connectivity failures and was cancelled. Unchanged rerun success does not prove
  the resolver issue fixed. Current uncommitted changes need fresh hosted evidence.
- Tools: PATH includes `$HOME/.cargo/bin`; audit0.22.2, deny0.20.2 and
  wasm-bindgen0.2.129 under `/tmp/aaru-audit-tools/bin`. Sandbox Chrome/driver
  156.0.8078.47 paths/provenance are in `tmp/RUNTIME_PROBE.md`. Probe sessions and
  server/driver processes are stopped. No production browser claim is established.
- Unnecessary first-party source/test/config comments were removed. Necessary
  action-version metadata remains. Keep the rule in all subsequent implementations.
- Next: independently review stable integrated G1; run full locked native/WASM,
  pre-commit and security gates; stage/commit only reviewed files, push and verify
  actual hosted outcomes. Use a protected foundations PR, then resume two builders
  against the accepted source. Reconcile their lockfiles and update the master plan
  to the accepted topology. Require the real-browser vertical transport/storage/
  cancellation gate, extensive fixtures, verified mechanics and full release evidence.

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
