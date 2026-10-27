# Behavioral Testing Overhaul Protocol for Autonomous Coding Agents

**Status:** Prototype methodology for empirical evaluation and practical use  
**Scope:** Language-, framework-, and test-runner-agnostic guidance for auditing and overhauling an existing software test suite  
**Primary objective:** Build a defensible, reproducible body of behavioral evidence about an unfamiliar codebase, then improve its test system so that consequential behavioral claims are supported by evidence proportionate to their scope.

---

## 0. Protocol intent

This protocol is for an AI coding agent entering an unfamiliar repository with no prior assumptions about:

- programming language;
- framework;
- architecture;
- test runner;
- build system;
- repository size;
- deployment model;
- runtime environment;
- existing test quality;
- external dependencies;
- documentation quality;
- domain;
- whether the software is a library, service, CLI, UI application, embedded component, distributed system, data pipeline, or another form of software.

The task is **not** to perform a conventional code review and **not** merely to increase test count or code coverage.

The task is to:

1. determine what behavioral evidence already exists;
2. determine what claims the existing tests can and cannot legitimately support;
3. identify consequential behavioral surfaces and uncertainty;
4. challenge the existing measurement apparatus;
5. generate additional evidence where warranted;
6. improve or replace tests where doing so strengthens behavioral evidence;
7. preserve observations separately from judgments;
8. state unresolved uncertainty explicitly;
9. leave the repository with a more trustworthy, maintainable, and behaviorally meaningful test system.

This protocol treats testing as one evidence-producing mechanism among several. Tests are valuable because they can make observations repeatable, but a test is not automatically a specification, proof, or statement of correctness.

---

# 1. Core epistemic model

Throughout the investigation, keep the following concepts distinct.

## 1.1 Mechanism

**Mechanism** is what the implementation is capable of doing or how an effect may be produced.

Examples:

- control flow;
- data flow;
- call relationships;
- state transitions;
- dependency bindings;
- serialization logic;
- configuration reads;
- database writes;
- network calls;
- concurrency primitives;
- retry logic.

Source inspection, static analysis, dependency graphs, types, and code search are often strong sources of mechanistic evidence.

Mechanism does **not** by itself establish that a path actually executes under a relevant condition.

## 1.2 Actuality

**Actuality** is what happened in a specific execution.

An actuality claim must be bounded by relevant conditions such as:

- program version;
- input;
- pre-state;
- configuration;
- dependency state;
- environment;
- runtime version;
- schedule or timing;
- external service state;
- identity or permissions;
- generated seed;
- feature flags.

Example:

> Under commit `abc123`, using configuration `C`, input `I`, database version `D`, and pre-state `S`, calling entry point `E` produced result `O`.

A runtime observation is strong evidence for that execution. It is not automatically evidence for all possible executions.

## 1.3 Intent

**Intent** is what the system is expected to do according to some external or historical source.

Potential intent sources include:

- specifications;
- API contracts;
- schemas;
- user documentation;
- issue history;
- changelogs;
- compatibility commitments;
- business rules;
- comments;
- migration documentation;
- incident reports;
- domain experts;
- established external standards.

Intent sources may be stale, incomplete, contradictory, or wrong.

## 1.4 Correctness

**Correctness** is whether behavior satisfies an applicable requirement, invariant, contract, safety property, compatibility obligation, or domain judgment.

Do not infer correctness solely from:

- current implementation behavior;
- existing tests;
- passing tests;
- snapshots;
- mocks;
- code coverage;
- developer comments;
- high execution volume.

Correctness requires an oracle, proof obligation, relation, specification, or justified domain judgment.

---

# 2. Non-negotiable operating principles

## 2.1 Evidence proportionality

Never make a conclusion broader than the evidence supporting it.

If an external service was mocked, do not claim the real service was validated.

If only one configuration was exercised, do not claim all configurations behave equivalently.

If one input class was tested, do not claim all valid inputs behave correctly.

If only staging was exercised, do not silently generalize to production conditions that staging does not reproduce.

## 2.2 Observation is not verdict

Record what happened before deciding what it means.

Example:

**Observation:** Two writers produce different decoded values from the same logical input.

Possible verdicts:

- defect;
- intentional format difference;
- compatibility behavior;
- unresolved;
- risk;
- expected behavior.

Do not rewrite the observation when the verdict changes.

## 2.3 Existing behavior is not automatically correct behavior

Characterization tests may intentionally preserve current behavior to detect changes.

They do not automatically prove the behavior is desirable.

A characterization snapshot may faithfully preserve a bug.

## 2.4 Existing tests are evidence instruments, not trusted authorities

Treat the current suite as an object of investigation.

A passing test may be:

- strong;
- weak;
- tautological;
- over-mocked;
- implementation-coupled;
- flaky;
- redundant;
- stale;
- insufficiently asserted;
- correctly scoped;
- incorrectly scoped.

Do not assume test count or historical longevity implies quality.

## 2.5 Coverage is reach, not correctness

Coverage can reveal which code was exercised.

Coverage does not establish:

