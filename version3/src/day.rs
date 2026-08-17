use chrono::{Days, NaiveDate};
use sqlx::PgPool;
use topcoat::{
    Result,
    context::Cx,
    router::{error::bad_request, query_params},
    view::view,
};

#[derive(Debug)]
pub struct Day {
    pub date: NaiveDate,
    pub today: NaiveDate,
    pub water_open: bool,
}

#[topcoat::router::query_params(error = bad_request)]
struct DayQuery {
    date: Option<String>,
    water: Option<String>,
}

impl Day {
    pub async fn from_request(cx: &Cx, pool: &PgPool, timezone: &str) -> Result<Self> {
        let query = query_params::<DayQuery>(cx)?;
        let today = sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date")
            .bind(timezone)
            .fetch_one(pool)
            .await?;
        let date = query
            .date
            .as_deref()
            .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
            .transpose()
            .map_err(|_| bad_request("date must use YYYY-MM-DD"))?
            .unwrap_or(today);

        Ok(Self {
            date,
            today,
            water_open: query.water.as_deref() == Some("open"),
        })
    }

    pub fn url(&self) -> String {
        if self.date == self.today {
            "/".to_owned()
        } else {
            format!("/?date={}", self.date)
        }
    }

    fn adjacent_url(&self, days: i64) -> Result<String> {
        let date = if days < 0 {
            self.date.checked_sub_days(Days::new(days.unsigned_abs()))
        } else {
            self.date.checked_add_days(Days::new(days as u64))
        }
        .ok_or_else(|| bad_request("date is out of range"))?;

        Ok(if date == self.today {
            "/".to_owned()
        } else {
            format!("/?date={date}")
        })
    }
}

#[topcoat::view::component]
pub async fn day_navigation(day: &Day) -> Result {
    let previous = day.adjacent_url(-1)?;
    let next = day.adjacent_url(1)?;
    let label = if day.date == day.today {
        "Today".to_owned()
    } else {
        day.date.format("%A").to_string()
    };
    let date = day.date.format("%B %-d, %Y").to_string();

    view! {
        <header class="grid grid-cols-[2.75rem_1fr_2.75rem] items-center gap-2">
            <a
                href=(previous)
                aria-label="Previous day"
                hx-boost="true"
                class="grid size-11 place-items-center rounded-lg text-gray-700 hover:bg-gray-100 focus-visible:outline-2 focus-visible:outline-gray-500"
            >
                <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" class="size-5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="m15 18-6-6 6-6"></path>
                </svg>
            </a>
            <div class="text-center">
                <p class="text-sm text-gray-600">(label)</p>
                <h1 class="text-xl font-semibold tracking-tight text-gray-1000">(date)</h1>
            </div>
            <a
                href=(next)
                aria-label="Next day"
                hx-boost="true"
                class="grid size-11 place-items-center rounded-lg text-gray-700 hover:bg-gray-100 focus-visible:outline-2 focus-visible:outline-gray-500"
            >
                <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" class="size-5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="m9 18 6-6-6-6"></path>
                </svg>
            </a>
        </header>
    }
}
