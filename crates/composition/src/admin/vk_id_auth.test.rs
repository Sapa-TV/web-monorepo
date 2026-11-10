use std::collections::BTreeMap;

use super::*;

#[test]
fn start_builds_pkce_authorization_url() {
    let service = VkIdAuthService::new(Some(Arc::new(VkIdConfig::fixture())));
    let url = Url::parse(&service.start_login().expect("start")).expect("url");
    let params: BTreeMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(params["code_challenge_method"], "S256");
    assert_eq!(params["response_type"], "code");
    assert_eq!(params["client_id"], "vk-client");
    assert!(params["code_challenge"].len() >= 43);
    assert!(params["device_id"].len() > 10);
}

#[test]
fn start_requires_config() {
    let service = VkIdAuthService::new(None);
    assert!(matches!(
        service.start_login(),
        Err(AdminAuthError::NotConfigured)
    ));
}

#[tokio::test]
async fn unknown_state_is_rejected() {
    let service = VkIdAuthService::new(Some(Arc::new(VkIdConfig::fixture())));
    service.start_login().expect("start");
    assert!(matches!(
        service.exchange("code", "unknown").await,
        Err(AdminAuthError::CsrfMismatch)
    ));
}

#[test]
fn pkce_challenge_matches_rfc_vector() {
    assert_eq!(
        pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}