- meaningful assertions;
- oracle quality;
- semantic correctness;
- environmental fidelity;
- fault sensitivity.

Use coverage as an index into uncertainty, not as a certificate.

## 2.6 Substitution creates an evidence boundary

Mocks, fakes, simulators, local substitutes, containers, staging systems, intercepted side effects, and record/replay systems are not inherently good or bad.

For each substitution ask:

> What real semantics have been removed from this experiment, and are those semantics material to the claim?

Use the cheapest controllable environment that still contains the semantics material to the proposition being tested.

## 2.7 Unknown is a valid result

When evidence is insufficient, report `UNKNOWN`.

Do not convert ambiguity into fluent explanation.

Do not invent design intent.

Do not infer undocumented guarantees without evidence.

## 2.8 Generated tests must not self-confirm implementation assumptions

When constructing expected results, avoid deriving the expected value from the same implementation logic being tested.

Seek independent or relational oracles where possible.

Examples:

- specification-derived expected value;
- round-trip property;
- independent implementation;
- previous compatible version;
- authoritative reference implementation;
- invariant;
- metamorphic relation;
- real dependency behavior;
- domain constraint.

## 2.9 Reproducibility matters

For expensive or consequential findings, preserve enough information for another investigator to reproduce them.

At minimum record when applicable:

- code version;
- command;
- inputs;
- configuration;
- environment;
- dependency versions;
- seed;
- pre-state;
- result;
- logs or artifacts;
- relevant test/probe path.

## 2.10 The suite should be behaviorally useful, not merely large

Do not optimize for:

- number of tests;
- number of assertions;
- raw line coverage;
- number of generated cases;
- mutation score alone;
- snapshot volume.

Optimize for meaningful, reproducible evidence about consequential behavior.

---

# 3. Required investigation states

## 3.1 Claim status

A claim may be:

- `SUPPORTED` — evidence supports the proposition at the stated scope.
- `PARTIALLY_SUPPORTED` — evidence supports only part of the proposition or only a limited domain.
- `UNSUPPORTED` — available evidence does not justify the proposition.
- `CONTRADICTED` — evidence directly conflicts with the proposition.
- `UNKNOWN` — evidence is insufficient to decide.
- `NOT_APPLICABLE` — the proposition does not apply to the repository or component.

Do not use `SUPPORTED` when the real status is “the agent believes this is probably true.”

## 3.2 Observation status

An observation may be:

- `REPRODUCED`;
- `OBSERVED_ONCE`;
- `FLAKY_OR_NONDETERMINISTIC`;
- `INFERRED_NOT_EXECUTED`;
- `HISTORICAL_ONLY`;
- `UNREPRODUCIBLE`.

## 3.3 Test disposition

An existing test or test family may be categorized as:

- `RETAIN`;
- `STRENGTHEN`;
- `REPLACE`;
- `CONSOLIDATE`;
- `RECLASSIFY`;
- `QUARANTINE`;
- `DELETE_WITH_JUSTIFICATION`;
- `INVESTIGATE_FURTHER`.

Never delete tests only because they look repetitive.

---

# 4. Evidence record

For every important observation or experiment, capture an evidence record.

```yaml
evidence_id:
type:
source:
repository_revision:
target:
proposition_or_question:
input_or_case:
pre_state:
configuration:
environment:
real_components:
substituted_components:
execution_conditions:
command_or_probe:
oracle_or_relation:
observation:
artifacts:
sensitivity_evidence:
reproducibility:
limitations:
timestamp:
notes:
```

Fields may be omitted when not applicable, but omissions that materially limit the conclusion must be stated.

---

# 5. Claim record

For every material behavioral conclusion, create a claim record.

```yaml
claim_id:
proposition:
scope:
status:
supporting_evidence:
contradicting_evidence:
exercised_domain:
included_reality:
excluded_reality:
oracle:
sensitivity:
provenance:
remaining_uncertainty:
verdict_if_any:
```

This operationalizes the conceptual dimensions:

- **P** — proposition;
- **D** — exercised domain;
- **R** — included reality;
- **O** — oracle or anomaly criterion;
- **S** — sensitivity evidence;
- **V** — provenance.

A claim is not complete merely because it has a proposition and a passing test.

---

# 6. Phase 0 — Safety, preservation, and workspace preparation

Before modifying the repository:

1. determine repository state;
2. identify uncommitted changes;
3. avoid overwriting user work;
4. determine whether destructive operations are safe;
5. establish how tests/builds can be run;
6. identify secrets, credentials, production endpoints, or destructive integrations;
7. prevent accidental production writes unless explicitly authorized;
8. preserve the initial repository revision;
9. record the environment available to the investigation.

Do not “clean up” unexpected state merely to make testing easier.

Unexpected state may itself be evidence.

---

# 7. Phase 1 — Repository reconnaissance

Start with inventory, not judgment.

## 7.1 Discover repository structure

Identify where possible:

