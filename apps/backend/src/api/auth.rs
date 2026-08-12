use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::http::{HeaderValue, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use subtle::ConstantTimeEq;

use crate::session::Session;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "sapa_session";
pub const LOGIN_COOKIE: &str = "sapa_login";

pub async fn require_auth(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let authorized = token
        .map(|t| {
            t.as_bytes()
                .ct_eq(state.config.access_key().as_bytes())
                .into()
        })
        .unwrap_or(false);

    if authorized {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
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
    let mut cookie = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax");
    if max_age_secs >= 0 {
        cookie.push_str(&format!("; Max-Age={max_age_secs}"));
    }
    if secure {
        cookie.push_str("; Secure");
    }
    HeaderValue::from_str(&cookie).expect("cookie header is valid")
}
