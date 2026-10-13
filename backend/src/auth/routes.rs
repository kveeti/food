use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookies as _, RouterBuilderCookieExt, cookies},
    router::{
        HeaderValue, RouterBuilder, StatusCode,
        content::Form,
        error::{bad_request, see_other, unauthorized},
        header, query_params,
        request::headers,
        response::{IntoResponse, Response},
        route,
    },
};
use uuid::Uuid;

use super::{
    Auth, auth,
    cookies::{
        NONCE_COOKIE, RETURN_COOKIE, SESSION_COOKIE, STATE_COOKIE, VERIFIER_COOKIE, remove_cookie,
        set_flow_cookie, set_session_cookie, take_cookie,
    },
    data, hash, random_cookie_token,
};
use crate::data::NewSession;

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .cookies()
        .route(start_sign_in)
        .route(callback)
        .route(logout)
        .route(backchannel_logout)
}

#[topcoat::router::query_params(error = bad_request)]
struct ReturnQuery {
    return_to: Option<String>,
}

#[topcoat::router::query_params(error = bad_request)]
struct CallbackQuery {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct LogoutForm {
    logout_token: String,
}

#[route(GET "/auth/sign-in")]
#[tracing::instrument(name = "auth::start_sign_in", level = "debug", skip_all)]
async fn start_sign_in(cx: &Cx) -> Result<Response> {
    let auth = auth(cx);
    let return_to = safe_return_path(auth, query_params::<ReturnQuery>(cx)?.return_to.as_deref());
    let expected_host = auth.oidc.app_url.host_str().unwrap_or_default();
    let expected_host = match auth.oidc.app_url.port() {
        Some(port) => format!("{expected_host}:{port}"),
        None => expected_host.to_owned(),
    };
    if headers(cx)
        .get(header::HOST)
        .and_then(|host| host.to_str().ok())
        != Some(expected_host.as_str())
    {
        let mut url = auth.oidc.app_url.join("auth/sign-in")?;
        url.query_pairs_mut().append_pair("return_to", &return_to);
        return redirect(StatusCode::FOUND, url.as_str(), cx);
    }

    let authorization = auth.oidc.authorize()?;
    set_flow_cookie(cx, STATE_COOKIE, &authorization.state, auth.secure_cookies);
    set_flow_cookie(cx, NONCE_COOKIE, &authorization.nonce, auth.secure_cookies);
    set_flow_cookie(
        cx,
        VERIFIER_COOKIE,
        &authorization.code_verifier,
        auth.secure_cookies,
    );
    set_flow_cookie(cx, RETURN_COOKIE, &return_to, auth.secure_cookies);

    redirect(StatusCode::FOUND, authorization.url.as_str(), cx)
}

#[route(GET "/auth/callback")]
#[tracing::instrument(name = "auth::callback", level = "debug", skip_all)]
async fn callback(cx: &Cx) -> Result<Response> {
    let auth = auth(cx);
    let query = query_params::<CallbackQuery>(cx)?;
    let state =
        take_cookie(cx, STATE_COOKIE).ok_or_else(|| bad_request("missing OIDC state cookie"))?;
    let nonce =
        take_cookie(cx, NONCE_COOKIE).ok_or_else(|| bad_request("missing OIDC nonce cookie"))?;
    let verifier = take_cookie(cx, VERIFIER_COOKIE)
        .ok_or_else(|| bad_request("missing OIDC verifier cookie"))?;
    let return_to = safe_return_path(auth, take_cookie(cx, RETURN_COOKIE).as_deref());
    if query.state != state {
        return Err(bad_request("OIDC state mismatch").into());
    }

    let callback_url = auth.oidc.app_url.join("auth/callback")?;
    let login = auth
        .oidc
        .exchange(&callback_url, &query.code, &nonce, &verifier)
        .await
        .inspect_err(|error| tracing::warn!(?error, "OIDC callback failed"))
        .map_err(|_| unauthorized())?;
    let token = random_cookie_token();
    let token_hash = hash(&token);
    let session_id = Uuid::now_v7();
    data(cx)
        .create_session(NewSession {
            id: session_id,
            token_hash: &token_hash,
            access_token: auth
                .cipher
                .encrypt(&login.tokens.access_token, &format!("{session_id}:access"))?,
            refresh_token: auth.cipher.encrypt(
                &login.tokens.refresh_token,
                &format!("{session_id}:refresh"),
            )?,
            oidc_session_id: &login.oidc_session_id,
            refresh_expires_at: login.tokens.refresh_expires_at,
            issuer: &login.issuer,
            subject: &login.subject,
            email: login.email.as_deref(),
        })
        .await?;
    set_session_cookie(
        cx,
        &token,
        login.tokens.refresh_expires_at,
        auth.secure_cookies,
    );

    see_other(&return_to).into_response(cx)
}

#[route(POST "/logout")]
#[tracing::instrument(name = "auth::logout", level = "debug", skip_all)]
async fn logout(cx: &Cx) -> Result<Response> {
    if let Some(token) = cookies(cx).get(SESSION_COOKIE) {
        data(cx).delete_session(&hash(token.value())).await?;
    }
    remove_cookie(cx, SESSION_COOKIE);

    see_other("/sign-in").into_response(cx)
}

#[route(POST "/auth/backchannel-logout")]
#[tracing::instrument(name = "auth::backchannel_logout", level = "debug", skip_all)]
async fn backchannel_logout(cx: &Cx, Form(form): Form<LogoutForm>) -> Result<Response> {
    let logout_request = auth(cx)
        .oidc
        .verify_logout(&form.logout_token)
        .await
        .inspect_err(|error| tracing::warn!(?error, "OIDC back-channel logout rejected"))
        .map_err(|_| bad_request("invalid logout token"))?;
    data(cx)
        .delete_oidc_session(&logout_request.issuer, &logout_request.session_id)
        .await?;

    ().into_response(cx)
}

fn safe_return_path(auth: &Auth, value: Option<&str>) -> String {
    let Some(value) = value else {
        return "/".to_owned();
    };
    let Ok(url) = auth.oidc.app_url.join(value) else {
        return "/".to_owned();
    };
    if url.origin() != auth.oidc.app_url.origin() {
        return "/".to_owned();
    }
    match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_owned(),
    }
}

fn redirect(status: StatusCode, location: &str, cx: &Cx) -> Result<Response> {
    (
        status,
        [(header::LOCATION, HeaderValue::from_str(location)?)],
        (),
    )
        .into_response(cx)
}
