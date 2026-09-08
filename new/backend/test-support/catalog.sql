INSERT INTO foods (id, source, source_id, display_name, brand, basis_unit)
VALUES ('00000000-0000-4000-8000-000000000001', 'fineli', '1', 'Apple', NULL, 'g'),
       ('00000000-0000-4000-8000-000000000002', 'open_food_facts', '2', 'Apple juice', 'Orchard', 'ml');

INSERT INTO food_names (food_id, name, locale)
VALUES ('00000000-0000-4000-8000-000000000001', 'Omena', 'fi');

INSERT INTO food_nutrients (food_id, nutrient_id, value)
SELECT f.id, n.id, CASE n.code WHEN 'energy' THEN 209.2 ELSE 2 END
FROM foods f CROSS JOIN nutrients n WHERE n.code IN ('energy', 'protein');

SELECT refresh_food_search_vector(id) FROM foods;
