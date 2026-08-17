# Food data model

## Food definitions

`foods` stores imported and personal foods in one shape.

An imported food is public. It has a source (`fineli` or
`open_food_facts`), a source ID, and no owner. A custom food has source
`custom`, an owner, and no source ID. Imported source IDs are unique within
their source.

Every food has exactly one nutrition basis:

- `g`: values are per 100 g
- `ml`: values are per 100 ml
- `count`: values are per one item

The basis does not change after creation. The app does not infer mass, volume,
or count conversions. Imported package and serving descriptions do not become
logging units.

Foods are archived instead of deleted. This keeps favourites, shortcuts, and
old diary references valid.

`food_aliases` stores translated and alternate search names. `display_name`
stays on `foods` so normal rendering needs no alias join. PostgreSQL maintains a
weighted `tsvector` from the name, brand, and aliases with Finnish, Swedish,
English, and simple dictionaries. Search uses its GIN index, language stemming,
and token-prefix queries for typeahead.

## Nutrients

`nutrients` is a controlled canonical nutrient list. Each nutrient has a stable
code, display name, category, order, and canonical unit. Energy uses kJ. Mass
values use g internally; the UI may display mg or µg.

`nutrient_source_keys` records how Fineli component IDs and Open Food Facts
column names map to canonical nutrients. The import code performs only explicit
compatible conversions.

`food_nutrients` stores one value per food and canonical nutrient. A missing row
means unknown. A stored zero means the source explicitly supplied zero.

Relevant source fields are retained in `foods.source_data`. This lets a later
importer version remap source values without treating unknown values as zero.

## Shortcuts

`food_shortcuts` belongs to one user and one food. A shortcut stores a name and
an exact positive amount in the food's existing basis unit. The unit is derived
from the food and is not stored twice.

Examples:

- `slice` = 34 g
- `my glass` = 320 ml
- `half` = 0.5 count

A shortcut only fills the amount. The diary records the canonical amount and
unit, not an opaque serving description.

## Diary snapshots

`food_entries` stores the consumed amount and a snapshot of the food identity,
name, brand, source, and basis. Its optional food link may be cleared without
invalidating the entry.

`food_entry_nutrients` stores both the source basis value and the calculated
consumed value. Editing an entry amount can recalculate from its original
snapshot without reading the current food definition. Daily totals group and
sum consumed values by nutrient.

Changing an imported or custom food never rewrites old entries.

## Source rules

Fineli is imported as a complete public catalog with Finnish display names and
Finnish, Swedish, English, and scientific aliases. Imported all-uppercase names
are stored in sentence case while their exact source text remains in
`source_data`. Its values are per 100 g.

Open Food Facts is imported only for products tagged `en:finland` that have a
barcode, a product name, and at least one supported numeric nutrient. The TSV
export exposes `_100g` values, so these foods use a 100 g basis. The importer
does not infer a 100 ml basis from a package, category, or serving description.

Fineli and Open Food Facts records are not merged. Source updates change the
current food definition while diary snapshots stay unchanged.
