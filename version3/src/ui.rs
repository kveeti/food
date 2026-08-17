use topcoat::{Result, view::view};

#[topcoat::view::component]
pub async fn search_spinner() -> Result {
    view! {
        <span class="search-spinner" aria-hidden="true">
            <span class="spinner">
                <span class="spinner-leaf"></span>
                <span class="spinner-leaf"></span>
                <span class="spinner-leaf"></span>
                <span class="spinner-leaf"></span>
                <span class="spinner-leaf"></span>
                <span class="spinner-leaf"></span>
                <span class="spinner-leaf"></span>
                <span class="spinner-leaf"></span>
            </span>
        </span>
    }
}
