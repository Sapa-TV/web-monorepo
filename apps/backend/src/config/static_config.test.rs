use super::*;

#[test]
fn split_separates_static_and_runtime() {
    let raw: RawConfig = serde_json::from_str(
        r#"{
            "port": 4321,
            "cors_origins": ["https://a.com", "https://b.com"],
            "cookie_secure": true,
            "twitch": {
                "client_id": "cid",
                "client_secret": "cs",
                "broadcaster_id": "bc",
                "redirect_uri": "https://localhost/cb",
                "credentials_redirect_uri": "https://localhost/creds/cb",
                "csrf_ttl_secs": 600
            },
            "roulette_timeout_secs": 30,
            "session_ttl_secs": 3600
        }"#,
    )
    .unwrap();

    let (static_cfg, seed) = StaticConfig::split(raw);

    assert_eq!(static_cfg.port, 4321);
    assert_eq!(
        static_cfg.cors_origins.as_deref().unwrap(),
        &["https://a.com", "https://b.com"]
    );
    assert!(static_cfg.cookie_secure);
    assert_eq!(static_cfg.twitch.as_deref().unwrap().broadcaster_id, "bc");

    let seed = seed.expect("seed present");
    assert_eq!(seed.roulette.timeout_secs, 30);
    assert_eq!(seed.session.ttl_secs, 3600);
    assert_eq!(
        seed.queue.retention_secs,
        QueueRuntimeConfig::default().retention_secs
    );
    assert!(seed.widget_access_key.is_empty());
}

#[test]
fn cors_origins_accepts_comma_string() {
    let raw: RawConfig =
        serde_json::from_str(r#"{ "cors_origins": " https://a.com , https://b.com ,, " }"#)
            .unwrap();

    let (static_cfg, _) = StaticConfig::split(raw);

    assert_eq!(
        static_cfg.cors_origins.as_deref().unwrap(),
        &["https://a.com", "https://b.com"]
    );
}

#[test]
fn admin_twitch_ids_accept_comma_string() {
    let raw: RawConfig =
        serde_json::from_str(r#"{ "admin_twitch_ids": " 111 , 222 ,, " }"#).unwrap();

    let (static_cfg, _) = StaticConfig::split(raw);

    assert_eq!(
        static_cfg.admin_twitch_ids.as_deref(),
        Some(&["111".to_string(), "222".to_string()][..])
    );
}

#[test]
fn split_passes_twitch_through() {
    let raw: RawConfig = serde_json::from_str(
        r#"{
            "twitch": {
                "client_id": "cid",
                "client_secret": "cs",
                "broadcaster_id": "bc",
                "redirect_uri": "https://localhost/cb",
                "credentials_redirect_uri": "https://localhost/creds/cb",
                "csrf_ttl_secs": 600
            }
        }"#,
    )
    .unwrap();

    let (static_cfg, _) = StaticConfig::split(raw);

    let twitch = static_cfg.twitch.expect("twitch set");
    assert_eq!(twitch.client_id, "cid");
    assert!(static_cfg.vk_video_live.is_none());
}

#[test]
fn split_passes_vk_video_live_through() {
    let raw: RawConfig = serde_json::from_str(
        r#"{
            "vk_video_live": {
                "client_id": "vk-cid",
                "client_secret": "vk-cs",
                "redirect_uri": "https://localhost/login-callback/vk-video-live",
                "credentials_redirect_uri": "https://localhost/creds-callback/vk-video-live",
                "csrf_ttl_secs": 600
            }
        }"#,
    )
    .unwrap();

    let (static_cfg, _) = StaticConfig::split(raw);

    let vk = static_cfg.vk_video_live.expect("vk set");
    assert_eq!(vk.client_id, "vk-cid");
    assert_eq!(
        vk.redirect_uri,
        "https://localhost/login-callback/vk-video-live"
    );
    assert!(static_cfg.twitch.is_none());
}
