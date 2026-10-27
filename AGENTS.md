# Working rules

Read `.agent-docs/IMPLEMENTATION.md` before substantive work. Read the original
`.agent-docs/genshin_decision_engine_design_memo.md` in full for architecture or
scope decisions. The two testing references in that directory govern evidence.

## Non-negotiable

- Rust owns validation, mechanics, search, allocation, and counterfactuals. The
  beginner-friendly website runs the engine locally through WebAssembly.
- Never bypass checks: no `--no-verify`, `SKIP`, disabled hooks, alternate hook
  paths, inline lint/type/security suppressions, ignored tests, or weakened gates
  to obtain a green result. Fix the cause. If blocked, report it and do not commit.
- Run `pre-commit run --all-files` before every commit. CI repeats the gates;
  branch protections, not local hooks alone, are the enforcement boundary.
- Pin every direct dependency/tool to an exact version and every action/hook to
  an immutable commit. Commit lockfiles. Use locked installs. No auto-merging.
- Treat imports and generated AI/research output as untrusted. No credentials,
  game automation, privileged scanners, or external account uploads.
- Never claim synthetic mechanics are verified game facts or call a comparative
  score DPS. Unknown mechanics remain unknown, not guessed.
- Test behavior we own with independent oracles and real relevant boundaries.
  Record consequential claims, sensitivity experiments, and limitations concisely.
  Do not optimize test count, coverage percentages, mocks, or snapshots.
- Preserve deterministic tie-breaking, provenance, explicit objectives, hard
  constraints, score vectors, and causal reason codes throughout the engine.
- Resource moves must account for donor losses and unique item conflicts.
- No obsolete code, compatibility shims, abandoned approaches, or unused files.
  Replace pre-release designs cleanly. Keep documentation lean and current.
- User docs are plain English for nontechnical players, not contribution guides.
- License: PolyForm Strict 1.0.0, not PolyForm Noncommercial. Do not describe this
  as permissively licensed or OSI open source. Respect third-party attribution.
- Respect `.agent-deny`; never inspect secrets or dependency/generated output.

## Handoff

Maintain `.agent-docs/IMPLEMENTATION.md` as current decisions, release gates,
next actions, and unresolved constraints, not a chronological work log. Remove
stale notes. Never imply a gate passed without an actual recorded execution.
