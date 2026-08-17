use topcoat::{
    Result,
    asset::{Asset, asset, asset_config},
    context::Cx,
    router::{HeaderValue, IntoResponse, Response, RouterBuilder, header, route},
};

pub const APP_ICON_192: Asset = asset!("public/app-icon-192.png");
const APP_ICON_512: Asset = asset!("public/app-icon-512.png");
const WORKBOX_CORE: Asset = asset!(
    "https://cdn.jsdelivr.net/npm/workbox-core@7.4.1/build/workbox-core.prod.js",
    checksum: "sha256:e5fe645c1c2019c0e32b5a863a03b484db2311d17df443704c025babc0bb1f4f"
);
const WORKBOX_ROUTING: Asset = asset!(
    "https://cdn.jsdelivr.net/npm/workbox-routing@7.4.1/build/workbox-routing.prod.js",
    checksum: "sha256:844e227e94979b17caa529a90bead72541c32985d91f1e17ed2589a741cf7647"
);
const WORKBOX_STRATEGIES: Asset = asset!(
    "https://cdn.jsdelivr.net/npm/workbox-strategies@7.4.1/build/workbox-strategies.prod.js",
    checksum: "sha256:c4139ddc2510aea347456a8ed167f820683492561394c64fd1eca5827cd219d3"
);

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(manifest).route(service_worker)
}

#[route(GET "/manifest.webmanifest")]
async fn manifest(cx: &Cx) -> Result<Response> {
    let assets = asset_config(cx);
    let body = format!(
        r##"{{
  "name": "Food",
  "short_name": "Food",
  "description": "A fast personal food and activity diary",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#f8f7f4",
  "theme_color": "#f8f7f4",
  "icons": [
    {{
      "src": "{icon_192}",
      "sizes": "192x192",
      "type": "image/png",
      "purpose": "any maskable"
    }},
    {{
      "src": "{icon_512}",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any maskable"
    }}
  ]
}}"##,
        icon_192 = assets.resolve(APP_ICON_192),
        icon_512 = assets.resolve(APP_ICON_512),
    );

    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/manifest+json"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        body,
    )
        .into_response(cx)
}

#[route(GET "/service-worker.js")]
async fn service_worker(cx: &Cx) -> Result<Response> {
    let assets = asset_config(cx);
    let body = format!(
        r#"importScripts("{core}", "{routing}", "{strategies}");

self.skipWaiting();
workbox.core.clientsClaim();

workbox.routing.registerRoute(
    ({{request}}) => request.mode === "navigate",
    new workbox.strategies.NetworkOnly()
);

workbox.routing.registerRoute(
    ({{request, url}}) => request.method === "GET" && url.origin === self.location.origin &&
        (url.pathname.startsWith("/_topcoat/assets/") || url.pathname.startsWith("/_topcoat/fonts/")),
    new workbox.strategies.CacheFirst({{cacheName: "food-public-assets-v1"}})
);
"#,
        core = assets.resolve(WORKBOX_CORE),
        routing = assets.resolve(WORKBOX_ROUTING),
        strategies = assets.resolve(WORKBOX_STRATEGIES),
    );

    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/javascript; charset=utf-8"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
        ],
        body,
    )
        .into_response(cx)
}
