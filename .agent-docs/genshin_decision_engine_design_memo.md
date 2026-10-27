# Account-Aware Genshin Decision Engine
## Product Theory, Deterministic Modeling, Competitive Landscape, and Security Boundaries

**Document purpose.** This memo defines the intended design and conceptual boundaries of a proposed Genshin Impact recommendation and research system. It is not an implementation roadmap. It focuses on the problem being solved, the semantics of a trustworthy recommendation, the distinction between deterministic decision logic and AI assistance, the competitive landscape, the information model required to support account-wide reasoning, and the security/trust boundaries that matter if the system consumes account data or third-party tooling.

**Scope note.** The system should not embed a hard-coded character roster. Character/account ownership should be acquired dynamically from an account data source such as HoYoLAB Battle Chronicle or another authorized account-state integration. The important design problem is therefore not "what characters does this specific account own?" but "how should the engine reason over whatever account state it is given?"

---

# 1. Executive thesis

The system being proposed is best understood as an **account-aware decision engine**, not primarily as an AI coach, a static team builder, a damage calculator, or a combat simulator.

Its core function is to answer questions of the form:

> Given the current state of this account, what configuration is best under a stated objective, and what downstream consequences follow if one assumption changes?

That formulation is deliberately broader than "recommend a team." A useful system needs to reason across several coupled domains at once:

- the characters the player owns;
- constellations, talents, and other character state;
- weapons and weapon allocation;
- artifact inventories and build constraints;
- elemental reactions and aura requirements;
- party-wide buffs, debuffs, resonances, and conditional effects;
- field-time competition;
- energy requirements;
- rotation compatibility;
- survivability and interruption resistance;
- special mechanics such as Lunar, Stellar, Hexerei, Nightsoul, or future systems;
- enemy- or content-dependent restrictions;
- and player objectives such as maximum damage, low execution burden, survivability, flexibility, or efficient account-wide resource allocation.

The defining feature is **counterfactual propagation**. If a player asks:

> "What if I gave Zibai Mistsplitter Reforged?"

the system should not merely compare two swords in isolation. It should determine which parts of the account state are causally affected, recompute only the relevant decision space, and explain the consequences. That may include a new optimal artifact distribution, a change in personal damage, a change in team-level output, a different ordering of candidate teams, and the opportunity cost of taking the weapon away from another character.

A trustworthy architecture therefore has three distinct kinds of intelligence:

1. **Symbolic intelligence** — explicit rules and graph relationships that encode game mechanics and compatibility.
2. **Numerical intelligence** — combinatorial search, constraint solving, optimization, and quantitative evaluation.
3. **Statistical/linguistic intelligence** — optional AI models used for interpreting natural-language intent, extracting structured information from research, making explicitly qualitative judgments, or explaining deterministic results.

The first two should be authoritative wherever the game mechanics are formalizable. AI can add usability and research leverage, but it should not be used to "guess" a result that can be derived from code.

---

# 2. The actual problem: account-level decision support

Most existing Genshin tools solve one of four narrower problems:

1. **Static guidance** — "What is generally good for this character?"
2. **Build optimization** — "Given this character and these artifacts, what is the strongest build?"
3. **Team recommendation** — "Given this roster, which combinations look strong?"
4. **Combat simulation** — "Given this exact team and rotation, what happens?"

The proposed system is intended to unify the *decision logic above those layers*.

A conventional optimizer often assumes that the user already knows which character, team, weapon, objective, and rotation should be evaluated. An account-level decision engine instead asks whether those assumptions are themselves optimal.

Examples of questions it should conceptually support include:

- Which teams are strongest from the account I actually own?
- Which of those teams are strongest under a "comfortable" rather than maximum-DPS objective?
- Which teams are robust across general content versus highly encounter-specific?
- Which character is the best recipient of a contested weapon?
- If I change one weapon, which artifact builds should change?
- Does that item change merely improve one character, or does it change which team archetype is preferred?
- If one character gains a weapon, what is the account-wide opportunity cost to the previous holder?
- Which missing character would add the greatest number or quality of new team solutions?
- Which artifact improvement produces the largest *account-wide* marginal gain rather than merely improving one build?
- Are two high-scoring teams mutually exclusive because they consume the same critical support or weapon?
- What is the best pair or trio of non-overlapping teams for Spiral Abyss or another multi-team mode?
- Which recommendation is sensitive to assumptions such as enemy susceptibility to Freeze, available field time, or energy generation?

This makes the object of optimization the **account as a system**, not just the active character.

---

# 3. Goals and non-goals

## 3.1 Primary goals

### Account-specific rather than generic recommendations

The engine should reason from the player's actual state, not from generic assumptions about C0R1 characters or standardized artifacts unless the user explicitly requests theorycrafting mode.

A strong recommendation should be able to distinguish:

- generally strong in the meta;
- strong for this specific account;
- strong only if a missing item or teammate is acquired;
- strong but high-opportunity-cost because it consumes a scarce support or weapon;
- strong under one objective but inferior under another.

### Systematic team discovery

The engine should generate and evaluate candidate teams instead of requiring the user to supply every candidate manually.

For a roster of \(n\) characters, all unordered four-character teams are:

\[
\binom{n}{4}
\]

For 22 characters, that is 7,315 teams — a trivial search space computationally. The difficulty is not enumeration. The difficulty is evaluating interactions accurately and avoiding bad heuristics.

As the roster grows, brute-force enumeration remains feasible for four-person combinations, but the system can still use structural pruning to avoid wasting expensive downstream calculations on obviously incoherent teams.

### Counterfactual reasoning

Every meaningful change should propagate through a dependency graph.

A weapon change can affect:

- base stats;
- crit distribution;
- energy;
- passive uptime;
- damage type coverage;
- artifact substat valuation;
- artifact main-stat choice;
- set choice;
- rotation assumptions;
- team-level buffs or requirements;
- and opportunity cost elsewhere in the account.

The system should therefore be able to compare **world states**, not merely items.

### Explainability

A recommendation should be accompanied by machine-readable reasons that can be rendered into human explanation.

A useful explanation is not:

> "Team A scores 91."

It is:

> "Team A ranks above Team B because it preserves the required Hydro aura, activates Geo Resonance, gives the primary damage dealer a compatible buffer, has no major field-time collision, meets its energy threshold with lower ER investment, and retains sufficient sustain. Team B gains more nominal personal damage from its fourth slot but loses reaction uptime and extends the rotation."

