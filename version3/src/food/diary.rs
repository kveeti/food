use super::*;
use crate::components::{button, chevron_right, input};

pub(super) fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(update_entry).route(delete_entry)
}

pub(super) async fn load_day(
    pool: &PgPool,
    user_id: Uuid,
    date: NaiveDate,
    timezone: &str,
) -> Result<(Vec<Meal>, DailyTotals)> {
    type EntryRow = (
        Uuid,
        Option<Uuid>,
        Option<String>,
        String,
        String,
        Option<String>,
        f64,
        String,
        DateTime<Utc>,
        String,
        Option<f64>,
    );

    let (entry_rows, total_rows): (Vec<EntryRow>, Vec<(String, f64, bool)>) =
        tokio::try_join!(
            sqlx::query_as(
                "SELECT entries.id, entries.meal_id, meals.name,
                        to_char(COALESCE(meals.started_at, entries.eaten_at) AT TIME ZONE $3, 'HH24:MI'),
                        entries.food_name, entries.food_brand, entries.amount, entries.unit,
                        entries.eaten_at,
                        to_char(entries.eaten_at AT TIME ZONE $3, 'HH24:MI'),
                        energy.consumed_value / 4.184
                 FROM food_entries entries
                 LEFT JOIN meals ON meals.id = entries.meal_id AND meals.user_id = entries.user_id
                 LEFT JOIN nutrients energy_name ON energy_name.code = 'energy'
                 LEFT JOIN food_entry_nutrients energy
                        ON energy.food_entry_id = entries.id
                       AND energy.nutrient_id = energy_name.id
                 WHERE entries.user_id = $1
                   AND entries.eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
                   AND entries.eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
                 ORDER BY entries.meal_id IS NULL, meals.started_at, entries.eaten_at, entries.created_at",
            )
            .bind(user_id)
            .bind(date)
            .bind(timezone)
            .fetch_all(pool),
            sqlx::query_as(
                "WITH day_entries AS (
                     SELECT id
                     FROM food_entries
                     WHERE user_id = $1
                       AND eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
                       AND eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
                 )
                 SELECT nutrients.code,
                        COALESCE(SUM(values.consumed_value), 0),
                        COUNT(values.food_entry_id) = (SELECT COUNT(*) FROM day_entries)
                 FROM nutrients
                 LEFT JOIN day_entries ON true
                 LEFT JOIN food_entry_nutrients values
                        ON values.food_entry_id = day_entries.id
                       AND values.nutrient_id = nutrients.id
                 WHERE nutrients.code = ANY($4)
                 GROUP BY nutrients.id, nutrients.code",
            )
            .bind(user_id)
            .bind(date)
            .bind(timezone)
            .bind(["energy", "protein", "carbohydrate", "fat", "fibre"])
            .fetch_all(pool),
        )?;

    let mut meals: Vec<Meal> = Vec::new();
    for row in entry_rows {
        if meals.last().is_none_or(|meal| meal.id != row.1) {
            meals.push(Meal {
                id: row.1,
                name: row.2.clone(),
                local_time: row.3.clone(),
                entries: Vec::new(),
                energy_kcal: 0.0,
                energy_complete: true,
            });
        }
        let meal = meals.last_mut().expect("meal was just added");
        if let Some(energy) = row.10 {
            meal.energy_kcal += energy;
        } else {
            meal.energy_complete = false;
        }
        meal.entries.push(FoodEntry {
            id: row.0,
            name: row.4,
            brand: row.5,
            amount: row.6,
            unit: row.7,
            eaten_at: row.8,
            local_time: row.9,
            energy_kcal: row.10,
        });
    }

    let total = |code: &str, energy: bool| {
        let row = total_rows.iter().find(|row| row.0 == code);
        NutrientTotal {
            value: row.map_or(0.0, |row| if energy { row.1 / 4.184 } else { row.1 }),
            complete: row.is_none_or(|row| row.2),
        }
    };
    let totals = DailyTotals {
        energy: total("energy", true),
        protein: total("protein", false),
        carbohydrate: total("carbohydrate", false),
        fat: total("fat", false),
        fibre: total("fibre", false),
    };

    Ok((meals, totals))
}

#[path_param(error = bad_request)]
struct EntryId(Uuid);

#[derive(Debug, serde::Deserialize)]
struct UpdateEntryForm {
    amount: String,
    date: String,
}

