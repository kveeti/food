INSERT INTO nutrients (code, display_name, unit, category, display_order)
VALUES
    ('energy', 'Energy', 'kJ', 'energy', 0),
    ('fat', 'Fat', 'g', 'macro', 10),
    ('carbohydrate', 'Carbohydrate', 'g', 'macro', 20),
    ('sugars', 'Sugars', 'g', 'macro', 21),
    ('fibre', 'Fibre', 'g', 'macro', 30),
    ('protein', 'Protein', 'g', 'macro', 40);

INSERT INTO foods (source, source_id, display_name, basis_unit, source_data)
VALUES
    ('fineli', '1', 'SOKERI', 'g', '{"fixture": true}'),
    ('fineli', '2', 'Persikka/nektariini, keskiarvo, punnittu kivineen', 'g', '{"fixture": true}'),
    ('fineli', '3', 'Maito, rasvaton, d-vitamiinia 1 ug', 'g', '{"fixture": true}'),
    ('fineli', '4', 'Kalakastike, kalamuhennos, maitopohjainen, rasvaton maito', 'g', '{"fixture": true}');

INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'SUGAR', 'en' FROM foods WHERE source = 'fineli' AND source_id = '1';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'PEACH/NECTARINE, AVERAGE, WEIGHED WITH STONE', 'en'
FROM foods WHERE source = 'fineli' AND source_id = '2';

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

INSERT INTO food_nutrients (food_id, nutrient_id, value)
SELECT foods.id, nutrients.id, 184::double precision
FROM foods
JOIN nutrients ON nutrients.code = 'energy'
WHERE foods.source = 'fineli' AND foods.source_id = '2';

INSERT INTO food_nutrients (food_id, nutrient_id, value)
SELECT foods.id, nutrients.id,
       CASE foods.source_id WHEN '3' THEN 115::double precision ELSE 527::double precision END
FROM foods
JOIN nutrients ON nutrients.code = 'energy'
WHERE foods.source = 'fineli' AND foods.source_id IN ('3', '4');
