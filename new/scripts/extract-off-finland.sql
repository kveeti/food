SET preserve_insertion_order = false;

COPY (
SELECT
    code,
    product_name,
    product_name_fi,
    product_name_sv,
    product_name_en,
    generic_name,
    generic_name_fi,
    generic_name_sv,
    generic_name_en,
    lang,
    brands,
    countries_tags,
    nutrition_data_per,
    nutrition,
    nutriments,
    quantity,
    product_quantity,
    product_quantity_unit
FROM read_json(
    getenv('OFF_INPUT'),
    format = 'newline_delimited',
    columns = {
        code: 'JSON',
        product_name: 'JSON',
        product_name_fi: 'JSON',
        product_name_sv: 'JSON',
        product_name_en: 'JSON',
        generic_name: 'JSON',
        generic_name_fi: 'JSON',
        generic_name_sv: 'JSON',
        generic_name_en: 'JSON',
        lang: 'JSON',
        brands: 'JSON',
        countries_tags: 'VARCHAR[]',
        nutrition_data_per: 'JSON',
        nutrition: 'JSON',
        nutriments: 'JSON',
        quantity: 'JSON',
        product_quantity: 'JSON',
        product_quantity_unit: 'JSON'
    },
    ignore_errors = true
)
WHERE list_contains(countries_tags, 'en:finland')
) TO (getenv('OFF_OUTPUT')) (
    FORMAT JSON,
    ARRAY false,
    COMPRESSION gzip
);
