use chrono::{Datelike as _, Days, NaiveDate, Utc};
use chrono_tz::Tz;
use topcoat::{
    Result,
    context::Cx,
    router::{error::bad_request, query_params},
    view::{component, view},
};

#[derive(Clone, Copy)]
pub struct Day {
    pub date: NaiveDate,
    pub today: NaiveDate,
}

#[topcoat::router::query_params(error = bad_request)]
struct DayQuery {
    date: Option<String>,
}

impl Day {
    pub fn from_request(cx: &Cx, timezone: Tz) -> Result<Self> {
        let query = query_params::<DayQuery>(cx)?;
        let today = Utc::now().with_timezone(&timezone).date_naive();
        let date = query
            .date
            .as_deref()
            .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
            .transpose()
            .map_err(|_| bad_request("date must use YYYY-MM-DD"))?
            .unwrap_or(today);
        Ok(Self { date, today })
    }

    pub fn url(self) -> String {
        if self.date == self.today {
            "/".to_owned()
        } else {
            format!("/?date={}", self.date)
        }
    }

    fn adjacent_url(self, days: i64) -> Result<String> {
        let date = if days < 0 {
            self.date.checked_sub_days(Days::new(days.unsigned_abs()))
        } else {
            self.date.checked_add_days(Days::new(days as u64))
        }
        .ok_or_else(|| bad_request("date is out of range"))?;
        Ok(Self { date, ..self }.url())
    }
}

#[component]
pub async fn day_navigation(day: Day) -> Result {
    let previous = day.adjacent_url(-1)?;
    let next = day.adjacent_url(1)?;
    let eyebrow = if day.date == day.today {
        "Today".to_owned()
    } else {
        day.date.format("%A").to_string()
    };
    let date = format!(
        "{} {}, {}",
        day.date.format("%B"),
        day.date.day(),
        day.date.year()
    );

    view! {
        <header class="grid grid-cols-[2.5rem_1fr_2.5rem] items-center gap-2">
            <a
                href=(previous)
                aria-label="Previous day"
                hx-boost="true"
                hx-target="#app"
                hx-swap="outerHTML"
                class="grid size-10 place-items-center rounded-lg text-gray-600 outline-2 outline-transparent outline-offset-2 hover:bg-gray-100 focus-visible:outline-outline dark:text-gray-400 dark:hover:bg-gray-900"
            >
                <span aria-hidden="true" class="text-xl">"‹"</span>
            </a>
            <div class="text-center">
                <p class="text-sm text-gray-500 dark:text-gray-400">(eyebrow)</p>
                <h1 class="text-xl font-semibold tracking-tight">
                    <time datetime=(day.date.to_string())>(date)</time>
                </h1>
            </div>
            <a
                href=(next)
                aria-label="Next day"
                hx-boost="true"
                hx-target="#app"
                hx-swap="outerHTML"
                class="grid size-10 place-items-center rounded-lg text-gray-600 outline-2 outline-transparent outline-offset-2 hover:bg-gray-100 focus-visible:outline-outline dark:text-gray-400 dark:hover:bg-gray-900"
            >
                <span aria-hidden="true" class="text-xl">"›"</span>
            </a>
        </header>
    }
}