The explanation layer should expose the actual features and constraints that caused the ranking.

### Reproducibility

Given:

- the same account snapshot;
- the same game-data version;
- the same objective;
- the same encounter assumptions;
- and the same engine version;

the system should produce the same result.

This is a key distinction between a decision engine and a general-purpose AI coach.

## 3.2 Non-goals

### Not a replacement for gameplay

The system recommends and evaluates; it does not need to automate combat inputs or interact with the game client.

### Not an LLM-first architecture

An LLM should not decide damage formulas, reaction legality, buff applicability, or artifact arithmetic when those can be encoded deterministically.

### Not necessarily a frame-perfect simulator

Full event-by-event combat simulation can be useful for validation or high-fidelity analysis, but it is not required for every decision. Much of the value can come from a higher-level team model using explicit mechanics, rotation constraints, uptime assumptions, and calibrated scoring.

### Not a generic tier list

The engine should not collapse account-specific opportunity cost and synergy into a global "S/A/B" character ranking.

### Not a hard-coded roster database owned by the user

The roster should come from dynamic account state. The system's durable intellectual property is the **mechanical ontology and decision logic**, not a static list of owned characters.

---

# 4. Deterministic architecture as the default

The central design principle is:

> If a conclusion can be derived from structured game mechanics, derive it from structured game mechanics.

This is important because Genshin has a large amount of formal structure:

- explicit character stats;
- explicit weapon stats;
- explicit artifact stats;
- deterministic damage formulas;
- reaction eligibility rules;
- resonance requirements;
- explicit party restrictions;
- duration and cooldown values;
- energy costs;
- conditional buffs;
- target types;
- element application;
- field-time requirements;
- and enumerable equipment combinations.

These are excellent candidates for algorithms, not probabilistic language models.

## 4.1 What should be deterministic

The following should normally be implemented as code or data-driven rules:

**Damage calculation.**  
Given the relevant stats, multipliers, resistance, defense, reaction state, and buffs, compute the result.

**Reaction legality.**  
Whether a reaction can occur should be a rule-system question.

**Buff applicability.**  
Whether a buff affects a target, damage category, reaction, on-field unit, off-field unit, element, or stat should be encoded explicitly.

**Weapon-stat propagation.**  
Changing a weapon should deterministically alter the character state and dependent formulas.

**Artifact optimization.**  
Given candidate artifacts, constraints, and an objective function, search the valid build space and rank it.

**Team enumeration.**  
Generate all four-character sets or generate only those satisfying basic constraints.

**Energy evaluation.**  
Energy requirements, particle assumptions, ER targets, and rotation feasibility should be numerical or rule-based.

**Field-time conflict.**  
Where input requirements and durations are known, conflicts can be estimated mechanically rather than guessed.

**Special mechanic activation.**  
Party requirements for systems such as Lunar, Stellar, Hexerei, Nightsoul, or any future system should be explicit predicates.

**Non-overlapping team selection.**  
Selecting the best two or three teams that cannot share characters is a combinatorial optimization problem.

**Opportunity cost.**  
If a weapon or artifact is moved, the previous owner and affected teams can be recomputed.

## 4.2 Why deterministic modeling matters

A deterministic engine gives several properties that are difficult to guarantee with an LLM:

- exact reproducibility;
- regression testing;
- source-level auditability;
- causal explanations;
- performance benchmarking;
- model versioning;
- confidence based on known assumptions rather than language-model self-assessment;
- clear failure modes;
- and the ability to test edge cases independently.

A model that says "Mistsplitter is probably a good fit with 0.71 confidence" is weaker than an engine that can state exactly which passive terms are useful, which are irrelevant, how the build changes, and what the measured objective delta is.

The system should therefore prefer:

> "Build B is 6.4% lower under these assumptions"

over:

> "Build B seems slightly worse."

---

# 5. The knowledge model: facts, relationships, and conditions

The system requires a structured representation of Genshin mechanics. A flat table of "character A works with character B" is not enough.

The preferable model is a **knowledge graph or typed relational ontology** in which entities and effects are explicit.

## 5.1 Character representation

A character profile should include more than element and nominal role.

Representative dimensions include:

- identity and version;
- element;
- weapon type;
- rarity;
- constellation state;
- talent state;
- base stats;
- primary and secondary scaling stats;
- damage categories;
- on-field/off-field capability;
- field-time demand;
- snapshot/dynamic behavior where relevant;
- infusion or conversion behavior;
- energy cost;
- particle generation assumptions;
- elemental application;
- aura requirements;
- internal cooldown or application cadence where relevant;
- team buffs;
- self buffs;
- debuffs;
- healing;
- shielding;
- interruption resistance;
- grouping;
- mobility;
- range;
- special mechanic tags;
- resonance contribution;
- reaction preferences;
- reaction exclusions;
- rotation windows;
- cooldown windows;
- and conditional effects.

"Role" should be multi-valued rather than a single label. A character can simultaneously be:

- off-field damage;
- reaction enabler;
- buffer;
- healer;
- battery;
- and driver under a specific configuration.

## 5.2 Equipment representation

Weapons and artifacts should be represented as effects rather than just stat bundles.

A weapon may:

- provide a static stat;
- grant a conditional stat;
- activate after a skill, burst, normal attack, or damage event;
- require field time;
- depend on stacks;
- buff only certain damage types;
- affect teammates;
- affect only the wielder;
- change ER constraints;
- or create opportunity cost because it is uniquely strong on another unit.

An artifact set is similarly a set of predicates and effects.

## 5.3 Effect graph

Conceptually:

```text
Mistsplitter Reforged
    ├── grants → CRIT_DMG
    ├── grants → elemental damage
    └── grants → stack-dependent elemental damage

Character
    ├── scales_with → DEF
    ├── produces → damage instance X
    ├── produces → reaction instance Y
    ├── requires → aura condition Z
    └── benefits_from → CRIT

Damage instance X
    ├── affected_by → CRIT
    ├── affected_by → DEF
    └── not_affected_by → selected conventional DMG% term

Support
    └── grants → buff B
            └── applies_if → target condition C
```

The evaluator then asks whether edges actually connect.

A support's theoretical buff value should be zero if no meaningful damage source can receive it.

## 5.4 Pairwise relationships are necessary but insufficient

For \(n\) characters, there are:

\[
\binom{n}{2}
\]

pairwise relationships. For 22 characters, that is 231 unique pairs.

