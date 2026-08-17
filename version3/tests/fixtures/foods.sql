INSERT INTO nutrients (code, display_name, unit, category, display_order)
VALUES
    ('energy', 'Energy', 'kJ', 'energy', 0),
    ('fat', 'Fat', 'g', 'macro', 10),
    ('carbohydrate', 'Carbohydrate', 'g', 'macro', 20),
    ('sugars', 'Sugars', 'g', 'macro', 21),
    ('fibre', 'Fibre', 'g', 'macro', 30),
    ('protein', 'Protein', 'g', 'macro', 40);

INSERT INTO foods (source, source_id, display_name, basis_unit, source_data)
VALUES ('fineli', '1', 'SOKERI', 'g', '{"fixture": true}');

INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'SUGAR', 'en' FROM foods WHERE source = 'fineli' AND source_id = '1';

INSERT INTO food_nutrients (food_id, nutrient_id, value)
SELECT foods.id, nutrients.id, values.value
FROM foods
CROSS JOIN (
    VALUES
        ('energy', 1698.3::double precision),
        ('fat', 0),
        ('carbohydrate', 99.9),
        ('sugars', 99.8),
        ('fibre', 0),
        ('protein', 0)
) AS values(code, value)
JOIN nutrients ON nutrients.code = values.code
WHERE foods.source = 'fineli' AND foods.source_id = '1';
