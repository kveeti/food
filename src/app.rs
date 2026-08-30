use topcoat::{
    Result,
    context::Cx,
    htmx::hx_request,
    router::{RouterBuilder, error::redirect, layout, page, request::uri},
    view::view,
};

use crate::{auth, settings};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.layout(root_layout).page(home)
}

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Result) -> Result {
    let show_navigation = uri(cx).path() != "/sign-in";
    let app = view! {
        <div id="app">
            if show_navigation {
                <nav>
                    <a
                        href="/"
                        hx-boost="true"
                        hx-target="#app"
                        hx-swap="outerHTML"
                    >
                        "Today"
                    </a>
                    <a
                        href="/settings"
                        hx-boost="true"
                        hx-target="#app"
                        hx-swap="outerHTML"
                    >
                        "Settings"
                    </a>
                    <form
                        method="post"
                        action="/logout"
                        hx-boost="true"
                        hx-target="#app"
                        hx-swap="outerHTML"
                    >
                        <button type="submit">"Log out"</button>
                    </form>
                </nav>
            }
            <div id="content">(slot?)</div>
        </div>
    }?;
    if hx_request(cx) {
        return Ok(app);
    }

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="htmx-config" content=(r#"{"extensions":"hx-optimistic"}"#)>
                <title>"Food"</title>
                <script defer="true" src="/assets/htmx-4.0.0.min.js"></script>
                <script defer="true" src="/assets/hx-optimistic.js"></script>
                <script defer="true" src="/assets/settings.js"></script>
                topcoat::dev::script()
            </head>
            <body>(app)</body>
        </html>
    }
}

#[page("/")]
#[tracing::instrument(name = "app::home", level = "debug", skip_all)]
async fn home(cx: &Cx) -> Result {
    let user = auth::require_user(cx).await?;
    if !user.has_settings() {
        let return_to = uri(cx).path_and_query().map_or("/", |value| value.as_str());
        return Err(redirect(settings::setup_url(return_to)).into());
    }

    view! {
        <main>
            <h1>"Food"</h1>
            <p>(user.email.as_deref().unwrap_or("Signed in"))</p>
        </main>
    }
}