Pairwise compatibility is useful for early scoring and pruning, but team quality cannot be computed by simply adding six pair scores.

Why?

Because many interactions are **higher-order**:

- a third element can destroy an aura required by the first two;
- an extra on-field attacker may make a previously compatible pair impossible to rotate;
- energy needs can change based on the number of same-element teammates;
- a buff can become redundant because another teammate already supplies the same non-stacking effect;
- a healer may be unnecessary in a composition with sufficient defensive utility;
- a reaction archetype can require a specific four-character arrangement.

The system therefore needs team-level evaluation after pairwise analysis.

---

# 6. Team classification dimensions

A "good team" should not be defined by a single heuristic. It should be evaluated across a set of dimensions.

## 6.1 Role completeness

Questions include:

- Where does most of the team's damage come from?
- Is the team built around one carry, multiple quick-swap sources, reactions, or passive/off-field damage?
- Are the necessary enabling roles present?
- Is sustain required, and if so, is it present?
- Is the team's intended damage engine actually supported?

## 6.2 Reaction coherence

The engine should determine:

- what elemental auras the team is likely to create;
- which reactions are enabled;
- whether reaction ownership matters;
- whether an extra element interferes with the intended reaction;
- whether the team is intentionally mono-element and therefore should not be penalized merely for having fewer reactions.

This prevents simplistic logic such as:

> "All-Cryo is bad because it does not Freeze."

A mono-Cryo team might be excellent if its characters have strong internal synergy, compatible buffs, energy efficiency, resistance reduction, and favorable content assumptions.

## 6.3 Buff and debuff compatibility

The system should ask:

- what does the buff modify?
- who can receive it?
- does the primary damage source actually use that stat?
- is the effect snapshot or dynamic?
- does it stack?
- is another teammate already providing the same category?
- is uptime realistic?
- does the buff affect reaction damage or only conventional damage?

A nominally large buff that applies to none of the team's meaningful damage should contribute nothing.

## 6.4 Field-time compatibility

Multiple characters can be individually strong and collectively bad because each requires an incompatible amount of on-field time.

This should be modeled as a scheduling/rotation constraint rather than an arbitrary "two DPS = bad" rule.

Quickswap units should not be penalized like extended-field carries.

## 6.5 Energy compatibility

Energy should be treated as part of build quality rather than an afterthought.

A team that requires unrealistic ER to execute its intended rotation is materially weaker even if its one-cycle damage sheet looks strong.

Relevant features include:

- burst cost;
- particle generation;
- funneling;
- same-element particle value;
- Favonius-like effects;
- time between bursts;
- rotation length;
- and ER opportunity cost.

## 6.6 Elemental application and aura stability

Two teams with the same four elements can behave very differently depending on application rate, timing, gauge, and whether the relevant source is on-field or off-field.

The system should therefore separate:

> "contains Hydro"

from:

> "provides enough Hydro application during the relevant window to support the intended reaction."

## 6.7 Rotation compatibility

Cooldown and duration alignment matter.

The engine should evaluate:

- burst/skill cooldowns;
- duration of buffs;
- duration of summons or off-field effects;
- field-time windows;
- swap requirements;
- and whether the nominal rotation is repeatable.

## 6.8 Defensive sufficiency

Survivability should not be treated as binary "has healer / no healer."

Useful defensive dimensions include:

- healing throughput;
- shielding;
- interruption resistance;
- damage reduction;
- crowd control;
- range;
- mobility;
- invulnerability frames intrinsic to the rotation;
- and content-specific defensive needs.

## 6.9 Special-system coherence

Modern Genshin increasingly includes named combat subsystems.

The engine needs explicit support for mechanic families such as:

- Lunar;
- Stellar;
- Hexerei;
- Nightsoul;
- and future systems.

These should not be reduced to element alone.

## 6.10 Enemy/content dependence

A recommendation should distinguish:

- generally strong;
- boss-specialized;
- AoE-specialized;
- Freeze-dependent;
- shield-sensitive;
- energy-sensitive;
- mobile-enemy-sensitive;
- or cycle-specific.

This prevents a single score from pretending that all combat environments are interchangeable.

---

# 7. Hard constraints, soft scores, and Pareto reasoning

Not every weakness should be treated equally.

## 7.1 Hard constraints

A hard constraint means the configuration cannot satisfy the intended archetype or objective.

Examples:

- a mechanic explicitly requires Hydro, but the team has no source of Hydro;
- a party restriction forbids the included element;
- two builds attempt to equip the same unique artifact or weapon in a simultaneous multi-team optimization;
- a requested mode forbids reusing characters between teams.

Hard constraints remove candidates.

## 7.2 Soft penalties

Other properties should reduce quality without invalidating the team:

- excess ER requirement;
- awkward field-time overlap;
- incomplete buff uptime;
- lack of sustain;
- encounter dependence;
- weak AoE coverage;
- high execution difficulty;
- or redundant buffs.

These should remain visible as trade-offs.

## 7.3 Why a single score is insufficient

A single score is useful for ranking but dangerous as a semantic representation.

Two teams can have the same score for completely different reasons:

- Team A: excellent damage, poor comfort.
- Team B: lower peak damage, excellent consistency.
- Team C: highly encounter-specific but dominant in its target content.

A stronger model stores a vector of dimensions and derives the presentation score from an explicit objective.

For example:

```text
damage potential        0.91
energy stability        0.84
reaction coherence      0.97
survivability           0.63
rotation simplicity     0.42
enemy generality        0.76
resource opportunity    0.88
```

Then "maximum damage," "balanced," "comfortable," and "generalist" can weight the same underlying facts differently.

## 7.4 Pareto-optimal teams

A useful decision engine should expose the **Pareto frontier** rather than pretending there is always one objectively best team.

A team is Pareto-dominated if another team is at least as good on every relevant dimension and better on at least one.

This gives the user a principled shortlist:

- highest damage;
- easiest rotation;
- strongest survivability;
- best general-purpose team;
- lowest contested-resource cost;
- best boss team;
- best AoE team.

The final choice can then reflect player preference without corrupting the underlying mechanics.

---

# 8. Counterfactual propagation: the defining feature

The strongest differentiator of the proposed system is not team generation itself. It is the ability to model **changes to the account state**.

## 8.1 Example structure

Suppose the user asks:

> What happens if Character A receives Weapon W?

The engine should conceptually perform:

