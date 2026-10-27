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
- Per-copy identity, exact lock semantics, talent constellation-bonus treatment,
  percentage units and exporter/version semantics need explicit confirmation;
  these files alone do not settle them.

Before claiming full GOOD support: confirm version-specific specification,
representative independent exports and extension/partial-state semantics. Reject
duplicate JSON properties before a generic map or JavaScript parse erases them.
Never score a real account with fabricated mechanics.
