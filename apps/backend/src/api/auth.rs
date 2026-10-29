use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

use crate::session::Session;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "sapa_session";
pub const LOGIN_COOKIE: &str = "sapa_login";

pub async fn require_same_origin(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if !is_mutating(req.method()) {
        return Ok(next.run(req).await);
    }
    let headers = req.headers();
    let has_auth_cookie = read_cookie(headers, SESSION_COOKIE).is_some()
        || read_cookie(headers, LOGIN_COOKIE).is_some();
    if !has_auth_cookie {
        return Ok(next.run(req).await);
    }
    let allowed: Vec<String> = state
        .config
        .cors_origins()
        .map(<[String]>::to_vec)
        .unwrap_or_default();
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok());
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let referer = headers.get(header::REFERER).and_then(|v| v.to_str().ok());
    if mutating_request_origin_allowed(origin, referer, host, &allowed) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

fn is_mutating(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

fn mutating_request_origin_allowed(
    origin: Option<&str>,
    referer: Option<&str>,
    host: Option<&str>,
    allowed_origins: &[String],
) -> bool {
    let presented = origin.or(referer);
    let Some(presented) = presented else {
        return true;
    };
    let Some(presented) = url_authority(presented) else {
        return false;
    };
    if host.and_then(url_authority).is_some_and(|h| h == presented) {
        return true;
    }
    allowed_origins
        .iter()
        .filter_map(|o| url_authority(o))
        .any(|a| a == presented)
}

fn url_authority(value: &str) -> Option<String> {
    let without_scheme = match value.split_once("://") {
        Some((_, rest)) => rest,
        None => value,
    };
    let authority = without_scheme.split(['/', '?', '#']).next()?.trim();
    (!authority.is_empty()).then(|| authority.to_ascii_lowercase())
}

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

pub fn auth_cookie(name: &str, value: &str, max_age_secs: i64, secure: bool) -> Cookie<'static> {
    let mut builder = Cookie::build((name.to_owned(), value.to_owned()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure);
    if max_age_secs >= 0 {
        builder = builder.max_age(Duration::seconds(max_age_secs));
    }
    builder.build()
}

#[cfg(test)]
#[path = "auth.test.rs"]
mod tests;
