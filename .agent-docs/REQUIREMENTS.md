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
| 4. Windows and Linux | Rust/WASM browser architecture selected; real hosted Windows/Linux and browser executions remain open. |
| 5. Intelligent pre-commit; no bypass/suppressions | Hooks installed and executed for existing files. Source-specific hooks configured, initial policy fault tests pass. Full engine/web gates, policy hardening and CI repetition remain open. No hooks have been bypassed. |
| 6. Exact dependency pins | Policy rejects Cargo/npm ranges and action/hook tags. Hook SHA pinned. Exact Rust toolchain file exists; code dependencies and lockfile verification remain open. |
| 7. StepSecurity hardening | Public hosted runner eligibility established; per-job implementation and actual hosted execution remain open. Linux block / Windows audit support must be respected. |
| 8. Dependabot security PRs; exact fixes; no auto-merge | GitHub alerts and automated security fixes enabled; repository auto-merge disabled. Ecosystem configuration, pinned-fix behavior and CI validation remain open. |
| 9. Enforceable reasonable security using dbinfo as reference | Read dbinfo hooks, Dependabot and deploy workflow. Added stronger SHA/pin/bypass policies. Main protection verified via API; secret scanning and push protection enabled. Full CI/advisory/dependency gates remain open. No waiting-period rules. |
| 10. Evidence-governed tests, not theater | Both provided testing references read fully (PDF converted to text). Initial policy tests exercise rejected faults and accepted controls. Product claim/evidence matrix, independent oracles, real-boundary execution and sensitivity challenges remain open. |
| 11. Freedom to reorganize | No existing source structure imposes a compatibility obligation. Prefer clean replacements over preservation of poor designs. |
| 12. Autonomous through release readiness; correctness first | Active multi-phase build/review. Scaffold/demo completion is NOT release acceptance. Verified game mechanics, performance, security and meaningful behavior are release gates. |
| 13. No old/deprecated/unused code | Enforce during code review and subsequent redesigns; do not add shims for pre-release states. |
| 14. Durable, lean context | `AGENTS.md`, `IMPLEMENTATION.md`, this acceptance map and original references. Temporary orchestration artifacts ignored; keep only useful current evidence. |
| 15. Entire design memo | All 1,968 lines read at intake. Deterministic account reasoning, causal counterfactuals, opportunity cost, explicit objectives/uncertainty and trust boundaries govern implementation. |
| 16. Extensive realistic synthetic data first | Open implementation gate: full inventory model plus substantial fixtures including unequipped weapons/artifacts, duplicates, progression and incomplete imports. Do not confuse fabricated accounts with verified mechanics or sufficient data from Battle Chronicle. |
| Clarification: Rust | All first-party application/UI/tests/repository tooling use Rust; static HTML/CSS and generated WASM bindings are allowed. Rust repo policy, six behavioral fault-test families, formatting and strict Clippy have executed successfully. No Python/Node project stack. |
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
