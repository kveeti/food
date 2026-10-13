ALTER TABLE meals
    ADD CONSTRAINT meals_user_id_id_key UNIQUE (user_id, id);

ALTER TABLE food_entries
    DROP CONSTRAINT food_entries_meal_id_fkey;

ALTER TABLE food_entries
    ADD CONSTRAINT food_entries_meal_owner_fkey
    FOREIGN KEY (user_id, meal_id)
    REFERENCES meals (user_id, id)
    ON DELETE SET NULL (meal_id);