```text
weapon assignment changes
        ↓
character stats change
        ↓
weapon conditionals change
        ↓
artifact marginal values change
        ↓
optimal artifact build may change
        ↓
personal damage / utility changes
        ↓
team scores involving Character A change
        ↓
ranking of those teams changes
        ↓
previous holder of Weapon W loses the weapon
        ↓
their optimal build changes
        ↓
their teams change
        ↓
account-wide result changes
```

This is a dependency graph problem.

## 8.2 Local recomputation

The engine should not need to recompute the entire universe after every small change.

If a dependency graph is explicit, the evaluator can identify the affected subgraph.

For example:

- changing one artifact affects its holder;
- that affects teams containing the holder;
- that can affect a simultaneous multi-team solution if those teams compete for resources.

This provides both performance and interpretability.

## 8.3 Opportunity cost

Most build tools evaluate the recipient but not the donor.

An account-level engine must answer:

> Is giving this weapon to Zibai good?

and:

> Is giving this weapon to Zibai the *best use of this weapon on the account*?

Those are different questions.

The second requires comparing:

\[
\Delta_{\text{recipient}} - \Delta_{\text{previous holder}} + \Delta_{\text{team interactions}}
\]

and potentially recomputing both sides.

## 8.4 Marginal roster value

The same concept extends to pulls.

A new character is valuable not only because they are individually strong, but because they may:

- create new team archetypes;
- replace a weak slot in several teams;
- free a contested support for another side;
- reduce resource conflicts;
- make existing weapons useful;
- or improve multi-team coverage.

This is a more meaningful account-specific "pull value" than a generic tier list.

---

# 9. Objective functions and player intent

A deterministic engine does not require a universal definition of "best." It requires an explicit objective.

Examples:

- maximize single-target DPS;
- maximize AoE effectiveness;
- maximize expected clear reliability;
- minimize rotation difficulty;
- maximize survivability;
- maximize general-purpose coverage;
- minimize contested-resource usage;
- maximize two-team or three-team aggregate quality;
- maximize improvement per unit of investment;
- maximize account-wide marginal gain.

A user might say:

> "I want a comfortable team and don't care about speedrun damage."

That natural-language statement should be translated into a deterministic objective, for example:

```text
damage weight            0.55
survivability weight     1.00
rotation complexity max  2/5
energy stability weight  0.90
exclude characters       [...]
```

The AI, if present, interprets the request. The deterministic engine solves it.

---

# 10. AI is optional, but useful at the edges

The system can be highly capable without AI in the core.

That is not an anti-AI position. It is a division-of-responsibility decision.

## 10.1 Good uses of AI

### Natural-language intent parsing

User:

> "Give me a comfy Zibai team. I hate strict rotations and I don't want Bennett."

AI converts that into explicit constraints and weights.

### Research ingestion

New characters and mechanics release continuously. Human-readable sources may contain:

- official kit text;
- theorycrafting notes;
- KQM guides;
- community testing;
- issue discussions;
- known bugs;
- patch notes.

An AI or decision model can propose structured facts from this corpus:

```yaml
field_time: high
scaling:
  - def
role:
  - on_field_damage
requires:
  - hydro_aura
benefits_from:
  - geo_teammate
```

The extracted facts should then be validated against schemas, evidence, tests, or human review before becoming canonical.

### Qualitative classifications

Some dimensions are partly subjective:

- rotation difficulty;
- forgiveness;
- overworld comfort;
- cognitive burden;
- controller/mobile friendliness;
- reliance on tight animation cancels.

A statistical model may be reasonable here, provided the output is clearly marked as qualitative.

### Tool routing

If the system becomes agentic, an AI decision model can determine:

- whether a question requires inventory lookup;
- whether artifact reoptimization is necessary;
- whether external research is needed;
- whether a combat simulator is warranted;
- or whether deterministic cached results are sufficient.

### Explanation generation

Once the engine has produced structured reasons, an LLM can render them into concise or detailed prose.

The LLM should explain the engine's result rather than invent the result.

## 10.2 Bad uses of AI

AI should generally not be the source of truth for:

- damage formulas;
- reaction legality;
- artifact arithmetic;
- buff applicability when rules are known;
- weapon-stat propagation;
- energy math;
- team enumeration;
- exact set constraints;
- or deterministic regression tests.

If the answer is computable, compute it.

---

# 11. Decision models such as SubQ Snap and Jev

SubQ Snap and Jev are relevant, but they should be considered **supporting infrastructure**, not the foundational game engine.

Both products are examples of decision models: they receive context plus a finite set of questions/answers and return structured labels, choices, scores, or probability distributions rather than arbitrary prose.

SubQ describes Snap as appropriate for routing, ranking, classifying, and yes/no decisions, with long-context support. Jev similarly exposes structured question types and calibrated probability distributions.

That is appealing for several parts of this system.

## 11.1 Strong fit: research classification

A long bundle of patch notes, theorycrafting documents, and issue discussions can be classified into a bounded ontology.

Example:

```text
Question: What is the primary field-time profile?
Choices: low | medium | high

Question: Which stats directly scale the principal damage instance?
Choices: ATK | DEF | HP | EM | mixed

Question: Does the support effect apply off-field?
Choices: yes | no | conditional
```

A decision model can efficiently propose those labels.

## 11.2 Strong fit: agent routing

Given a user question and account state:

```text
Need inventory lookup? yes/no
Need artifact optimization? yes/no
Need team regeneration? yes/no
Need external research? yes/no
Need simulator? yes/no
```

That is exactly the kind of bounded classification task decision models are designed for.

## 11.3 Potential fit: subjective team attributes

Decision models may help classify:

- easy vs demanding;
- forgiving vs brittle;
- generalist vs specialist;
- comfortable vs high-maintenance.

Those outputs should remain distinct from exact mathematical properties.

## 11.4 Poor fit: numerical mechanics

A decision model should not replace a direct formula with a probability distribution.

If a damage multiplier or energy requirement can be derived from known values, model inference adds uncertainty without adding value.

## 11.5 Snap versus Jev

The choice between these tools is secondary to architecture.

As of late October 2026:

- SubQ markets Snap as a long-context decision model with structured answer spaces and very large context capacity.
- Jev exposes a decision API that answers multiple structured questions from a supplied state and returns probabilities.

For this system, the meaningful comparison is not "which model is smarter?" but:

- how much research context needs to be processed at once;
- whether model outputs need to remain stable across versions;
- cost and latency;
- calibration quality;
- privacy and data-retention terms;
- and how easily the model can be isolated from sensitive account data.