- programming languages;
- frameworks;
- package/build systems;
- executable entry points;
- libraries/modules/packages;
- test directories;
- fixtures;
- configuration;
- deployment manifests;
- schemas;
- migrations;
- generated code;
- scripts;
- CI workflows;
- observability/tracing configuration;
- external service clients;
- persistence mechanisms;
- messaging systems;
- scheduled/background work;
- feature flags;
- environment-variable usage;
- authentication/authorization boundaries.

Do not assume conventional naming.

## 7.2 Discover existing test infrastructure

Identify:

- test runners;
- unit/integration/system/end-to-end distinctions if present;
- custom harnesses;
- fixtures;
- mocks;
- fakes;
- stubs;
- snapshot tests;
- golden files;
- contract tests;
- property-based tests;
- fuzzers;
- mutation tooling;
- load/performance suites;
- browser tests;
- database tests;
- service virtualization;
- test containers;
- staging test scripts;
- CI-only tests;
- ignored or disabled tests.

Do not assume the repository's labels are semantically accurate.

A test named `integration` may still mock every meaningful external component.

## 7.3 Discover intent sources

Locate potentially authoritative material:

- README;
- design docs;
- ADRs;
- API docs;
- schemas;
- formal contracts;
- examples;
- changelog;
- issue references;
- migration docs;
- compatibility notes;
- comments;
- release notes;
- historical tests;
- commit history.

Classify these as evidence of intent, not behavior.

---

# 8. Phase 2 — Establish the unmodified baseline

Run the repository as found before changing tests or production code, if safely possible.

Record:

- build success/failure;
- test commands;
- test totals;
- passes;
- failures;
- skips;
- disabled tests;
- flaky behavior;
- runtime;
- retries;
- warnings;
- environment requirements;
- unavailable dependencies;
- coverage if already supported;
- mutation results if already supported;
- external systems actually contacted;
- whether test isolation assumptions hold.

If tests cannot run, investigate why before attempting an overhaul.

The inability to establish a baseline is itself a material finding.

Do not silently fix baseline failures and then report the repository as initially healthy.

---

# 9. Phase 3 — Inventory the current measurement apparatus

Construct a test/evidence inventory.

For each major test family, determine:

- what component or behavior it exercises;
- execution scope;
- dependencies included;
- dependencies substituted;
- primary assertions;
- oracle source;
- fixtures/pre-state;
- configuration;
- expected side effects;
- failure behavior;
- whether the test is deterministic;
- whether it contributes evidence distinct from other tests.

The goal is to answer:

> What does this test suite actually observe?

not:

> How many tests exist?

---

# 10. Phase 4 — Identify behavioral surfaces

Derive the repository's consequential behavioral surfaces from the system itself.

Do not use a fixed universal checklist as a substitute for repository-specific analysis.

Potential surfaces include, where relevant:

- parsing;
- serialization;
- persistence;
- state transitions;
- API behavior;
- CLI behavior;
- authentication;
- authorization;
- validation;
- concurrency;
- caching;
- retries;
- timeout handling;
- failure recovery;
- transactions;
- ordering;
- idempotence;
- consistency;
- migrations;
- backward compatibility;
- configuration;
- feature flags;
- scheduling;
- time;
- randomness;
- localization;
- numeric precision;
- permissions;
- external provider semantics;
- filesystem behavior;
- browser behavior;
- UI state;
- message delivery;
- eventual consistency;
- schema evolution;
- resource cleanup;
- partial failures;
- error translation;
- logging or audit behavior where contractually relevant.

For each surface ask:

1. Why does it matter?
2. Which entry points expose it?
3. Which state or dependencies affect it?
4. What evidence currently exists?
5. What remains unknown?

Prioritize behavior, not files.

---

# 11. Phase 5 — Build the initial claim-evidence matrix

Create a matrix covering consequential propositions.

Example:

| Claim | Existing evidence | Exercised domain | Real components | Substitutions | Oracle | Sensitivity | Status | Gap |
|---|---|---|---|---|---|---|---|---|

Do not fill the matrix with vague claims such as:

> Authentication works.

Prefer scoped propositions such as:

> An authenticated user with role `editor` can update resource X through entry point Y when the record is in state Z.

The matrix is not expected to be exhaustive.

It should concentrate on material behavioral claims and uncertainty.

---

# 12. Phase 6 — Audit existing tests as evidence

## 12.1 Assertion adequacy

Ask:

- Does the test observe the behavior that matters?
- Does it only assert that execution completed?
- Does it assert only status codes while ignoring state or content?
- Does it check the final observable result?
- Does it inspect irrelevant implementation detail?
- Could the behavior be materially wrong while the test still passes?

## 12.2 Oracle quality

Ask:

- Where does the expected result come from?
- Is it independent of the implementation under test?
- Is it derived from a specification?
- Is it based on historical behavior?
- Is it a snapshot?
- Is it relational?
- Is it merely whatever the system produced last time?

Classify the oracle's epistemic strength.

## 12.3 Mock and substitution audit

For each significant substitution:

- identify the real component being replaced;
- identify the semantics omitted;
- identify the claim the test is being used to support;
- decide whether omitted semantics are material to that claim.

Do not label all mocks bad.

A deterministic fake clock may increase test quality for a pure time rule.

