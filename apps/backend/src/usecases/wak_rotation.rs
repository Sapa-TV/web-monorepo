use axum::http::StatusCode;
use serde_json::json;
use std::slice::from_ref;

use super::bearer;
use super::cookie;
use super::get_json;
use super::post_json;
use super::wapi_path;
use crate::test_fixtures::api_path;
use crate::test_fixtures::session_cookie;
use crate::test_fixtures::test_router;
use crate::test_fixtures::test_state;

#[tokio::test]
async fn wak_rotation_invalidates_previous_generations() {
    let state = test_state().await;
    state.admin_service.seed("100").await.unwrap();
    let admin_cookie = cookie(&session_cookie(&state, "100").await);
    let app = test_router(state.clone());

    let (status, body) = get_json(
        app.clone(),
        &api_path("/admin/widget-access-key"),
        from_ref(&admin_cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let first_key = body["widget_access_key"].as_str().expect("key").to_string();

    let (status, _) = get_json(app.clone(), &wapi_path("/queue"), &[bearer(&first_key)]).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = post_json(
        app.clone(),
        &api_path("/admin/widget-access-key"),
        from_ref(&admin_cookie),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let second_key = body["widget_access_key"].as_str().expect("key").to_string();
    assert_ne!(second_key, first_key);

    let (status, _) = get_json(app.clone(), &wapi_path("/queue"), &[bearer(&first_key)]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = get_json(app.clone(), &wapi_path("/queue"), &[bearer(&second_key)]).await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = get_json(
        app.clone(),
        &api_path("/admin/widget-access-key"),
        from_ref(&admin_cookie),
    )
    .await;
    assert_eq!(body["widget_access_key"], second_key.as_str());

    let (_, body) = post_json(
        app.clone(),
        &api_path("/admin/widget-access-key"),
        &[admin_cookie],
        json!({}),
    )
    .await;
    let third_key = body["widget_access_key"].as_str().expect("key").to_string();
    assert_ne!(third_key, second_key);

    let (status, _) = get_json(app.clone(), &wapi_path("/queue"), &[bearer(&second_key)]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = get_json(app.clone(), &wapi_path("/queue"), &[bearer(&third_key)]).await;
    assert_eq!(status, StatusCode::OK);
}
