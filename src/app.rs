use topcoat::{
    Result,
    context::Cx,
    htmx::hx_request,
    router::{RouterBuilder, error::redirect, layout, page, request::uri},
    tailwind,
    view::{class, view},
};

use crate::{
    auth,
    components::button::{ButtonVariant, button},
    settings,
};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.layout(root_layout).page(home)
}

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Result) -> Result {
    let uri = uri(cx);
    let path = uri.path();
    let forced_settings = path == "/settings"
        && uri.query().is_some_and(|query| {
            url::form_urlencoded::parse(query.as_bytes()).any(|(key, _)| key == "return_to")
        });
    let show_navigation = path != "/sign-in";
    let app = view! {
        <div id="app" class="min-h-screen">
            if show_navigation {
                <nav class="h-9 bg-gray-100/80 text-gray-950 backdrop-blur-md dark:bg-gray-900/80 dark:text-gray-100">
                    <div class="mx-auto flex h-full max-w-xl items-stretch px-4">
                        if !forced_settings {
                            <a
                                href="/"
                                hx-boost="true"
                                hx-target="#app"
                                hx-swap="outerHTML"
                                class=(class!(
                                    "inline-flex h-full items-center px-3 text-sm text-inherit outline-2 outline-transparent outline-offset-[-2px] hover:bg-gray-200/80 focus-visible:outline-outline dark:hover:bg-gray-800/80",
                                    "underline" if path == "/",
                                ))
                            >
                                "Today"
                            </a>
                            <a
                                href="/settings"
                                hx-boost="true"
                                hx-target="#app"
                                hx-swap="outerHTML"
                                class=(class!(
                                    "inline-flex h-full items-center px-3 text-sm text-inherit outline-2 outline-transparent outline-offset-[-2px] hover:bg-gray-200/80 focus-visible:outline-outline dark:hover:bg-gray-800/80",
                                    "underline" if path == "/settings",
                                ))
                            >
                                "Settings"
                            </a>
                        }
                        <form
                            method="post"
                            action="/logout"
                            hx-boost="true"
                            hx-target="#app"
                            hx-swap="outerHTML"
                            class="ml-auto h-full"
                        >
                            button(variant: ButtonVariant::Nav, "Log out")
                        </form>
                    </div>
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
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                <script defer="true" src="/assets/htmx-4.0.0.min.js"></script>
                <script defer="true" src="/assets/hx-optimistic.js"></script>
                <script defer="true" src="/assets/settings.js"></script>
                topcoat::dev::script()
            </head>
            <body class="min-h-screen bg-gray-50 font-sans text-gray-950 antialiased dark:bg-gray-950 dark:text-gray-100">(app)</body>
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
        <main class="mx-auto max-w-xl px-7 py-10">
            <p class="mt-3 text-gray-600 dark:text-gray-400">(user.email.as_deref().unwrap_or("Signed in"))</p>
        </main>
    }
}