#[route(POST "/food-entries/{entry_id}")]
async fn update_entry(cx: &Cx, Form(input): Form<UpdateEntryForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    user.timezone.ok_or_redirect("/settings?required=1")?;
    let entry_id = path_param::<EntryId>(cx)?;
    let amount = parse_required_amount(&input.amount)?;
    let date = parse_date(&input.date)?;
    let mut transaction = db(cx).begin().await?;
    let unit: Option<String> = sqlx::query_scalar(
        "UPDATE food_entries
         SET amount = $1
         WHERE id = $2 AND user_id = $3
         RETURNING unit",
    )
    .bind(amount)
    .bind(*entry_id)
    .bind(user.id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(unit) = unit else {
        return Err(not_found().into());
    };
    let factor = if unit == "count" {
        amount
    } else {
        amount / 100.0
    };
    sqlx::query(
        "UPDATE food_entry_nutrients
         SET consumed_value = basis_value * $1
         WHERE food_entry_id = $2",
    )
    .bind(factor)
    .bind(*entry_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    redirect_to_day(cx, date)
}

#[derive(Debug, serde::Deserialize)]
struct DeleteEntryForm {
    date: String,
}

#[route(POST "/food-entries/{entry_id}/delete")]
async fn delete_entry(cx: &Cx, Form(input): Form<DeleteEntryForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    user.timezone.ok_or_redirect("/settings?required=1")?;
    let entry_id = path_param::<EntryId>(cx)?;
    let date = parse_date(&input.date)?;
    let mut transaction = db(cx).begin().await?;
    let meal_id: Option<Uuid> = sqlx::query_scalar(
        "DELETE FROM food_entries
         WHERE id = $1 AND user_id = $2
         RETURNING meal_id",
    )
    .bind(*entry_id)
    .bind(user.id)
    .fetch_optional(&mut *transaction)
    .await?
    .flatten();
    if let Some(meal_id) = meal_id {
        sqlx::query(
            "DELETE FROM meals
             WHERE id = $1 AND user_id = $2
               AND NOT EXISTS (
                   SELECT 1 FROM food_entries WHERE meal_id = meals.id
               )",
        )
        .bind(meal_id)
        .bind(user.id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;

    redirect_to_day(cx, date)
}

#[topcoat::view::component]
pub(super) async fn daily_totals(totals: &DailyTotals) -> Result {
    let incomplete = !totals.energy.complete
        || !totals.protein.complete
        || !totals.carbohydrate.complete
        || !totals.fat.complete
        || !totals.fibre.complete;

    view! {
        <div aria-label="Daily nutrition totals" class="mt-6 grid grid-cols-3 gap-x-3 gap-y-3 border-y border-gray-200 py-3">
            daily_total(label: "Energy", total: &totals.energy, unit: "kcal")
            daily_total(label: "Protein", total: &totals.protein, unit: "g")
            daily_total(label: "Carbs", total: &totals.carbohydrate, unit: "g")
            daily_total(label: "Fat", total: &totals.fat, unit: "g")
            daily_total(label: "Fibre", total: &totals.fibre, unit: "g")
            if incomplete {
                <p class="self-end text-xs text-gray-500">"* Incomplete"</p>
            }
        </div>
    }
}

#[topcoat::view::component]
async fn daily_total(label: &str, total: &NutrientTotal, unit: &str) -> Result {
    view! {
        <p>
            <span class="block text-xs text-gray-500">(label)</span>
            <strong class="text-sm font-medium tabular-nums">(total_text(total, unit))</strong>
        </p>
    }
}

#[topcoat::view::component]
pub(super) async fn food_diary(meals: &[Meal], date: NaiveDate) -> Result {
    view! {
        <div id="food-diary" class="mt-5">
            if meals.is_empty() {
                <p class="py-3 text-center text-sm text-gray-500">"No food logged"</p>
            } else {
                for meal in meals {
                    <section class="border-b border-gray-200 py-3 first:border-t" aria-label=(meal_name(meal.name.as_deref()))>
                        <header class="mb-1 flex items-start justify-between gap-4">
                            <div>
                                <h3 class="text-sm font-medium text-gray-900">
                                    (meal_name(meal.name.as_deref())) " · " (&meal.local_time)
                                </h3>
                                <p class="text-xs tabular-nums text-gray-500">
                                    (format_number(meal.energy_kcal)) " kcal"
                                    if !meal.energy_complete { "*" }
                                </p>
                            </div>
                            if let Some(meal_id) = meal.id {
                                <a
                                    href=(meals::meal_url(meal_id, date))
                                    hx-get=(meals::meal_preview_url(meal_id, date))
                                    hx-push-url=(meals::meal_url(meal_id, date))
                                    hx-target="#food-preview"
                                    hx-swap="outerHTML show:top showTarget:#food-preview"
                                    aria-label=(format!("Copy {} meal", meal_name(meal.name.as_deref())))
                                    class="grid size-8 shrink-0 place-items-center rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700"
                                >
                                    <svg aria-hidden="true" width="15" height="15" viewBox="0 0 15 15" fill="none" xmlns="http://www.w3.org/2000/svg">
                                        <path d="M1 9.50006C1 10.3285 1.67157 11.0001 2.5 11.0001H4L4 10.0001H2.5C2.22386 10.0001 2 9.7762 2 9.50006L2 2.50006C2 2.22392 2.22386 2.00006 2.5 2.00006L9.5 2.00006C9.77614 2.00006 10 2.22392 10 2.50006V4.00002H5.5C4.67158 4.00002 4 4.67159 4 5.50002V12.5C4 13.3284 4.67158 14 5.5 14H12.5C13.3284 14 14 13.3284 14 12.5V5.50002C14 4.67159 13.3284 4.00002 12.5 4.00002H11V2.50006C11 1.67163 10.3284 1.00006 9.5 1.00006H2.5C1.67157 1.00006 1 1.67163 1 2.50006V9.50006ZM5 5.50002C5 5.22388 5.22386 5.00002 5.5 5.00002H12.5C12.7761 5.00002 13 5.22388 13 5.50002V12.5C13 12.7762 12.7761 13 12.5 13H5.5C5.22386 13 5 12.7762 5 12.5V5.50002Z" fill="currentColor" fill-rule="evenodd" clip-rule="evenodd"></path>
                                    </svg>
                                </a>
                            }
                        </header>
                        <ul>
                            for entry in &meal.entries {
                                food_entry(entry: entry, date: date)
                            }
                        </ul>
                    </section>
                }
            }
        </div>
    }
}

#[topcoat::view::component]
async fn food_entry(entry: &FoodEntry, date: NaiveDate) -> Result {
    let amount = format!(
        "{} {}",
        format_amount(entry.amount),
        entry_unit(&entry.unit)
    );
    let energy = entry
        .energy_kcal
        .map(|value| format!("{} kcal", format_number(value)))
        .unwrap_or_else(|| "— kcal".to_owned());
    let delete_dialog_id = format!("delete-food-entry-{}", entry.id);

    view! {
        <li class="border-t border-gray-200 first:border-t-0">
            <details class="group">
                <summary class="flex min-h-11 cursor-pointer list-none items-center justify-between gap-3 py-2">
                    <span class="min-w-0">
                        <span class="block truncate text-sm text-gray-900">(&entry.name)</span>
                        if let Some(brand) = &entry.brand {
                            <span class="block truncate text-xs text-gray-500">(brand)</span>
                        }
                    </span>
                    <span class="flex shrink-0 items-center gap-3 text-right text-xs tabular-nums text-gray-500">
                        <span>
                            <span class="block">(amount)</span>
                            <span class="block">(energy)</span>
                        </span>
                        <time datetime=(entry.eaten_at.to_rfc3339())>(&entry.local_time)</time>
                        chevron_right(class: "shrink-0 transition-transform group-open:rotate-90")
                    </span>
                </summary>
                <div class="flex items-stretch gap-2 pb-3">
                    <form
                        method="post"
                        action=(format!("/food-entries/{}", entry.id))
                        hx-boost="true"
                        class="flex min-w-0 flex-1 items-stretch gap-2"
                    >
                        <input type="hidden" name="date" value=(date.to_string())>
                        <label class="min-w-0 flex-1">
                            <span class="sr-only">"Amount in " (unit_name(&entry.unit))</span>
                            <input
                                name="amount"
                                type="number"
                                inputmode="decimal"
                                min="0"
                                max="100000"
                                step="any"
                                required="true"
                                value=(entry.amount)
                                class=(input::FIELD)
                            >
                        </label>
                        <button type="submit" class=(format!("self-stretch {}", button::OUTLINE))>"Save"</button>
                    </form>
                    <button
                        type="button"
                        commandfor=(&delete_dialog_id)
                        command="show-modal"
                        aria-label=(format!("Delete {}", entry.name))
                        class="grid w-11 place-items-center rounded-lg border border-gray-200 text-gray-500 hover:bg-danger-surface hover:text-danger-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-danger-text"
                    >
                        <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" class="size-4">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>
            </details>

            <dialog
                id=(&delete_dialog_id)
                aria-labelledby=(format!("{delete_dialog_id}-title"))
                class="food-delete-dialog w-[calc(100%_-_1.5rem)] max-w-sm rounded-xl border border-gray-200 bg-canvas p-0 text-gray-950 shadow-xl"
            >
                <div class="p-4">
                    <h2 id=(format!("{delete_dialog_id}-title")) class="text-base font-semibold text-gray-1000">"Delete food?"</h2>
                    <p class="mt-1 text-sm text-gray-600">"Remove " (&entry.name) " from this meal?"</p>
                    <div class="mt-5 flex justify-end gap-2">
                        <form method="dialog" data-close-dialog-form="true">
                            <button type="submit" class=(button::GHOST)>"No, cancel"</button>
                        </form>
                        <form
                            method="post"
                            action=(format!("/food-entries/{}/delete", entry.id))
                            hx-boost="true"
                            data-delete-dialog-form="true"
                        >
                            <input type="hidden" name="date" value=(date.to_string())>
                            <button
                                type="submit"
                                class="rounded-lg border border-gray-200 px-3 py-2 text-sm font-medium text-gray-800 hover:bg-danger-surface hover:text-danger-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-danger-text"
                            >
                                "Yes, delete"
                            </button>
                        </form>
                    </div>
                </div>
            </dialog>
            <div aria-hidden="true" class="food-delete-backdrop"></div>
        </li>
    }
}
