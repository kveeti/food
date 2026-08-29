use chrono::{DateTime, Utc};
use topcoat::{
    context::Cx,
    cookie::{Cookie, Cookies as _, SameSite, cookies, time::Duration},
};

pub(super) const SESSION_COOKIE: &str = "food_session";
pub(super) const STATE_COOKIE: &str = "food_oidc_state";
pub(super) const NONCE_COOKIE: &str = "food_oidc_nonce";
pub(super) const VERIFIER_COOKIE: &str = "food_oidc_code_verifier";
pub(super) const RETURN_COOKIE: &str = "food_oidc_return_to";

pub(super) fn set_flow_cookie(cx: &Cx, name: &'static str, value: &str, secure: bool) {
    cookies(cx).add(
        Cookie::build((name, value.to_owned()))
            .path("/")
            .http_only(true)
            .secure(secure)
            .same_site(SameSite::Lax)
            .max_age(Duration::minutes(10))
            .build(),
    );
}

pub(super) fn set_session_cookie(cx: &Cx, token: &str, expires_at: DateTime<Utc>, secure: bool) {
    let seconds = (expires_at - Utc::now()).num_seconds().max(0);
    cookies(cx).add(
        Cookie::build((SESSION_COOKIE, token.to_owned()))
            .path("/")
            .http_only(true)
            .secure(secure)
            .same_site(SameSite::Lax)
            .max_age(Duration::seconds(seconds))
            .build(),
    );
}

pub(super) fn take_cookie(cx: &Cx, name: &'static str) -> Option<String> {
    let value = cookies(cx)
        .get(name)
        .map(|cookie| cookie.value().to_owned());
    remove_cookie(cx, name);
    value
}

pub(super) fn remove_cookie(cx: &Cx, name: &'static str) {
    cookies(cx).remove(Cookie::build((name, "")).path("/").build());
}
