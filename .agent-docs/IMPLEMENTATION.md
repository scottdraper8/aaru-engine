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

## Current state / next action

Worktree branch: `feat/rust-decision-engine`. Rust 1.98.1 is installed under
`$HOME/.cargo/bin`; prepend it to PATH for cargo/pre-commit. Toolchain, wasm32,
rustfmt and Clippy are pinned. External pre-commit framework is 4.6.2 with exact
hashed requirements in `.github/`; there is no first-party Python/Node source.

PUBLIC remote, main protection, vulnerability alerts, automated security fixes,
secret scanning/push protection verified via API; auto-merge disabled. Feature
updates must go through the protected PR, not a direct push to main.

Active coordination (remove this section when the handoff is complete):
- Orchestrator PID **450363 is intentionally SIGSTOP-paused**, not failed. Phase
  0/1 completed under `.agent-docs/tmp/big-build/aaru-engine`. Do NOT resume it
  until G0/G1 is accepted; otherwise planners/builders start without valid shared
  contracts. Read the complete Phase 1 `qa-review.md`, especially severe S1–S4.
- Initial contract writer finished owned commit `2e96a48`. Contract-owned paths
  are clean, but G1 is NOT accepted just because tests pass. Next run the focused
  follow-up at `.agent-docs/tmp/contract-corrections-prompt.md` after reading
  `.agent-docs/tmp/contract-review-required.md` and the complete architectural QA.
  Coordinator owns `crates/aaru-contract/**`, `CONTRACTS.md`, and genuinely needed
  root Cargo/lock changes only. No consumer engine/UI threads before acceptance.
- Parent owns pending `.github/workflows/ci.yml`, `repo-policy` enhancements,
  `deny.toml`, `.gitattributes`, README and this acceptance documentation. Do not
  accidentally include other owners' staged changes in a commit.
- On the completed combined tree, formatting, full locked workspace tests
  (16 contract + 10 policy), strict all-target Clippy, cargo-audit with denied
  warnings and cargo-deny all passed. Writer also checked contract wasm32
  compilation and observed two detected/reverted numeric and generation faults.
  These are scoped contract/policy results, not live-product correctness proof.
  Run pre-commit against staged parent CI/policy changes before pushing. No checks
  have been bypassed.
- `cargo-audit` 0.22.2 and `cargo-deny` 0.20.2 installed under
  `/tmp/aaru-audit-tools/bin`; current graph audited with denied warnings. Run
  both again after contract finalization, including the explicit license policy.
- Limited optimized Rust/WASM/module-worker/CSP compatibility was observed in
  actual Chrome + ChromeDriver 156.0.8078.47 with normal sandbox. Exact provenance,
  executable paths and artifact hashes are in `.agent-docs/tmp/RUNTIME_PROBE.md`.
  wasm-bindgen CLI/test runner 0.2.129 installed under the same tooling root.
  Probe server/driver were stopped. This is NOT production Aaru browser evidence.
- Security follow-up before final acceptance: exact hashed framework requirements
  do not by themselves lock the external generic hook environment's dependencies.
  Close that transitive-resolution gap without removing equivalent checks or the
  required pre-commit framework; include external framework advisory coverage.
- Next: accept/fix G1, run and commit all required gates, push and verify hardened
  Linux/Windows/advisory hosted CI, protected PR for foundations, then resume the
  two implementation threads against the accepted contracts. Both threads may
  own crate-local manifests and regenerate worktree Cargo.lock; coordinator
  resolves integration. Freeze complete interfaces instead of guessed stubs.

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
