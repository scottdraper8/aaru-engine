# GOOD interoperability reference

Inspected upstream source, not executed. This is evidence for upstream schema
shape, NOT a complete independent GOOD specification or verified combat data.
Use it to design realistic inventories without freezing remembered historical
limits or copying upstream's forgiving/coercive import behavior.

## Provenance

Genshin Optimizer, commit `c3586033db8a0e1717b494cfd845a87cf9ae79ec`:

- [Envelope](https://github.com/frzyc/genshin-optimizer/blob/c3586033db8a0e1717b494cfd845a87cf9ae79ec/libs/gi/good/src/schemas/good-format.ts)
- [Character](https://github.com/frzyc/genshin-optimizer/blob/c3586033db8a0e1717b494cfd845a87cf9ae79ec/libs/gi/schema/src/character/schema.ts)
- [Weapon](https://github.com/frzyc/genshin-optimizer/blob/c3586033db8a0e1717b494cfd845a87cf9ae79ec/libs/gi/schema/src/weapon/schema.ts)
- [Artifact](https://github.com/frzyc/genshin-optimizer/blob/c3586033db8a0e1717b494cfd845a87cf9ae79ec/libs/gi/schema/src/artifact.ts)
- [GOOD documentation and version history](https://github.com/frzyc/genshin-optimizer/blob/c3586033db8a0e1717b494cfd845a87cf9ae79ec/libs/gi/page-doc/src/index.tsx)

These four files and upstream MIT license were read in full. No upstream code
is copied into Aaru or executed. If later code/data is reused, retain its required
license/attribution. Current observations:

- Envelope: `format: "GOOD"`, `source`, `version` (1, 2 or 3 upstream); character,
  weapon and artifact arrays are independently optional. An omitted section is
  not a declaration that the player's inventory is empty.
- Character: `key`, `level`, `ascension`, `constellation`, `talent` containing
  `auto`, `skill`, `burst`. Upstream character schema accepts levels through 100;
  do not permanently assume level 90 is the current character maximum. Actual
  progression/talent relationships need separate source-backed rules.
- Weapon: `key`, `level`, `ascension`, `refinement`, `location`, `lock`. Empty
  location means unequipped. Repeated identical records are separate copies.
  Upstream permits extra weapon fields before normalization.
- Artifact: `setKey`, `slotKey`, `level`, `rarity`, `mainStatKey`, `location`,
  `lock`, `substats` (`key`, `value`, optional `initialValue`). Optional metadata
  includes `totalRolls`, `astralMark`, `elixirCrafted`, `unactivatedSubstats`.
- These inventory schemas do not supply base stats, main-stat values, passive
  formulas, kits, rotations, aura/energy assumptions or special-system rules.
- Upstream clamps/defaults some malformed values. Aaru's boundary must not copy
  this silently: reject invalid understood data, preserve supported unknown
  extensions inertly, and distinguish unsupported from evaluable inventory.
- Those four schemas alone do not settle the semantic questions below. Their
  acceptance/defaulting behavior is not a complete GOOD specification.

## Additional pinned documentary evidence

The documentation source was fetched and read in full on 2026-10-27, without
executing it. SHA-256:
`86ab1211444c0ad7aeb1aa9784d7a38c1eec4d6361d22e625da97f4f9ecfd328`.
The envelope-schema bytes hash to
`da47383dda5634946b1445bb6a78375bc43dfbb7d2ec53d59dfb9351de1fb9ed`.

- Version 1 describes characters, weapons and artifacts. Version 2 adds optional
  `materials: { [MaterialKey]: number }`; version 3 adds the artifact metadata
  listed above. The docs describe backwards compatibility. The inspected newer
  envelope schema omits materials and does not distinguish metadata by dialect.
  **GOOD can carry material balances.** Their absence is not zero or infinite;
  the current canonical materials/investment implementation remains open.
- Talents explicitly exclude constellation boosts. Apply source-backed,
  character-specific boosts once; do not assume every constellation affects the
  same talent. A permitted parser range is not progression legality.
- The percentage substat example `critDMG_: 19.4` means 19.4 percentage points,
  or fraction `0.194`; flat ATK/HP/DEF/EM do not receive that conversion. This is
  documentary intent, not proof every exporter obeys it.
- Weapon/artifact `lock` means locked in-game, not an optimization pin or an
  instruction to keep equipped. Preserve the supplied/unknown lock distinction
  and separate Aaru movement policy. Import claims are not game-server audits.
- Base weapon/artifact interfaces require no physical-copy ID. GO's own export
  extension adds IDs; inspected managers treat non-GO IDs/merges differently.
  Neither a definition key nor a self-reported exporter name proves cross-export
  copy identity. Aaru retains its explicit byte-source/position identity policy.
- `astralMark` is described as a favorite star, distinct from lock. Unactivated
  substats are not currently active. Initial values/history, crafting guarantees
  and costs must not be invented when absent or independently unverified.
- Docs allow artifact rarity 1–5, while current schema keys cover 3–5; documented
  five-star totalRolls is 3–9, while the schema accepts 0–9. Record these as profile
  discrepancies, not permission to reject valid lower-rarity data or copy coercion.
- Traveler equipment location, selectable element and gender-specific calculation
  sheet are distinct identities. Legacy GO database migrations do not establish
  unlocked forms or additional owned-character availability.

Independent exporter conformity, completeness assertions, form mapping, numeric
legality and extension/version policy remain explicit importer gates. Reject
duplicate JSON properties before a generic map or JavaScript parse erases them.
Never score a real account with fabricated mechanics. No combat/cost table or kit
is established merely by these import schemas or documentation.