The core engine should remain functional if either service is removed.

---

# 12. Competitor landscape

No single product identified in this research clearly combines all of the following:

1. persistent/dynamic account state;
2. systematic team discovery;
3. precise build optimization;
4. account-wide counterfactual propagation;
5. opportunity-cost reasoning;
6. deterministic and reproducible evaluation;
7. causal explanations.

Several products cover important subsets.

---

## 12.1 MondLab

**Position:** roster-aware team recommendation.

MondLab is the closest existing product to the proposed team's **classification and search layer**.

Its published scoring methodology explicitly considers:

- baseline character strength;
- pair synergy;
- useful buffs;
- elemental application;
- damage contribution;
- reaction requirements;
- field-time competition;
- energy;
- and whether support effects actually function in the proposed team.

It also emphasizes that its scores are comparative recommendations rather than damage calculations.

### What MondLab does well

- Generates teams from a selected roster.
- Handles more than simplistic element matching.
- Models pair synergy and team-level rules.
- Recognizes field-time conflict.
- Applies current endgame-cycle constraints.
- Supports non-overlapping team suggestions for modes requiring multiple teams.
- Provides pull-value style analysis for missing characters.
- Has a clear conceptual distinction between "recommendation score" and "simulated damage."

### What MondLab does not solve

Its current model does not track exact constellation and equipment state. Instead, it exposes power multipliers for manual adjustment.

That means it is not well suited to precise questions such as:

> "If this exact weapon moves to this character, how does the optimal artifact build and account-wide team ranking change?"

Its implementation is also not presented as an open-source library intended for integration. This matters if the goal is to build an extensible decision system rather than merely use a website.

### Lessons to borrow

MondLab validates several important design assumptions:

- pairwise synergy alone is insufficient;
- field time belongs in team evaluation;
- reaction requirements must be explicit;
- support effects must be checked for applicability;
- comparative team scores are useful even when they are not DPS simulations.

