use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, page},
    view::view,
};

use crate::auth;

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.page(home)
}

#[page("/")]
#[tracing::instrument(name = "app::home", level = "debug", skip_all)]
async fn home(cx: &Cx) -> Result {
    let user = auth::require_user(cx).await?;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Food"</title>
                topcoat::dev::script()
            </head>
            <body>
                <header>
                    <span>(user.email.as_deref().unwrap_or("Signed in"))</span>
                    <form method="post" action="/logout">
                        <button type="submit">"Log out"</button>
                    </form>
                </header>
                <main>
                    <h1>"Food"</h1>
                </main>
            </body>
        </html>
    }
}