A mocked database cannot establish real transaction or collation behavior.

## 12.4 Implementation coupling

Determine whether a test:

- asserts internal call order unnecessarily;
- duplicates implementation logic;
- relies on private methods as the primary contract;
- breaks on harmless refactors;
- verifies interactions when the externally relevant behavior is an outcome.

Implementation-level tests may still be justified when the interaction itself is contractual.

## 12.5 Redundancy

Do not define redundancy by textual similarity.

Two tests may look similar yet provide distinct evidence because they differ in:

- environment;
- entry point;
- contract;
- configuration;
- dependency;
- fault model;
- diagnostic value.

Consolidate only when evidentiary contribution is genuinely duplicated.

## 12.6 Brittleness and snapshot risk

For snapshots or golden files ask:

- What semantic behavior is being preserved?
- Is the snapshot reviewable?
- Can a large update conceal meaningful changes?
- Is approval of new baselines deliberate?
- Is the snapshot preserving accidental implementation detail?
- Does it encode a known bug or compatibility requirement?

---

# 13. Phase 7 — Join structural and behavioral evidence

Use source/static structure to explain observations and guide further investigation.

Potential structural artifacts include:

- call relationships;
- control-flow information;
- data-flow information;
- dependency graphs;
- schema relationships;
- configuration-use maps;
- inheritance/type relationships;
- event/message flows;
- entry-point maps.

Do not treat structural graphs as behavioral truth.

Use them to answer:

- What mechanism could produce this result?
- What other entry points reach this logic?
- Which callers may inherit this behavior?
- Which configuration controls this branch?
- Which dependency influences this outcome?

Where practical, prefer mechanically reproducible structural artifacts over hand-maintained prose maps.

---

# 14. Phase 8 — Select evidence-generation methods

Choose methods based on the behavioral question, not fashion.

No method is mandatory for every repository.

## 14.1 Characterization testing

Use when:

- current behavior must be preserved before change;
- behavior is poorly documented;
- a regression sensor is needed;
- legacy behavior is consequential.

Procedure:

1. exercise current behavior;
2. record the actual result;
3. make the test explicitly about continuity;
4. avoid calling the behavior correct unless an oracle establishes correctness;
5. preserve relevant context.

## 14.2 Property-based testing

Use when meaningful domain-level properties can be expressed.

Examples:

- round-trip preservation;
- idempotence;
- commutativity where applicable;
- monotonicity;
- invariant preservation;
- representation invariance.

Evaluate both property quality and generator quality.

High test volume does not compensate for a weak property or an impoverished generator.

## 14.3 Differential testing

Use when two or more paths are expected to be meaningfully comparable.

Examples:

- old vs. new implementation;
- two parsers;
- two serializers;
- two storage backends;
- two API versions;
- reference vs. optimized implementation.

A disagreement is an anomaly signal.

It does not automatically identify which side is correct.

Validate comparability before treating disagreement as meaningful.

## 14.4 Metamorphic testing

Use when an exact expected output is difficult to specify but a relation among executions should hold.

Examples:

- permutation invariance;
- encode/decode preservation;
- predictable scaling;
- reversible transformation;
- representation changes that should preserve semantics.

A metamorphic relation is itself an oracle and must be justified.

## 14.5 Combinatorial interaction testing

Use when behavior depends on many discrete configuration dimensions.

Procedure:

1. model relevant parameters;
2. model meaningful values;
3. select interaction strength appropriate to risk;
4. generate a covering set;
5. record omitted dimensions.

Do not claim completeness over dimensions that were never modeled.

## 14.6 Fuzzing

Use when:

- the target accepts structured or semi-structured input;
- automatic failure signals exist;
- crashes, hangs, sanitizer violations, assertions, protocol errors, or differential mismatches are meaningful.

Recognize the limitation:

A fuzzer cannot notice semantic wrongness unless an oracle exposes it.

## 14.7 Symbolic or concolic execution

Use when:

- appropriate tooling exists;
- the target can be modeled;
- path exploration is valuable;
- generated path-reaching inputs may expose untested behavior.

Treat symbolic execution as an exploration mechanism, not as an automatic correctness oracle.

Record modeling limitations and unsupported environmental behavior.

## 14.8 Mutation testing

Use to ask:

> Would the current suite notice plausible wrongness?

Interpret results diagnostically.

A killed mutant indicates sensitivity to that perturbation.

A surviving mutant may indicate:

- equivalent behavior;
- missing execution;
- weak assertion;
- behavior outside current claims;
- inadequate observation.

Do not reduce test quality to a mutation-score target.

## 14.9 Fault injection

Use where behavior under failures matters.

Potential faults:

- timeout;
- permission denial;
- malformed dependency response;
- unavailable service;
- partial write;
- dropped connection;
- retryable failure;
- resource exhaustion where safely testable;
- corrupted input;
- clock discontinuity.

Select faults relevant to actual claims.

## 14.10 Static analysis

Use for propositions concerning:

- reachability;
- types;
- data flow;
- dependency relationships;
- taint flow;
- possible states;
- prohibited patterns;
- sound properties under a defined abstraction.