**Reference:** [MondLab scoring methodology](https://mondlab.net/scoring)

---

## 12.2 Genshin Optimizer (frzyc/genshin-optimizer)

**Position:** precise build and artifact optimization from actual inventory.

Genshin Optimizer is the strongest open foundation identified for the **equipment and calculation** side of the problem.

Its public repository includes:

- the GOOD import format used by scanners;
- Genshin character/stat data;
- calculation infrastructure;
- artifact optimization;
- teams and loadouts;
- theorycrafting-related configuration;
- a large body of maintained game-domain logic.

The repository is MIT licensed.

### What Genshin Optimizer does well

- Uses actual inventory rather than generic best-in-slot assumptions.
- Supports artifact optimization.
- Models character, weapon, artifact, and team effects.
- Has a mature scanner/import ecosystem around GOOD.
- Contains a substantial maintained body of game-specific mechanics.
- Is open source and therefore inspectable and reusable within license terms.
- Has a meaningful automated test/CI ecosystem.
- Maintains structured game-data pipelines.

### Where it differs from the proposed system

Its primary question is closer to:

> "Given this character/team/configuration, what build is best?"

The proposed system starts one level earlier:

> "Given the entire account, which configurations should be evaluated at all?"

Genshin Optimizer therefore looks more like a potentially valuable **calculation/data dependency** than a complete replacement for the recommendation engine.

### Architectural caution

The current Genshin frontend is described by its own repository as using the now-deprecated Waverider calculation engine, while newer Pando-related calculation work exists elsewhere in the monorepo.

The project also previously experimented with a fuller Next.js/Supabase account-backed architecture and later removed it. That history suggests that adding login/cloud persistence directly upstream is not a small feature; it is a product and architecture decision.

This is one reason a separate decision layer may be cleaner than a deep frontend fork.

**References:**

- [Genshin Optimizer repository](https://github.com/frzyc/genshin-optimizer)
- [Genshin Optimizer README](https://github.com/frzyc/genshin-optimizer/blob/master/README.md)

---

## 12.3 gcsim

**Position:** detailed combat simulation for a specified team and rotation.

gcsim models combat from explicit user inputs and produces outputs including:

- total damage;
- per-character damage;
- per-skill damage;
- reaction counts;
- energy regeneration;
- and other combat statistics.

### What gcsim does well

- High-fidelity validation of an already-selected team.
- Explicit rotations.
- Energy/reaction tracking.
- Repeatable simulation.
- Useful theorycrafting and benchmarking infrastructure.

### What it does not solve

gcsim is principally an evaluator of a chosen configuration, not an account-wide search engine.

It generally assumes the user or another system has already decided:

- which four characters;
- which builds;
- which rotation;
- which configuration.

Therefore it fits below the proposed decision engine, not above it.

### Potential conceptual relationship

The decision engine could use a lower-cost deterministic team model for broad search and reserve simulation for ambiguous or high-value finalists.

The core product should not require a full combat simulator for every candidate.

**Reference:** [gcsim documentation](https://docs.gcsim.app/)

---

## 12.4 HakkoAI Genshin AI Coach

**Position:** conversational, screen-aware game coaching.

HakkoAI describes its Genshin product as a screen-aware and voice-enabled coach that can help with:

- builds;
- party setup;
- artifacts;
- Spiral Abyss;
- Resin planning;
- bosses;
- quests;
- and pull decisions.

### What HakkoAI does well

- Low-friction conversational interface.
- Contextual help during active gameplay.
- Screen awareness.
- Broad coverage of moment-to-moment player questions.
- Natural-language explanation.

### What it does not clearly demonstrate

Its published description does not establish that it:

- maintains a normalized full account inventory;
- exhaustively or systematically generates candidate teams;
- performs deterministic build optimization across the account;
- propagates item changes through all affected teams;
- models donor opportunity cost when moving equipment;
- exposes reproducible scoring assumptions;
- or produces a stable causal trace of why rankings changed.

The distinction is therefore:

**AI coach:** "Here is a useful recommendation for your current situation."

**Decision engine:** "Given this exact state and objective, these are the candidate configurations, these constraints removed these candidates, these calculations caused this ranking, and this is how the result changes under a counterfactual."

HakkoAI may overlap substantially in user experience without replacing the computational architecture proposed here.

**Reference:** [HakkoAI Genshin AI Coach](https://www.hakko.ai/blog/article/genshin-ai-coach/)

---

## 12.5 KQM and the Theorycrafting Library

**Position:** vetted human theorycrafting knowledge.

KQM is not a personalized optimizer, but it is an important comparator because it represents high-quality human knowledge about:

- damage formulas;
- elemental mechanics;
- character kits;
- builds;
- weapons;
- rotations;
- teams;
- and tested edge cases.

Its Theorycrafting Library explicitly maintains evidence-backed submissions and reviewed summaries.

### What KQM does well

- Human-vetted mechanics.
- Evidence and source context.
- Edge-case documentation.
- Character-specific guidance.
- Theorycrafting terminology and community consensus.

### What it does not solve

- no persistent account optimization;
- no systematic candidate search over the player's inventory;
- no automatic counterfactual propagation;
- no account-wide opportunity-cost engine.

### Role in the proposed system

KQM-like resources are valuable as **evidence and validation sources** for the ontology.

The engine should not treat prose guides as runtime truth. Instead, vetted conclusions can be converted into structured rules with provenance.

**Reference:** [KQM Theorycrafting Library](https://library.keqingmains.com/)

---

## 12.6 Other local-first artifact optimizers

There are also newer tools focused on the narrower question:

> "Which five artifacts from my GOOD inventory should this character equip?"

For example, GenshinOptimizer.com describes a local-browser workflow that imports GOOD files, stores them locally, and ranks artifact combinations.

This is useful evidence that GOOD is an effective interoperability boundary and that local-first optimization is practical.

It still does not address account-wide team discovery or counterfactual team reasoning.

**Reference:** [GenshinOptimizer.com](https://www.genshinoptimizer.com/)

---

# 13. Competitive gap

The competitive landscape can be summarized as:

| System | Account/Roster Awareness | Team Discovery | Exact Equipment | Artifact Optimization | Combat Simulation | Counterfactual Propagation | Deterministic/Auditable |
|---|---:|---:|---:|---:|---:|---:|---:|
| MondLab | Yes, roster | Strong | Abstracted | No | No | Limited | Partially opaque |
| Genshin Optimizer | Strong inventory model | Limited/manual | Strong | Strong | Partial/model-based | Character/build scope | Strong |
| gcsim | Manual/config-driven | No | Strong | Limited/adjacent | Strong | Manual | Strong |
| HakkoAI | Contextual | Conversational | Unclear | Advisory | No evidence of exhaustive sim | Unclear | Low/opaque |
| KQM | No personal account | Curated examples | Generic/reference | Guidance | Theorycrafting | No | High evidence, not computational |
| Proposed engine | Strong | Strong | Strong | Strong or delegated | Optional | **Core feature** | **Core requirement** |

The proposed product is differentiated primarily by:

1. **systematic account-wide search;**
2. **dependency-aware counterfactuals;**
3. **opportunity-cost reasoning;**
4. **objective-aware ranking;**
5. **causal explanations;**
6. **deterministic reproducibility.**

---

# 14. Security and trust boundaries

Security matters because the system may eventually interact with:

- account data;
- external APIs;
- third-party scanners;
- open-source repositories;
- AI services;
- and local development tooling.

The correct security posture is to separate these trust domains instead of treating "Genshin data" as one homogeneous input.

---

## 14.1 Account-state boundary

The account-state interface should return only the data required for recommendations.

Examples may include:

- owned characters;
- constellation/talent state;
- weapons;
- equipped state;
- artifact state where available;
- and other relevant progression metadata.

The system should not require game credentials if an authorized account-data mechanism can supply the necessary state.

Core principle:

> Never make the recommendation engine responsible for holding credentials it does not need.

Authentication/session material should be isolated from the mechanics engine.

---

## 14.2 Public mechanics versus private account state

The system should separate:

**Public/static knowledge**

- character kit definitions;
- weapon definitions;
- artifact definitions;
- reactions;
- formulas;
- published mechanics;
- theorycrafting evidence.

from:

**Private/user-specific state**

- owned roster;
- constellations;
- equipment inventory;
- artifact inventory;
- saved preferences;
- historical recommendations.

This separation improves:

- privacy;
- cacheability;
- testing;
- reproducibility;
- and least-privilege design.

A public mechanics engine should be usable without access to a user's account.

---

## 14.3 Scanner boundary

Genshin Optimizer's GOOD ecosystem illustrates an important architecture:

```text
scanner
    ↓
GOOD JSON
    ↓
schema validation
    ↓
optimizer
```

The optimizer can treat the exported file as data.

The scanner itself may be far more privileged.

Different scanners may use:

- OCR;
- screenshot capture;
- automatic input;
- game-window interaction;
- packet capture;
- or network sniffing.

Those are materially different risk profiles.

A recommendation engine that consumes GOOD should not imply that every program capable of generating GOOD is equally trustworthy.

The file format is one trust boundary; the scanner executable is another.

---

## 14.4 Schema validation boundary

All external structured data should be validated before it reaches the canonical state model.

This includes:

- GOOD imports;
- account API data;
- research extraction;
- cached AI decisions;
- manually edited configuration.

Invalid or unexpected fields should fail closed or be isolated rather than silently interpreted.

---

## 14.5 Open-source repository execution boundary

Cloning a public repository and executing its development toolchain are different security events.

A repository can contain:

- package-manager binaries;
- preinstall/install/postinstall scripts;
- custom build tooling;
- workspace plugins;
- shell scripts;
- submodules;
- test executors;
- IDE tasks;
- and CI definitions.

The Genshin Optimizer repository, for example, includes a repository-local Yarn configuration and a preinstall Node-version check. Its ordinary development workflow executes a large JavaScript dependency graph.

The appropriate conceptual security boundary is:

> Source inspection can happen in a low-privilege context; code execution should happen in an isolated environment until the dependency/tooling chain is trusted.

A development environment used to evaluate third-party code should ideally not expose:

- SSH agents;
- broad GitHub tokens;
- cloud credentials;
- password-manager sockets;
- host home directories;
- Docker daemon sockets;
- or other high-value secrets.

This is not a claim that the repository is malicious. It is normal supply-chain hygiene for large open-source JavaScript projects.

---

## 14.6 Dependency and CI/CD boundary

If external open-source components become dependencies of the proposed system, the concern is broader than malicious maintainers.

Risks include:

- compromised packages;
- moved action tags;
- dependency confusion;
- vulnerable transitive dependencies;
- unsafe build scripts;
- untrusted build artifacts crossing into privileged deployment workflows.

A mature system should distinguish:

- source dependency;
- build-time dependency;
- runtime dependency;
- privileged deployment dependency.

This becomes especially important if the project eventually stores user data.

---

## 14.7 AI-service boundary

If SubQ Snap, Jev, or an LLM is used, the system should define exactly what data may leave the deterministic core.

An ideal default is:

- public research can be sent to external AI services;
- account-specific state should be minimized or pseudonymized where possible;
- credentials should never be sent;
- raw artifact inventories should not be sent unless the feature genuinely requires them;
- AI output should be treated as an untrusted proposal until validated.

An AI model should not become a covert persistence layer for account data.

---

# 15. Provenance, evidence, and uncertainty

A deterministic engine can still be wrong if its inputs are wrong.

Therefore the knowledge model should distinguish:

- official game data;
- directly tested mechanics;
- community theorycrafting consensus;
- inferred approximations;
- encounter assumptions;
- subjective ratings.

Every nontrivial mechanic should ideally have provenance.

Conceptually:

```yaml
effect:
  id: example_effect
  rule: ...
  source_type: theorycrafting_evidence
  source: ...
  game_version: ...
  confidence_class: verified
```

This makes it possible to answer:

> "Why does the engine believe this?"

without relying on a model's internal reasoning.

## 15.1 Versioning

Genshin changes over time.

Results should therefore be versioned against:

- game version;
- mechanics dataset version;
- engine version;
- account snapshot timestamp;
- objective configuration.

A recommendation without version context can become misleading after balance changes or new mechanics.

## 15.2 Uncertainty should be localized

If one part of a team score depends on an approximation, the system should identify that exact component.

For example:

```text
Damage formula          exact
Weapon stats            exact
Reaction eligibility    exact
Energy model            estimated
Rotation difficulty     qualitative
Enemy grouping value    encounter-dependent
```

This is much more informative than a single vague "confidence score."

---

# 16. Team labels should describe archetypes, not judgments

A team should be classified by both **what it is** and **how good it is**.

Example:

```text
Archetype: Freeze
Quality: Strong
Primary damage source: Ayaka
Reaction coherence: Excellent
Energy stability: Good
Survivability: Excellent
Boss generality: Conditional
Rotation difficulty: Moderate
```

Another:

```text
Archetype: Mono Cryo
Quality: Strong
Reaction contribution: Low by design
Buff compatibility: Excellent
Energy stability: Excellent
Enemy dependence: Moderate
```

This avoids conflating:

> "No reaction"

with:

> "Bad team."

---

# 17. Explainability as a first-class output

The engine should generate structured reason codes while evaluating candidates.

Examples:

```text
BUFF_COMPATIBLE
AURA_REQUIREMENT_SATISFIED
FIELD_TIME_CONFLICT
ENERGY_DEFICIT
REACTION_INTERFERENCE
RESONANCE_ACTIVE
SPECIAL_SYSTEM_ACTIVE
SUSTAIN_SUFFICIENT
RESOURCE_CONFLICT
ENEMY_DEPENDENT
```

The numerical evaluator stores those facts.

The UI or LLM can later produce:

> "This team falls behind because the fourth character introduces Electro, which destabilizes the Freeze aura, and the resulting benefit does not compensate for the lost Freeze uptime."

This is preferable to an LLM inventing a post-hoc explanation for a black-box score.

---

# 18. Determinism does not mean oversimplification

There is a risk of treating "deterministic" as synonymous with "simple."

The proposed engine can still model uncertainty.

Examples:

- probabilistic particle generation;
- enemy behavior distributions;
- variable reaction ownership;
- imperfect buff uptime;
- latency or execution variation;
- multi-target hit distributions.

The correct approach is to make the uncertainty explicit.

For example:

\[
E[\text{team output}] = \sum_i P(s_i)\cdot \text{output}(s_i)
\]

A probabilistic numerical model is still deterministic in the engineering sense if the same probability assumptions produce the same expected result.

The distinction is:

**Explicit stochastic model:** assumptions are known and inspectable.

**LLM guess:** the probability distribution is implicit and difficult to audit.

---

# 19. A conceptual scoring model

The exact weights should be objective-specific, but a general team-quality vector may contain:

\[
Q =
(D, R, B, E, F, S, G, C, O)
\]

where:

- \(D\): damage potential;
- \(R\): reaction coherence;
- \(B\): buff/debuff compatibility;
- \(E\): energy stability;
- \(F\): field-time and rotation fit;
- \(S\): sustain/defensive sufficiency;
- \(G\): generality across encounters;
- \(C\): special-system coherence;
- \(O\): opportunity-cost efficiency.

A user objective produces weights:

\[
\text{Score} = w_DD + w_RR + w_BB + \cdots + w_OO
\]

But hard constraints are applied before scoring.

This means the same team can legitimately rank differently under:

- speedrun;
- comfort;
- general overworld;
- boss;
- AoE;
- resource efficiency;
- low-investment;
- multi-team account optimization.

That is a feature, not inconsistency.

---

# 20. Research versus runtime truth

The system should distinguish two knowledge flows.

## Research flow

Human-readable information is messy:

```text
official descriptions
community testing
KQM evidence
patch notes
issue threads
guide prose
```

AI can help convert that into candidate structured knowledge.

## Runtime flow

The runtime evaluator should consume only validated structured mechanics.

```text
validated character model
validated weapon model
validated artifact model
validated reaction rules
validated encounter assumptions
```

This separation is critical.

It allows AI to accelerate maintenance without letting an AI hallucination directly decide a user's recommendation.

---

# 21. Why the system should not simply wrap an LLM around Genshin guides

A general-purpose LLM with access to guides can sound persuasive but suffers from structural limitations:

- incomplete awareness of the account;
- inconsistent recall of prior state;
- inability to guarantee exhaustive team search;
- difficulty proving that all candidate teams were considered;
- unstable rankings across prompt wording;
- inability to guarantee exact arithmetic;
- post-hoc explanations;
- version drift;
- hidden assumptions.

An account-aware deterministic system solves these by representing the state explicitly.

An LLM becomes much more valuable after the engine can provide:

```json
{
  "team": [...],
  "score_vector": {...},
  "constraints": [...],
  "reason_codes": [...],
  "counterfactual_delta": {...}
}
```

The LLM's job is then presentation, not truth generation.

---

# 22. Why the proposed product is closer to a decision-support system than a game assistant

HakkoAI demonstrates the appeal of an AI coach:

- conversational;
- screen-aware;
- immediate;
- broad.

The proposed engine optimizes for a different property:

**decision traceability**.

A decision-support system should be able to answer:

- What alternatives were evaluated?
- Which alternatives were removed?
- Which objective was used?
- Which assumptions were active?
- Which rules contributed to the result?
- What changes if one assumption is removed?
- What is the sensitivity of the ranking?
- What is the opportunity cost elsewhere in the account?

This makes the proposed system useful not only for casual recommendations, but also for research and theorycrafting.

---

# 23. Sensitivity analysis

A valuable deterministic engine should not merely output rankings. It should show when rankings are fragile.

For example:

```text
Team A > Team B by 1.2%
```

may not be meaningfully different if:

- energy assumptions are uncertain;
- enemy grouping changes;
- one buff has variable uptime.

The engine should therefore support sensitivity reasoning such as:

> Team A wins under 20-second rotations, but Team B wins if the cycle extends beyond 24 seconds.

or:

> Team A is best against Freezeable enemies; Team B becomes dominant against bosses.

or:

> Mistsplitter improves Zibai personally, but the account-wide result becomes negative if Ayaka is the donor.

This is more decision-useful than a static tier.

---

# 24. Multi-team optimization

Endgame modes often require more than one team.

This turns the problem into resource allocation.

If \(T_i\) represents a candidate team and \(V(T_i)\) its value, selecting two non-overlapping teams can be formulated as:

\[
\max \sum_i x_i V(T_i)
\]

subject to:

\[
x_i \in \{0,1\}
\]

and no character or unique contested resource is used in incompatible simultaneous assignments.

This is a natural integer optimization problem.

The engine should therefore be able to discover a subtle account-level truth:

> The individually strongest team may not belong in the strongest two-team lineup because it consumes two supports that are more valuable when split across both sides.

That is precisely the kind of reasoning generic team guides rarely provide.

---

# 25. Resource allocation as a first-class concept

Characters are not the only scarce resources.

Depending on the modeled context, the engine may need to reason about:

- one-copy five-star weapons;
- shared artifact pieces;
- artifact set availability;
- support characters;
- elemental coverage;
- investment materials;
- or future farming priorities.

This opens a broader decision class:

> Which allocation of my scarce resources maximizes account utility?

That is a more general optimization problem than "what is this character's best weapon?"

---

# 26. What "good" ultimately means

The system should never encode "good" as an unexplained binary.

A team is good **for an objective under assumptions**.

Formally:

\[
\text{Good}(T) =
f(T,\ \text{account state},\ \text{objective},\ \text{encounter},\ \text{assumptions})
\]

This resolves many apparent contradictions.

An all-Cryo team may be poor under one objective and excellent under another.

A high-DPS team may be undesirable for a player prioritizing comfort.

A character may be weak in isolation but optimal because they unlock a specific party condition.

A weapon may improve one character while lowering total account value.

The engine should preserve those distinctions rather than flattening them into a universal ranking.

---

# 27. Recommended product identity

The strongest conceptual identity is:

> **A deterministic, account-aware Genshin decision engine with optional AI assistance.**

Not:

- an AI coach that happens to know Genshin;
- an artifact calculator;
- a rotation simulator;
- a generic team tier list;
- or a frontend fork of an existing optimizer.

Its defensible core is the relationship between:

```text
account state
    +
mechanics ontology
    +
constraint/search engine
    +
counterfactual dependency graph
    +
objective model
    =
explainable account-level decisions
```

Existing products validate the components:

- MondLab validates roster-aware team recommendation.
- Genshin Optimizer validates exact inventory/build optimization and the value of GOOD interoperability.
- gcsim validates deterministic combat simulation.
- KQM validates evidence-backed mechanics research.
- HakkoAI validates the demand for conversational guidance.
- Snap/Jev validate the value of bounded AI decisions for routing and classification.

The opportunity lies in combining the *deterministic decision semantics* that none of those products, individually, clearly provide end-to-end.

---

# 28. Final design principles

The system should be governed by the following principles:

**Derive what can be derived.**  
Use formulas, rules, graphs, search, and optimization before probabilistic AI.

**Keep account state structured.**  
Do not rely on conversational memory for inventory truth.

**Make objectives explicit.**  
"Best" has no meaning without an objective and encounter assumptions.

**Separate archetype from quality.**  
Describe what a team is before judging how good it is.

**Model opportunity cost.**  
Every scarce resource has a donor as well as a recipient.

**Prefer causal relationships to manually curated pair scores.**  
If A buffs B because B satisfies a formal condition, encode the condition.

**Use pairwise synergy only as an intermediate representation.**  
Team-level interactions can invalidate pairwise conclusions.

**Treat counterfactual propagation as a core capability.**  
The system should answer "what changes?" rather than merely "what is good?"

**Preserve uncertainty explicitly.**  
Do not hide encounter assumptions or qualitative judgments inside one number.

**Use AI as an adapter, not an oracle.**  
AI may interpret, extract, classify, route, and explain. Deterministic mechanics remain the source of truth.

**Keep provenance.**  
Rules should point back to evidence, game versions, or validated sources.

**Isolate trust domains.**  
Account credentials, external scanners, open-source tooling, AI services, and public mechanics data should not share unnecessary privileges.

**Design for reproducibility.**  
The same snapshot, mechanics version, objective, and assumptions should yield the same result.

---

# References

1. MondLab, **How recommendations work / scoring methodology**  
   https://mondlab.net/scoring

2. MondLab, **Roster-aware team suggestions**  
   https://mondlab.net/teams

3. Genshin Optimizer, **GitHub repository**  
   https://github.com/frzyc/genshin-optimizer

4. Genshin Optimizer, **README / architecture overview**  
   https://github.com/frzyc/genshin-optimizer/blob/master/README.md

5. gcsim, **Documentation**  
   https://docs.gcsim.app/

6. HakkoAI, **Genshin AI Coach: AI Game Tool for Smarter Strategy**  
   https://www.hakko.ai/blog/article/genshin-ai-coach/

7. KeqingMains Theorycrafting Library  
   https://library.keqingmains.com/

8. Subquadratic, **Introducing SubQ Snap**  
   https://subq.ai/introducing-subq-snap

9. Subquadratic, **SubQ Snap product overview**  
   https://subq.ai/

10. Jev, **Decision API documentation**  
    https://jev-ai.org/docs/

11. GenshinOptimizer.com, **Local-first GOOD artifact optimizer**  
    https://www.genshinoptimizer.com/
