use topcoat::{Result, view::view};

use crate::{
    components::{input, select_control},
    food::{LatestMeal, meal_name},
};

#[topcoat::view::component]
pub async fn meal_selector(latest_meal: Option<&LatestMeal>) -> Result {
    let meal_options = view! {
        <option value="new" selected=(latest_meal.is_none_or(|meal| !meal.active))>"Start a new meal"</option>
        if let Some(meal) = latest_meal {
            <option value=(meal.id.to_string()) selected=(meal.active)>
                "Continue " (meal_name(meal.name.as_deref())) " · " (&meal.local_time)
            </option>
        }
    };

    view! {
        <div class="mt-4 grid gap-3 sm:grid-cols-2">
            <label class="text-sm text-gray-700">
                <span class="mb-1 block">"Meal"</span>
                select_control(name: "meal", options: meal_options)
            </label>
            <label class="text-sm text-gray-700">
                <span class="mb-1 block">"New meal name (optional)"</span>
                <input
                    name="meal_name"
                    list="meal-name-suggestions"
                    maxlength="100"
                    autocomplete="off"
                    placeholder="Meal"
                    class=(input::FIELD_WITH_PLACEHOLDER)
                >
            </label>
        </div>
        <datalist id="meal-name-suggestions">
            <option value="Breakfast"></option>
            <option value="Lunch"></option>
            <option value="Dinner"></option>
            <option value="Snack"></option>
        </datalist>
    }
}