Do not translate a static warning into an observed runtime failure.

## 14.11 Formal methods

If formal specifications, model checking, proof artifacts, verified components, or strong type-level guarantees exist, treat them as first-class evidence.

Testing does not inherently supersede formal evidence.

Record the specification, assumptions, abstraction, and trusted base.

## 14.12 Historical and documentary evidence

Use history to investigate:

- intent;
- compatibility;
- incidents;
- prior bug fixes;
- behavior changes;
- rationale;
- hotspots.

Treat history as fallible evidence.

A frequently changed file is a prioritization signal, not proof of importance.

A rarely changed function may still be critical.

## 14.13 Real-boundary execution

Where a claim depends on real external semantics, include the real component when safe and feasible.

Examples:

- actual database engine;
- actual serialization library version;
- sandbox cloud service;
- real filesystem;
- browser engine;
- protocol endpoint;
- hardware or emulator when material.

Real systems are not automatically superior.

Use them when their semantics are material to the proposition.

---

# 15. Phase 9 — Challenge the measurement apparatus

Do not accept green tests at face value.

For consequential claims, ask:

> If this behavior were wrong in a plausible way, would the current observation system fail?

Possible challenges:

- mutate the implementation;
- deliberately invert a predicate;
- remove a state update;
- alter serialization;
- change a boundary value;
- inject dependency failure;
- alter ordering;
- bypass validation;
- return stale data;
- change configuration;
- simulate partial completion.

Only perform safe, reversible perturbations.

Record which tests detect each perturbation.

This phase is about validating sensors, not producing a ceremonial mutation score.

---

# 16. Phase 10 — Generate new evidence where gaps matter

Prioritize gaps using risk, not completeness rhetoric.

Useful prioritization signals include:

- behavioral criticality;
- change frequency;
- dependency centrality;
- incident history;
- external exposure;
- irreversible side effects;
- data sensitivity;
- security relevance;
- financial relevance;
- compatibility obligations;
- user impact;
- complexity;
- weak existing evidence.

For each important gap, choose the least expensive method that can legitimately support the needed claim.

Do not write a unit test when the proposition depends on real integration semantics.

Do not build an end-to-end environment when a pure local property can be established more strongly and cheaply in isolation.

---

# 17. Phase 11 — Write or modify tests

When persistent tests are the appropriate evidence vehicle:

## 17.1 Prefer behavioral assertions

Assert meaningful outputs, state transitions, side effects, invariants, contracts, and observable interactions.

Avoid unnecessary implementation-detail assertions.

## 17.2 Preserve useful existing tests

Do not rewrite the suite merely for stylistic consistency.

Retain tests that contribute independent evidence even if they are old or stylistically unusual.

## 17.3 Strengthen weak tests

Examples:

- replace “does not throw” with state/output assertions;
- add semantic assertions to broad end-to-end tests;
- replace tautological expected values;
- add missing error-state checks;
- validate persisted values, not merely repository method calls.

## 17.4 Use characterization tests honestly

Name and document characterization tests as continuity evidence where appropriate.

Do not silently promote them to normative specification.

## 17.5 Avoid test-generated circularity

Do not:

- copy the implementation into the test;
- calculate the expected output using the function being tested;
- create a mock that simply reproduces the agent's inferred implementation assumptions;
- assert that the calls made match the calls the agent just read from source unless call shape is the actual contract.

## 17.6 Use appropriate boundaries

Choose unit, integration, system, or real-boundary execution based on the proposition.

Do not assume one level is inherently superior.

---

# 18. Phase 12 — Re-run and compare

After modifications:

1. run the full relevant baseline;
2. run newly added evidence;
3. repeat sensitivity checks where warranted;
4. compare old and new behavior;
5. investigate unintended regressions;
6. confirm newly generated tests actually fail under the defects they are meant to detect when safe to simulate;
7. record environmental requirements;
8. check that test runtime and complexity remain operationally reasonable.

A larger suite that nobody can reliably execute is not automatically an improvement.

---

# 19. Phase 13 — Behavioral test-suite overhaul decisions

For each major test family, make one of the following recommendations.

## RETAIN

Keep because it supplies meaningful, distinct evidence.

## STRENGTHEN

Keep structure but improve assertions, fixtures, oracle, or environment fidelity.

## REPLACE

Replace because the current test cannot legitimately support the claim for which it is being used.

## CONSOLIDATE

Merge only where evidentiary value is genuinely duplicated.

## RECLASSIFY

Change documentation or naming because test scope is misrepresented.

Example: a test called “integration” that exercises only mocked collaborators.

## QUARANTINE

Temporarily isolate tests that are nondeterministic or environmentally invalid while preserving evidence of the problem.

## DELETE_WITH_JUSTIFICATION

Remove only when the test has no material independent evidentiary value or is actively misleading.

---

# 20. Phase 14 — Final behavioral evidence ledger

Produce a final ledger of consequential claims.

For each claim include:

- proposition;
- scope;
- status;
- supporting evidence;
- exercised domain;
- real components;
- substitutions;
- oracle;
- sensitivity evidence;
- provenance;
- remaining uncertainty.

