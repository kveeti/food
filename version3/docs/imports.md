# Food imports

Imports are repeatable upserts. Each run replaces aliases and nutrient values
for its source, archives source records no longer present, and leaves diary
snapshots untouched.

Canonical nutrient codes, names, units, categories, and display order belong to
the app. Importers map source fields to that registry and preserve each source's
own label in `nutrient_source_keys.source_name`. A nutrient with no values on
current foods is archived rather than deleted, so diary snapshots and existing
goals keep their stable nutrient ID. A later import can make it active again.

Set `DATABASE_URL` before running the CLI importer.

## Production administration

Production admin access requires `OIDC_ADMIN_CLAIM` and `OIDC_ADMIN_VALUE`.
`OIDC_SCOPES` may add provider-specific scopes needed for the claim. A string
claim must equal the configured value; an array claim must contain it. The role
is copied into the seven-day app session at login.

Admins use `/admin/imports` to upload a Fineli ZIP, an OFF full JSONL gzip, or
one or more OFF delta files. OFF full and delta-index URLs can be changed there.
Uploads stream to `IMPORT_DIR` and are removed after their background job. A
full URL sync parses the gzip stream directly without storing the 13 GB file.
Fineli runs only from an admin upload. OFF deltas run weekly and full OFF imports
run yearly. Both can also be started by an admin.

Delta filenames provide their start and end timestamps. Files are applied oldest
first and the cursor advances only after the database update commits. A gap
forces a full import. OFF deltas cannot report deleted products, so full imports
remain necessary. Existing foods changed to a non-Finnish or unusable record are
archived by the delta.

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

Do not copy or expand the full export locally. This command streams the
compressed JSONL file on the source host with DuckDB, writes a temporary
Finland-only JSONL file there, and streams that small file through gzip to the
Rust importer. The temporary file is removed after the stream. No source or
temporary file is written locally. The import fails before touching the
database if extraction fails or the gzip stream is incomplete.

```bash
scripts/import-open-food-facts.sh \
  192.168.40.9:openfoodfacts-products.jsonl.gz
```

The importer keeps named Finnish products with a barcode and at least one
supported nutrient. A modern as-sold `100g` or `100ml` nutrition aggregate
supplies both the basis and values. When
that aggregate is absent, the importer falls back to legacy `nutriments` values
and their `nutrition_data_per` basis. A modern aggregate takes priority when the
two forms disagree. Serving-based, prepared, and missing bases are skipped
rather than converted. Package quantity and unit never determine the basis.

Every supported nutrient from the matching aggregate is kept, not only energy
or macros. Source units are converted only through explicit compatible
conversions. Missing values remain unknown.

Open Food Facts data is available under the Open Database License. Product
images and individual contents may have separate terms; this importer does not
copy images.
