use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::http::{HeaderValue, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

use crate::session::Session;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "sapa_session";
pub const LOGIN_COOKIE: &str = "sapa_login";

pub async fn require_session(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let Some(token) = read_cookie(req.headers(), SESSION_COOKIE) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let session = state
        .session_service
        .validate_session(token.as_str())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(session);
    Ok(next.run(req).await)
}

pub async fn require_admin(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let Some(session) = req.extensions().get::<Session>().cloned() else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let is_admin = state
        .admin_service
        .is_admin(&session.twitch_user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tracing::debug!(
        "require_admin: twitch_user_id={}, is_admin={}",
        session.twitch_user_id,
        is_admin
    );
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}

pub async fn require_root(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let Some(session) = req.extensions().get::<Session>().cloned() else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let is_root = state
        .admin_service
        .is_root(&session.twitch_user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tracing::debug!(
        "require_root: twitch_user_id={}, is_root={}",
        session.twitch_user_id,
        is_root
    );
    if !is_root {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}

pub fn read_cookie(headers: &header::HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|part| {
            let part = part.trim();
            let (key, value) = part.split_once('=')?;
            (key == name).then(|| value.to_string())
        })
        .next()
        .filter(|value| !value.is_empty())
}

pub fn cookie_header(name: &str, value: &str, max_age_secs: i64, secure: bool) -> HeaderValue {
    let mut builder = Cookie::build((name, value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure);
    if max_age_secs >= 0 {
        builder = builder.max_age(Duration::seconds(max_age_secs));
    }
    HeaderValue::from_str(&builder.build().encoded().to_string()).expect("cookie header is valid")
}
