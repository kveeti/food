use topcoat::{Result, view::view};

#[topcoat::view::component]
pub async fn daily_progress(
    food_kcal: f64,
    food_complete: bool,
    food_goal_kcal: Option<f64>,
    water_ml: i64,
    water_goal_ml: Option<i64>,
) -> Result {
    view! {
        <div class="mt-5 grid gap-2 sm:grid-cols-2">
            food_progress(consumed: food_kcal, complete: food_complete, goal: food_goal_kcal)
            water_progress(consumed: water_ml, goal: water_goal_ml, swap_oob: false)
        </div>
    }
}

#[topcoat::view::component]
async fn food_progress(consumed: f64, complete: bool, goal: Option<f64>) -> Result {
    let consumed = consumed.max(0.0);
    let (goal_width, over_width, maximum) = goal
        .map(|goal| progress_segments(consumed, goal))
        .unwrap_or((0.0, 0.0, consumed.max(1.0)));
    let progress = goal.map(|goal| progress_text(consumed, goal, "kcal"));
    let amount = progress.as_ref().map_or_else(
        || format!("{} kcal / no goal", number(consumed)),
        |progress| progress.0.clone(),
    );

    view! {
        <div
            id="food-goal-progress"
            role="progressbar"
            aria-label="Food energy progress"
            aria-valuemin="0"
            aria-valuemax=(maximum)
            aria-valuenow=(consumed)
            class="relative isolate overflow-hidden rounded-xl bg-food-progress px-4 py-3"
        >
            if goal_width > 0.0 {
                <div aria-hidden="true" class="absolute inset-y-0 left-0 -z-10 bg-food-fill" style=(format!("width:{goal_width:.3}%"))></div>
            }
            if over_width > 0.0 {
                <div aria-hidden="true" data-goal-over="true" class="absolute inset-y-0 -z-10 border-l-[3px] border-food-progress bg-[repeating-linear-gradient(135deg,var(--food-fill)_0_6px,var(--food-over)_6px_10px)]" style=(format!("left:{goal_width:.3}%;width:{over_width:.3}%"))></div>
            }
            <p class="text-lg font-semibold tabular-nums">
                (amount)
                if !complete { "*" }
            </p>
            if let Some(progress) = &progress {
                <p class="mt-0.5 text-xs tabular-nums">(&progress.1)</p>
            }
            if !complete {
                <p class="mt-1 text-xs">"* Some food energy is unavailable"</p>
            }
        </div>
    }
}

#[topcoat::view::component]
pub async fn water_progress(consumed: i64, goal: Option<i64>, swap_oob: bool) -> Result {
    let consumed = consumed.max(0) as f64;
    let goal = goal.map(|goal| goal as f64);
    let (goal_width, over_width, maximum) = goal
        .map(|goal| progress_segments(consumed, goal))
        .unwrap_or((0.0, 0.0, consumed.max(1.0)));
    let progress = goal.map(|goal| progress_text(consumed, goal, "ml"));
    let amount = progress.as_ref().map_or_else(
        || format!("{} ml / no goal", number(consumed)),
        |progress| progress.0.clone(),
    );

    view! {
        <div
            id="water-goal-progress"
            hx-swap-oob=(swap_oob.then_some("outerHTML"))
            role="progressbar"
            aria-label="Water progress"
            aria-valuemin="0"
            aria-valuemax=(maximum)
            aria-valuenow=(consumed)
            class="relative isolate overflow-hidden rounded-xl bg-water-progress px-4 py-3"
        >
            if goal_width > 0.0 {
                <div aria-hidden="true" class="absolute inset-y-0 left-0 -z-10 bg-water-fill" style=(format!("width:{goal_width:.3}%"))></div>
            }
            if over_width > 0.0 {
                <div aria-hidden="true" data-goal-over="true" class="absolute inset-y-0 -z-10 border-l-[3px] border-water-progress bg-[repeating-linear-gradient(135deg,var(--water-fill)_0_6px,var(--water-over)_6px_10px)]" style=(format!("left:{goal_width:.3}%;width:{over_width:.3}%"))></div>
            }
            <p class="text-lg font-semibold tabular-nums">(amount)</p>
            if let Some(progress) = &progress {
                <p class="mt-0.5 text-xs tabular-nums">(&progress.1)</p>
            }
        </div>
    }
}

fn progress_segments(consumed: f64, goal: f64) -> (f64, f64, f64) {
    let maximum = consumed.max(goal);
    let goal_width = consumed.min(goal) / maximum * 100.0;
    let over_width = (consumed - goal).max(0.0) / maximum * 100.0;
    (goal_width, over_width, maximum)
}

fn progress_text(consumed: f64, goal: f64, unit: &str) -> (String, String) {
    let status = if consumed <= goal {
        format!("{} {unit} left", number(goal - consumed))
    } else {
        format!("{} {unit} over", number(consumed - goal))
    };
    let progress = format!("{} / {} {unit}", number(consumed), number(goal));
    (status, progress)
}

fn number(value: f64) -> String {
    let rounded = value.round() as i64;
    let digits = rounded.abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(character);
    }
    if rounded < 0 {
        grouped.insert(0, '-');
    }
    grouped
}
