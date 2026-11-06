use crate::api::auth::mutating_request_origin_allowed;

fn allowed(origins: &[&str]) -> Vec<String> {
    origins.iter().map(|o| o.to_string()).collect()
}

#[test]
fn passes_when_no_origin_presented() {
    assert!(mutating_request_origin_allowed(
        None,
        None,
        Some("sapa-tv.ru"),
        &allowed(&["https://sapa-tv.ru"])
    ));
}

#[test]
fn passes_when_origin_matches_host() {
    assert!(mutating_request_origin_allowed(
        Some("https://sapa-tv.ru"),
        None,
        Some("sapa-tv.ru"),
        &[]
    ));
}

#[test]
fn passes_when_origin_matches_host_with_port() {
    assert!(mutating_request_origin_allowed(
        Some("http://localhost:3000"),
        None,
        Some("localhost:3000"),
        &[]
    ));
}

#[test]
fn passes_when_origin_in_allowlist() {
    assert!(mutating_request_origin_allowed(
        Some("http://localhost:5173"),
        None,
        Some("localhost:3000"),
        &allowed(&["https://sapa-tv.ru", "http://localhost:5173"])
    ));
}

#[test]
fn falls_back_to_referer() {
    assert!(mutating_request_origin_allowed(
        None,
        Some("https://sapa-tv.ru/games?foo=1"),
        Some("backend.local"),
        &allowed(&["https://sapa-tv.ru"])
    ));
}

#[test]
fn compares_authority_case_insensitively() {
    assert!(mutating_request_origin_allowed(
        Some("HTTPS://SAPA-TV.RU"),
        None,
        Some("sapa-tv.ru"),
        &[]
    ));
}

#[test]
fn rejects_foreign_origin() {
    assert!(!mutating_request_origin_allowed(
        Some("https://evil.example"),
        None,
        Some("sapa-tv.ru"),
        &allowed(&["https://sapa-tv.ru"])
    ));
}

#[test]
fn rejects_null_origin() {
    assert!(!mutating_request_origin_allowed(
        Some("null"),
        None,
        Some("sapa-tv.ru"),
        &[]
    ));
}

#[test]
fn rejects_schemeless_foreign_value() {
    assert!(!mutating_request_origin_allowed(
        Some("evil.example"),
        None,
        Some("sapa-tv.ru"),
        &[]
    ));
}

#[test]
fn rejects_same_site_other_port_not_in_allowlist() {
    assert!(!mutating_request_origin_allowed(
        Some("http://localhost:5173"),
        None,
        Some("localhost:3000"),
        &[]
    ));
}

#[test]
fn rejects_empty_origin() {
    assert!(!mutating_request_origin_allowed(
        Some(""),
        None,
        Some("sapa-tv.ru"),
        &[]
    ));
}
