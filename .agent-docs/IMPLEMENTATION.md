# Aaru Engine — implementation contract

## Product

A local-first, beginner-friendly Genshin account decision website. Rust is the
source of truth, compiled to WebAssembly; no backend or game credentials required.
Windows and Linux use the same browser UI. Authoritative intent is the full
`genshin_decision_engine_design_memo.md`. Read it for nuance, not just this note.

The user authorized initialization and remote creation. GitHub owner:
`scottdraper8`; repository name `aaru-engine`. Default to private until the owner
chooses publication. Never merge automated dependency updates.

## Durable requirements

1. PolyForm Strict 1.0.0, exact upstream license text and copyright notice.
2. Exact direct dependency/tool versions, Cargo/npm lockfiles, SHA-pinned actions
   and hooks. Locked installs; no unreviewed lifecycle scripts.
3. pre-commit with Rust formatting, strict Clippy, behavioral tests, web lint and
   repo-specific policy checks. No bypasses, suppressions, or skipped tests.
4. Harden every hosted CI job with StepSecurity; least privilege, bounded time,
   untrusted PR code isolated from secrets, Linux + Windows native validation,
   production WASM build and real-browser checks. Rust advisories and dependency
   policy enforced in CI. No unreasonable waiting periods.
5. Dependabot opens security-fix PRs with exact direct pins, never auto-merges.
   Security updates require repository alerts/security-update settings as well
   as configuration. Enforce required CI through branch protection if available.
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

## Evidence discipline

Follow `behavioral_testing_overhaul_agent_protocol.md` and
`Evidence_Governed_Agentic_Reverse_Engineering.pdf` (both read in full at intake).
Mechanism, observed execution, intent, and correctness are distinct. Use normative
hand-calculated cases, invariants/metamorphic checks, differential small-domain
search, real native/browser boundaries and targeted mutation/fault challenges.
Claims must name domain, oracle, reality/substitution, provenance and uncertainty.
Keep a compact current claim-evidence matrix, not a ledger of every action.

## Architecture and delivery

Rust engine library with strict typed state and data-driven mechanics; WASM
adapter exposes structured errors/results. Lightweight static website with a
worker for expensive computations; no server-side inventory storage. Synthetic
fixtures are loaded by the same import/evaluation path as user data. Keep public
mechanics independently versioned; unsupported facts remain visible.

## Current state / next action

Initial folder contained only the three supplied reference documents and an OS
`.directory` file. There was no Git repository, source, tests or baseline to run.
Rust 1.98.1 is installed under `$HOME/.cargo/bin` but that directory is not in the
shell PATH. Node 26.12.1, Python, pre-commit 4.6.2, uv and authenticated gh exist.

Scaffolding in progress. Establish checked-in security/tooling foundations and
launch the multi-phase build, then independently verify all claimed checks.
Release is blocked until verified mechanics coverage, full behavior/UI/security
checks and honest scope documentation exist. A synthetic-data demonstration is
not a validated live-account recommendation release.
