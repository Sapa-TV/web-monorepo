use super::*;

#[test]
fn fixture_is_valid() {
    let config = VkIdConfig::fixture();
    assert_eq!(config.api_base_url, DEFAULT_API_BASE_URL);
}

#[test]
fn empty_client_id_is_rejected() {
    assert!(
        VkIdConfig::build(
            String::new(),
            "secret".to_string(),
            "https://localhost/cb".to_string(),
            DEFAULT_API_BASE_URL.to_string(),
            600,
        )
        .is_err()
    );
}
