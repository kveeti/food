use topcoat::{Result, view::view};

use crate::components::{chevron_right, input};

#[topcoat::view::component]
pub async fn select_control(name: &str, options: Result) -> Result {
    view! {
        <span class="relative block">
            <select name=(name) class=(input::SELECT)>
                (options?)
            </select>
            chevron_right(class: "pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 rotate-90 text-gray-500")
        </span>
    }
}
