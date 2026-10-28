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
| 4. Windows and Linux | Accepted G1 baseline1b66491/run40582912311 passed Linux/Windows native jobs. Sandboxed Linux Chrome research primitives observed, not production UI. Real optimized application/browser/storage/cancellation on both platforms remains open. |
| 5. Intelligent pre-commit; no bypass/suppressions | Required pre-commit retained with equivalent locked Rust checks. Current policy: 20 unit + 4 real Git/CLI tests pass, including staged blobs and mandatory cross-platform/advisory gates. Latest combined engine/web/hosted gates remain open. No bypasses. |
| 6. Exact dependency pins | Exact Cargo/toolchain pins, Cargo.lock, SHA actions and hash-locked external framework. Independently resolved generic hooks removed only after Rust equivalents passed. Audit/license/source checks and live fail-closed OSV check of the locked framework graph passed the checked snapshot; rerun on final integration. |
| 7. StepSecurity hardening | First-step Linux block / Windows audit and least privilege preserved. Foundation5b9c64f/run40579865822 passed Windows/security; Linux checkout failed connecting to allowed github.com. Verified immutable v2.22.3 update/run40582912311 passed ALL required jobs at1b66491, Linux block/Windows audit preserved. Successful operation is not universal root-cause proof. Historical72f2048 green and40493810875 failure remain scoped history. |
| 8. Dependabot security PRs; exact fixes; no auto-merge | Alerts and security updates enabled; auto-merge disabled. Cargo/pip security-only and weekly GitHub Actions configuration exists. Actual pinned-fix behavior and hosted checks remain open. |
| 9. Enforceable reasonable security using dbinfo as reference | Reference reviewed; enforced pins, no bypass/ignored failures, hardened aggregate gate and real staged-boundary checks. Main protection/scanning verified; denied-warning Rust audits and framework query passed locally and commit72f2048 passed hosted security/Quality. Final integration/release evidence remains open; no waiting periods or zero-vulnerability guarantee. |
| 10. Evidence-governed tests, not theater | Both references govern work. Native G1 accepted after independent lifecycle/reference/budget probes, corrected parent oracle and exact hosted gates; historical failures preserved. H0 independent review then found discarded foreign claims despite82/20/4 green. Repaired shapes now pass87/20/4, unchanged18+2 probes and4 new independent controls, with3 restored assertion-detecting faults; strict native/contract-WASM passes. H0 complete staged/hosted acceptance pending. Counts alone prove neither universal strictness nor product readiness. Full engine/UI oracles, actual browser memory/storage/paint/cancellation/Windows and release evidence remain open. |
| 11. Freedom to reorganize | No existing source structure imposes a compatibility obligation. Prefer clean replacements over preservation of poor designs. |
| 12. Autonomous through release readiness; correctness first | Active multi-phase build/review. Scaffold/demo completion is NOT release acceptance. Verified game mechanics, performance, security and meaningful behavior are release gates. |
| 13. No old/deprecated/unused code | Enforce during code review and subsequent redesigns; do not add shims for pre-release states. |
| 14. Durable, lean context | `AGENTS.md`, `IMPLEMENTATION.md`, this acceptance map and original references. Temporary orchestration artifacts ignored; keep only useful current evidence. |
| 15. Entire design memo | All 1,968 lines read at intake and reread for the product map in `RELEASE_GATES.md`. That map includes team/build/global search, marginal roster/investment value, dependency propagation, sensitivity, actual browser state and release delivery; shared-contract or synthetic-only success cannot close those gates. |
| 16. Extensive realistic synthetic data first | Open implementation gate: full inventory model plus substantial fixtures including unequipped weapons/artifacts, duplicates, progression and incomplete imports. Do not confuse fabricated accounts with verified mechanics or sufficient data from Battle Chronicle. |
| Clarification: Rust | All first-party application/UI/tests/tooling use Rust; static HTML/CSS and generated initialization-only WASM glue allowed. Native strict checks and contract wasm32 compilation/lint passed the checked snapshot. Native shared transport G1 accepted; actual engine/UI adapters remain to implement. No Python/Node project stack. |
| Clarification: PUBLIC repository | Applied and verified via GitHub API. Do not revert to private or cite private-tier restrictions as blockers. |
| Clarification: PDF is reference only | No PDF extraction/parsing/generation/linting in hooks or CI. One-time text extraction is solely a reading aid. |
| Clarification: Genshin-themed user-facing systems | Follow `PRODUCT_LANGUAGE.md`; Sumeru/Aaru identity, correct game/quest terminology and novice-readable actions. Verify lore, avoid spoilers and unlicensed assets. |
| Clarification: minimal code comments, retroactive | Persisted in AGENTS.md. Unnecessary first-party source/test/config comments removed; independent contract review found no remaining comment blocker. Keep only organizational headers or absolutely necessary comments, including required tooling metadata, and apply to every new implementation. |

## Current external configuration evidence

Repository API: public; `allow_auto_merge=false`; vulnerability alerts return 204;
automated security fixes enabled; secret scanning and push protection enabled.
Main protection API: `strict=true`, `Quality gate` bound to GitHub Actions app
15368, `enforce_admins=true`, PRs required, conversations resolved, force pushes
and deletions disabled. These settings do not establish that the CI jobs exist
or pass; that must be demonstrated separately.
