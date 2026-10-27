# Acceptance requirements

The owner's original instructions and later clarifications are binding. Autonomy
is not permission to relax them. This is a current acceptance map, not a changelog.
Update evidence/status as implementation advances. “Configured” is not “verified
in execution.” No release until all material gates have supporting evidence.

| Owner requirement | Current evidence / remaining gate |
| --- | --- |
| 1. Initialize Git and create remote | Verified: local Git repository; public `scottdraper8/aaru-engine` on GitHub. Remote main requires PRs and the GitHub Actions `Quality gate`, including administrators. |
| 2. PolyForm Strict | `LICENSE.md` matches upstream PolyForm Strict 1.0.0 byte-for-byte; `NOTICE` records copyright. |
| 3. Novice-facing English docs and interface | README is player-focused; usable accessible website and usability verification remain open. |
| 4. Windows and Linux | Rust/WASM architecture; narrow optimized module-worker/CSP compatibility probe observed in actual sandboxed Chrome on Linux. Production browser behavior and hosted Windows/Linux execution remain open. |
| 5. Intelligent pre-commit; no bypass/suppressions | Hooks installed and executed; 10 Rust policy tests include aggregate-gate failures and hardening/credential safety. Full engine/web gates and hosted repetition remain open. No checks bypassed. |
| 6. Exact dependency pins | Exact Cargo/toolchain pins, Cargo.lock, SHA actions/hooks and hashed external framework requirements. Current dependency graph passes audit/license/source policy. External generic-hook transitive resolution and framework advisory coverage must also be closed before final acceptance. |
| 7. StepSecurity hardening | Per-job CI authored with Linux block / Windows audit and least privilege; actual hosted execution/protection remains unverified. |
| 8. Dependabot security PRs; exact fixes; no auto-merge | Alerts and security updates enabled; auto-merge disabled. Cargo/pip security-only and weekly GitHub Actions configuration exists. Actual pinned-fix behavior and hosted checks remain open. |
| 9. Enforceable reasonable security using dbinfo as reference | Reference reviewed; stronger pin/bypass/hardening/aggregate-gate policies tested. Main protection, secret scanning and push protection verified. Local cargo-audit (deny warnings) and cargo-deny pass; hosted enforcement and complete external-tool graph validation remain open. No waiting-period rules. |
| 10. Evidence-governed tests, not theater | Both testing references fully read. 16 contract tests include independent arithmetic oracle, provenance/resource/stale-event boundaries; two deliberate detected/reverted numeric and generation faults. Architectural QA still blocks contract acceptance. Product oracles, realistic boundaries, sensitivity/performance and complete claim-evidence matrix remain open. |
| 11. Freedom to reorganize | No existing source structure imposes a compatibility obligation. Prefer clean replacements over preservation of poor designs. |
| 12. Autonomous through release readiness; correctness first | Active multi-phase build/review. Scaffold/demo completion is NOT release acceptance. Verified game mechanics, performance, security and meaningful behavior are release gates. |
| 13. No old/deprecated/unused code | Enforce during code review and subsequent redesigns; do not add shims for pre-release states. |
| 14. Durable, lean context | `AGENTS.md`, `IMPLEMENTATION.md`, this acceptance map and original references. Temporary orchestration artifacts ignored; keep only useful current evidence. |
| 15. Entire design memo | All 1,968 lines read at intake. Deterministic account reasoning, causal counterfactuals, opportunity cost, explicit objectives/uncertainty and trust boundaries govern implementation. |
| 16. Extensive realistic synthetic data first | Open implementation gate: full inventory model plus substantial fixtures including unequipped weapons/artifacts, duplicates, progression and incomplete imports. Do not confuse fabricated accounts with verified mechanics or sufficient data from Battle Chronicle. |
| Clarification: Rust | All first-party application/UI/tests/repository tooling use Rust; static HTML/CSS and generated WASM bindings are allowed. Rust policy (10 tests), shared-contract tests (16), formatting and strict all-target Clippy executed successfully; contract handoff still requires architectural QA fixes. No Python/Node project stack. |
| Clarification: PUBLIC repository | Applied and verified via GitHub API. Do not revert to private or cite private-tier restrictions as blockers. |
| Clarification: PDF is reference only | No PDF extraction/parsing/generation/linting in hooks or CI. One-time text extraction is solely a reading aid. |
| Clarification: Genshin-themed user-facing systems | Follow `PRODUCT_LANGUAGE.md`; Sumeru/Aaru identity, correct game/quest terminology and novice-readable actions. Verify lore, avoid spoilers and unlicensed assets. |

## Current external configuration evidence

Repository API: public; `allow_auto_merge=false`; vulnerability alerts return 204;
automated security fixes enabled; secret scanning and push protection enabled.
Main protection API: `strict=true`, `Quality gate` bound to GitHub Actions app
15368, `enforce_admins=true`, PRs required, conversations resolved, force pushes
and deletions disabled. These settings do not establish that the CI jobs exist
or pass; that must be demonstrated separately.
