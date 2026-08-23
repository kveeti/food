# Food imports

Imports are repeatable upserts. Each run replaces aliases and nutrient values
for its source, archives source records no longer present, and leaves diary
snapshots untouched.

Canonical nutrient codes, names, units, categories, and display order belong to
the app. Importers map source fields to that registry and preserve each source's
own label in `nutrient_source_keys.source_name`.

Set `DATABASE_URL` before running either importer.

## Fineli

The importer reads release 20 CSV files as Windows-1252, handles semicolon
separators and decimal commas, and imports all 74 components.

```bash
cargo run --features import-tools --bin import-foods -- \
  fineli ~/Downloads/Fineli_rel20_74
```

Fineli release 20 is licensed under CC BY 4.0 by the Finnish Institute for
Health and Welfare (THL).

## Open Food Facts

The evaluated export contains 4,535,553 products and is about 1.2 GB compressed
and 13 GB expanded. Do not copy or expand it locally.

This command streams the compressed file over SSH, decompresses it into a pipe,
and filters it while parsing. The Rust importer keeps only named Finnish
products with a barcode and at least one numeric `_100g` nutrient. No local
source or temporary file is created. The full decompressed data never resides
on disk or in memory at once.

```bash
scripts/import-open-food-facts.sh \
  192.168.40.9:en.openfoodfacts.org.products.csv.gz
```

The importer keeps every supported nutrient present on each selected product,
not only energy or macros. Missing values remain unknown. The evaluated run
selected 3,734 products and 50,922 nutrient values. Together with Fineli, the
local database used about 66 MB after import. The streamed OFF run took about
two minutes and used no temporary disk space.

Open Food Facts data is available under the Open Database License. Product
images and individual contents may have separate terms; this importer does not
copy images.
