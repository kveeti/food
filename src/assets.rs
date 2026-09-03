use topcoat::{
    Result,
    context::Cx,
    router::{
        HeaderValue, RouterBuilder, header,
        response::{IntoResponse, Response},
        route,
    },
};

const HTMX: &[u8] = include_bytes!("../assets/htmx-4.0.0.min.js");
const HX_OPTIMISTIC: &[u8] = include_bytes!("../assets/hx-optimistic.js");
const SETTINGS: &[u8] = include_bytes!("../assets/settings.js");
const WATER: &[u8] = include_bytes!("../assets/water.js");

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .route(htmx)
        .route(hx_optimistic)
        .route(settings)
        .route(water)
}

#[route(GET "/assets/htmx-4.0.0.min.js")]
async fn htmx(cx: &Cx) -> Result<Response> {
    javascript(cx, HTMX, "public, max-age=31536000, immutable")
}

#[route(GET "/assets/hx-optimistic.js")]
async fn hx_optimistic(cx: &Cx) -> Result<Response> {
    javascript(cx, HX_OPTIMISTIC, "no-cache")
}

#[route(GET "/assets/settings.js")]
async fn settings(cx: &Cx) -> Result<Response> {
    javascript(cx, SETTINGS, "no-cache")
}

#[route(GET "/assets/water.js")]
async fn water(cx: &Cx) -> Result<Response> {
    javascript(cx, WATER, "no-cache")
}

fn javascript(cx: &Cx, body: &'static [u8], cache_control: &'static str) -> Result<Response> {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/javascript; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static(cache_control),
            ),
        ],
        body,
    )
        .into_response(cx)
}