Do not summarize the repository with a single “X% behavior covered” number unless behavior has been narrowly formalized.

---

# 21. Phase 15 — Uncertainty ledger

Explicitly report what remains outside current evidence.

Classify uncertainty as appropriate:

- unexecuted behavior;
- unmodeled inputs;
- untested configuration;
- unavailable external systems;
- unresolved intent;
- concurrency uncertainty;
- unsupported plugin/extension behavior;
- production-only semantics;
- historical ambiguity;
- insufficient oracle;
- insufficient sensitivity;
- inaccessible data;
- environment drift;
- nondeterminism.

The purpose is not to eliminate all uncertainty.

The purpose is to make consequential uncertainty visible.

---

# 22. Phase 16 — Final deliverables

The agent must produce the following artifacts.

## 22.1 Executive assessment

A concise explanation of:

- current test-system strengths;
- material weaknesses;
- most consequential behavioral gaps;
- highest-value improvements made;
- major unresolved uncertainty.

Avoid grading the suite with an unsupported single score.

## 22.2 Repository/test baseline

Include:

- repository revision;
- environment;
- build result;
- baseline test result;
- initial failures/skips;
- major test frameworks;
- major dependencies relevant to testing.

## 22.3 Behavioral surface inventory

List the major behavioral surfaces discovered and why they matter.

This list is repository-specific and need not be exhaustive.

## 22.4 Claim-evidence matrix

Include the final matrix of important propositions and their evidence state.

## 22.5 Existing-suite audit

Describe:

- strong evidence already present;
- weak assertions;
- invalid evidence boundaries;
- excessive substitution where material;
- misleading test categories;
- brittle snapshots;
- tautological or circular tests;
- true redundancy;
- significant missing observability.

## 22.6 New evidence generated

For every major new experiment or test:

- behavioral question;
- reason selected;
- method;
- result;
- scope;
- limitation.

## 22.7 Suite changes

Record:

- tests added;
- tests strengthened;
- tests replaced;
- tests consolidated;
- tests quarantined;
- tests deleted;
- test classifications changed.

Explain evidentiary reasons, not merely style reasons.

## 22.8 Sensitivity report

Describe how the improved suite was challenged.

Include:

- mutations/faults/perturbations attempted;
- which were detected;
- which survived;
- interpretation of surviving cases;
- limitations.

## 22.9 Uncertainty ledger

State explicitly what the investigation does not establish.

## 22.10 Recommended next investigations

Prioritize only consequential unresolved questions.

Do not generate an exhaustive backlog merely because unknowns exist.

---

# 23. Stopping conditions

Whole-codebase behavioral enumeration is generally not a realistic stopping criterion.

Stop or transition to maintenance when:

1. high-consequence behavioral surfaces have credible evidence;
2. major claims are appropriately scoped;
3. the suite demonstrates reasonable sensitivity to plausible faults in those areas;
4. important real-boundary assumptions have been exercised or explicitly excluded;
5. remaining uncertainty is documented;
6. additional investigation has lower expected value than its cost;
7. the suite remains operationally maintainable.

Do not claim complete behavioral understanding.

---

# 24. Priority heuristics

When investigation time is limited, prioritize using multiple signals.

Possible signals:

- user/business criticality;
- security consequence;
- financial consequence;
- irreversible state changes;
- public/external API exposure;
- historical defects;
- recent churn;
- dependency centrality;
- weak or absent tests;
- complex state transitions;
- external system boundaries;
- configuration sensitivity;
- concurrency;
- migration behavior;
- difficult rollback;
- large blast radius.

Do not use change frequency alone.

---

# 25. Anti-patterns the agent must actively avoid

## 25.1 Coverage theater

Increasing coverage without strengthening meaningful observation.

## 25.2 Test-count optimization

Treating more tests as inherently better.

## 25.3 Mock-confirmation loops

Inferring behavior from source, encoding that inference in mocks, then using those mocks as proof.

## 25.4 Snapshot absolutism

Treating current output as intended behavior.

## 25.5 Integration theater

Calling a test “integration” while substituting away the semantics material to its claim.

## 25.6 End-to-end theater

Running a broad stack while asserting almost nothing meaningful.

## 25.7 Static-to-runtime conflation

Reporting possible behavior as observed behavior.

## 25.8 Runtime-to-universal conflation

Generalizing one or several executions to all possible behavior.

## 25.9 Historical-intent conflation

Treating old documentation or tests as authoritative current intent without corroboration.

## 25.10 Mutation-score optimization

Chasing a high score instead of using surviving mutants diagnostically.

## 25.11 Universal-test ideology

Assuming unit, integration, or end-to-end tests are categorically superior.

## 25.12 Premature architecture repair

Refactoring production code merely because the current testing style finds it inconvenient, without determining whether the difficulty indicates poor design, an inappropriate test boundary, or inherent domain complexity.

## 25.13 Artificial certainty

Replacing `UNKNOWN` with a plausible story.

---

# 26. Agent decision discipline

Whenever the agent makes a substantive behavioral conclusion, it should be able to answer:

1. What exactly am I claiming?
2. What observations support it?
3. What domain did I actually exercise?
4. Which real components were present?
5. Which components were substituted?
6. What oracle or relation makes this meaningful?
7. Would my observation mechanism notice plausible wrongness?
8. What version/environment produced the evidence?
9. What remains outside the claim?

If the agent cannot answer these questions, narrow the claim or mark it unsupported/unknown.

---

# 27. Lightweight gating rules for a prompt-only prototype

These rules are intended to make the protocol usable before any external enforcement system exists.

The agent must not use the following labels without satisfying the associated conditions.

## `SUPPORTED`

Require:

- at least one concrete evidence record;
- explicit claim scope;
- exercised domain;
- environment/provenance;
- stated oracle or relation.

## `CORRECT`

Require:

- a normative oracle, specification, proof obligation, or justified domain requirement.

Current behavior alone is insufficient.

## `INTEGRATION_VERIFIED`

Require:

- identification of actual integrated components;
- disclosure of substitutions;
- evidence that semantics material to the claim were present.

## `TEST_SUITE ADEQUATE`

Do not use as a global conclusion.

Use scoped statements about particular behavioral surfaces.

## `REGRESSION_PROTECTED`

Require:

- a persistent executable sensor;
- demonstration that it detects a relevant change or plausible perturbation where practical.

## `UNKNOWN`

Use whenever evidence cannot support a stronger status.

---

# 28. Protocol self-audit

Before finalizing the work, the agent must review its own output.

Ask:

- Did I make any universal claim from sampled execution?
- Did I call current behavior correct without an oracle?
- Did I treat documentation as runtime evidence?
- Did I treat static possibility as actuality?
- Did I claim real integration from mocked execution?
- Did I generate expected results from the implementation itself?
- Did I equate coverage with quality?
- Did I recommend deleting tests based only on similarity?
- Did I hide unresolved uncertainty?
- Did I over-focus on one favored testing technique?
- Did I make changes that increase maintenance burden without clear evidentiary value?
- Did I preserve enough information to reproduce important findings?

Correct any violations before reporting completion.

---

# 29. Suggested working sequence

The following is a default sequence, not an inflexible workflow:

```text
1. Preserve repository state
2. Discover repository and test infrastructure
3. Establish baseline
4. Inventory current evidence
5. Identify consequential behavioral surfaces
6. Build initial claim-evidence matrix
7. Audit tests against claims
8. Select high-value evidence gaps
9. Generate additional evidence
10. Challenge the measurement apparatus
11. Modify the suite
12. Re-run and compare
13. Re-assess claim support
14. Record unresolved uncertainty
15. Produce final overhaul report
```

The agent may revisit earlier stages whenever new evidence changes the investigation.

---

# 30. Optional machine-readable summary

When feasible, produce a structured summary alongside human-readable findings.

```yaml
repository:
  revision:
  languages: []
  test_frameworks: []
  baseline:
    build:
    tests:

behavioral_surfaces:
  - id:
    description:
    criticality:
    status:

claims:
  - id:
    proposition:
    status:
    scope:
    evidence_ids: []
    uncertainty:

evidence:
  - id:
    type:
    observation:
    environment:
    oracle:
    limitations:

suite_changes:
  added: []
  strengthened: []
  replaced: []
  consolidated: []
  quarantined: []
  deleted: []

uncertainty:
  - category:
    description:
    consequence:
    next_step:
```

The schema should remain subordinate to the reasoning. Do not force repository-specific facts into inappropriate fields merely to satisfy structure.

---

# 31. Minimal completion criteria

A behavioral testing overhaul is not complete merely because tests pass.

At minimum, the investigation must leave behind:

- a reproducible baseline;
- a test/evidence inventory;
- a repository-specific behavioral surface model;
- scoped claims linked to evidence;
- an audit of important test weaknesses;
- additional evidence for material gaps where practical;
- at least some challenge of the observation apparatus for consequential behavior;
- justified suite changes;
- explicit uncertainty.

If any of these are impossible, state why.

---

# 32. Research-use notes

This protocol is itself a hypothesis about how an autonomous coding agent should investigate an unfamiliar test suite.

It should be evaluated rather than assumed correct.

A useful preliminary experimental comparison is:

### Condition A — Baseline

Same agent, same repository, same tools, same budget.

Instruction:

> Review this repository's test suite comprehensively and improve it.

### Condition B — Protocol

Same agent, repository, tools, and budget, but use this protocol.

Measure both conditions using identical criteria, including:

- behavioral facts correctly discovered;
- unsupported claim rate;
- false defect reports;
- missed consequential behavior;
- quality of uncertainty handling;
- reproducibility;
- meaningful new tests/evidence;
- ability to detect seeded or known defects;
- change-impact prediction;
- cost;
- run-to-run variance.

If the protocol does not materially improve results, avoid assuming that more infrastructure will solve the problem.

If the protocol improves results but exhibits recurring failures that prose instructions cannot control, those failures become candidates for later mechanical enforcement.

---

# 33. Methodological provenance

