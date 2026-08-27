INSERT INTO nutrients (code, display_name, unit, category, display_order)
VALUES
    ('energy', 'Energy', 'kJ', 'energy', 0),
    ('fat', 'Fat', 'g', 'macro', 10),
    ('carbohydrate', 'Carbs', 'g', 'macro', 20),
    ('sugars', 'Sugars', 'g', 'macro', 21),
    ('carbohydrate-by-difference', 'Carbohydrate by difference', 'g', 'carbohydrate', 22),
    ('fibre', 'Fibre', 'g', 'macro', 30),
    ('protein', 'Protein', 'g', 'macro', 40);

INSERT INTO nutrients (code, display_name, unit, category, display_order, is_archived)
VALUES ('old-nutrient', 'Old nutrient', 'g', 'other', 1000, true);

INSERT INTO foods (source, source_id, display_name, basis_unit, source_data)
VALUES
    ('fineli', '1', 'SOKERI', 'g', '{"fixture": true}'),
    ('fineli', '2', 'Persikka/nektariini, keskiarvo, punnittu kivineen', 'g', '{"fixture": true, "food_type": "FOOD", "process": "RAW"}'),
    ('fineli', '3', 'Maito, rasvaton, d-vitamiinia 1 ug', 'g', '{"fixture": true, "food_type": "FOOD", "process": "IND"}'),
    ('fineli', '4', 'Kalakastike, kalamuhennos, maitopohjainen, rasvaton maito', 'g', '{"fixture": true, "food_type": "DISH", "process": "BOIL"}'),
    ('open_food_facts', '5', 'Maitoleipä', 'g', '{"fixture": true}'),
    ('fineli', '6', 'Maitojauhe, rasvaton', 'g', '{"fixture": true, "food_type": "FOOD", "process": "DRIE"}'),
    ('fineli', '7', 'Omena, keskiarvo, punnittu kuorineen', 'g', '{"fixture": true, "food_type": "FOOD", "process": "RAW"}'),
    ('open_food_facts', '8', 'Omenasose', 'g', '{"fixture": true}'),
    ('fineli', '9', 'Taimen', 'g', '{"fixture": true, "food_type": "FOOD", "process": "RAW"}'),
    ('fineli', '10', 'Lohi', 'g', '{"fixture": true, "food_type": "FOOD", "process": "RAW"}'),
    ('fineli', '12', 'Kirjolohi', 'g', '{"fixture": true, "food_type": "FOOD", "process": "RAW"}');

INSERT INTO foods (source, source_id, display_name, brand, basis_unit, source_data)
VALUES ('open_food_facts', '11', 'Honung', 'Rainbow', 'g', '{"fixture": true}');

INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'SUGAR', 'en' FROM foods WHERE source = 'fineli' AND source_id = '1';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'SOCKER', 'sv' FROM foods WHERE source = 'fineli' AND source_id = '1';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'PEACH/NECTARINE, AVERAGE, WEIGHED WITH STONE', 'en'
FROM foods WHERE source = 'fineli' AND source_id = '2';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Milk, 0% fat, added vitamin D', 'en'
FROM foods WHERE source = 'fineli' AND source_id = '3';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Mjölk, fettfri, tillsatt vitamin D', 'sv'
FROM foods WHERE source = 'fineli' AND source_id = '3';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Apple, average, weighed with peel', 'en'
FROM foods WHERE source = 'fineli' AND source_id = '7';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Äpple, medelvärde, vägd med skal', 'sv'
FROM foods WHERE source = 'fineli' AND source_id = '7';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Trout', 'en' FROM foods WHERE source = 'fineli' AND source_id = '9';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Salmo trutta', 'sci' FROM foods WHERE source = 'fineli' AND source_id = '9';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Salmon', 'en' FROM foods WHERE source = 'fineli' AND source_id = '10';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Lax', 'sv' FROM foods WHERE source = 'fineli' AND source_id = '10';
INSERT INTO food_aliases (food_id, name, locale)
SELECT id, 'Rainbow trout', 'en' FROM foods WHERE source = 'fineli' AND source_id = '12';

INSERT INTO food_nutrients (food_id, nutrient_id, value)
SELECT foods.id, nutrients.id, values.value
FROM foods
CROSS JOIN (
    VALUES
        ('energy', 1698.3::double precision),
        ('fat', 0),
        ('carbohydrate', 99.9),
        ('sugars', 99.8),
        ('carbohydrate-by-difference', 99.88),
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
