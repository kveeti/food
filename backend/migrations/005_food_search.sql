ALTER TABLE foods
    ADD COLUMN search_vector tsvector NOT NULL DEFAULT ''::tsvector,
    ADD COLUMN search_fi_vector tsvector NOT NULL DEFAULT ''::tsvector,
    ADD COLUMN search_sv_vector tsvector NOT NULL DEFAULT ''::tsvector,
    ADD COLUMN search_en_vector tsvector NOT NULL DEFAULT ''::tsvector;

CREATE INDEX foods_search_vector_idx ON foods USING GIN (search_vector);
CREATE INDEX foods_search_fi_vector_idx ON foods USING GIN (search_fi_vector);
CREATE INDEX foods_search_sv_vector_idx ON foods USING GIN (search_sv_vector);
CREATE INDEX foods_search_en_vector_idx ON foods USING GIN (search_en_vector);

CREATE FUNCTION refresh_food_search_vector(target_food_id uuid)
RETURNS void
LANGUAGE sql
AS $$
    UPDATE foods
    SET search_vector =
            setweight(to_tsvector('simple', replace(display_name, '/', ' ')), 'A')
            || setweight(to_tsvector('simple', COALESCE(brand, '')), 'B')
            || setweight(to_tsvector('simple', COALESCE((
                SELECT string_agg(replace(name, '/', ' '), ' ')
                FROM food_names
                WHERE food_id = target_food_id
            ), '')), 'C'),
        search_fi_vector = to_tsvector('finnish', COALESCE((
            SELECT string_agg(replace(name, '/', ' '), ' ')
            FROM food_names
            WHERE food_id = target_food_id AND locale = 'fi'
        ), '')),
        search_sv_vector = to_tsvector('swedish', COALESCE((
            SELECT string_agg(replace(name, '/', ' '), ' ')
            FROM food_names
            WHERE food_id = target_food_id AND locale = 'sv'
        ), '')),
        search_en_vector = to_tsvector('english', COALESCE((
            SELECT string_agg(replace(name, '/', ' '), ' ')
            FROM food_names
            WHERE food_id = target_food_id AND locale = 'en'
        ), ''))
    WHERE id = target_food_id;
$$;

SELECT refresh_food_search_vector(id) FROM foods;