This protocol synthesizes ideas from established software-engineering areas including:

- program comprehension;
- software reverse engineering;
- feature location;
- software reconnaissance;
- characterization testing;
- test-oracle research;
- dynamic analysis;
- specification mining;
- property-based testing;
- differential testing;
- metamorphic testing;
- combinatorial testing;
- fuzzing;
- symbolic and concolic execution;
- mutation testing;
- fault injection;
- static analysis;
- formal verification;
- mining software repositories;
- architecture reconstruction;
- test-environment research.

The protocol also incorporates the following conceptual principles from the reviewed paper *Interrogating the Codebase: A Critical Synthesis of Evidence for Behavioral Comprehension of Existing Software Systems*:

- triangulation under bounded evidence;
- distinction among mechanism, actuality, intent, and correctness;
- evidence boundaries;
- evidence proportionality;
- observation-verdict separation;
- sensitivity auditing;
- claim-evidence representation;
- provenance and reproducibility;
- explicit uncertainty;
- automation for enumeration with reasoning for interpretation;
- caution against agent self-confirmation through generated mocks/tests.

These concepts should be treated as a proposed synthesis requiring empirical validation, not as a proven universal methodology.

---

# 34. Selected methodological references

- Ammons, G., Bodík, R., & Larus, J. R. (2002). *Mining specifications*. POPL.
- Barr, E. T., Harman, M., McMinn, P., Shahbaz, M., & Yoo, S. (2015). *The oracle problem in software testing: A survey*. IEEE Transactions on Software Engineering.
- Cadar, C., Dunbar, D., & Engler, D. (2008). *KLEE: Unassisted and automatic generation of high-coverage tests for complex systems programs*. OSDI.
- Chen, T. Y., Kuo, F.-C., Liu, H., Poon, P.-L., Towey, D., Tse, T. H., & Zhou, Z. Q. (2018). *Metamorphic testing: A review of challenges and opportunities*. ACM Computing Surveys.
- Claessen, K., & Hughes, J. (2000). *QuickCheck: A lightweight tool for random testing of Haskell programs*. ICFP.
- Cornelissen, B., Zaidman, A., van Deursen, A., Moonen, L., & Koschke, R. (2009). *A systematic survey of program comprehension through dynamic analysis*. IEEE Transactions on Software Engineering.
- Feathers, M. (2002). *Working Effectively With Legacy Code* (working paper).
- Godefroid, P., Klarlund, K., & Sen, K. (2005). *DART: Directed automated random testing*. PLDI.
- Inozemtseva, L., & Holmes, R. (2014). *Coverage is not strongly correlated with test suite effectiveness*. ICSE.
- Jia, Y., & Harman, M. (2011). *An analysis and survey of the development of mutation testing*. IEEE Transactions on Software Engineering.
- Just, R., Jalali, D., Inozemtseva, L., Ernst, M. D., Holmes, R., & Fraser, G. (2014). *Are mutants a valid substitute for real faults in software testing?* FSE.
- Kuhn, D. R., Kacker, R. N., & Lei, Y. (2010). *Practical Combinatorial Testing*. NIST SP 800-142.
- McKeeman, W. M. (1998). *Differential testing for software*. Digital Technical Journal.
- Wilde, N., & Scully, M. C. (1995). *Software reconnaissance: Mapping program features to code*. Journal of Software Maintenance: Research and Practice.
- Yang, X., Chen, Y., Eide, E., & Regehr, J. (2011). *Finding and understanding bugs in C compilers*. PLDI.

---

# 35. Compact instruction block for agent invocation

> Apply the Behavioral Testing Overhaul Protocol to this repository. Do not assume the language, framework, architecture, or existing test strategy in advance. Begin by preserving repository state, discovering the available build/test environment, and establishing the unmodified baseline. Treat existing tests as evidence-producing instruments rather than ground truth. Derive the repository's consequential behavioral surfaces, construct scoped claim-evidence relationships, audit whether current tests actually support those claims, and generate additional evidence where material gaps exist. Use characterization, property-based, differential, metamorphic, combinatorial, fuzzing, symbolic/concolic, mutation, fault-injection, static, historical, formal, and real-boundary techniques selectively when they fit the behavioral question and available capabilities; do not mechanically apply all techniques. Distinguish mechanism, actuality, intent, and correctness. Preserve observations separately from verdicts. Keep claims proportionate to exercised domains and real/substituted environments. Challenge the sensitivity of important tests where practical. Improve the suite only where changes strengthen meaningful behavioral evidence. Record provenance and unresolved uncertainty. Do not claim exhaustive behavioral understanding.

---

# 36. Final governing principle

A test suite is not trustworthy because it is large, fast, modern, highly covered, heavily mocked, integration-heavy, mutation-tested, or generated by an AI agent.

It is trustworthy to the extent that consequential behavioral claims can be connected to reproducible evidence, appropriate oracles, explicit execution domains, relevant real-world semantics, demonstrated sensitivity to plausible wrongness, and clearly bounded uncertainty.

The purpose of this protocol is to make that relationship inspectable and improvable.
