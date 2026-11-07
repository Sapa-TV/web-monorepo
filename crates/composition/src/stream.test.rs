use super::*;

#[test]
fn offline_by_default() {
    assert!(!StreamStatus::new().is_online());
}

#[test]
fn manual_flag_brings_online() {
    let status = StreamStatus::new();
    status.set_online(true);
    assert!(status.is_online());
    status.set_online(false);
    assert!(!status.is_online());
}

#[test]
fn platform_status_brings_online() {
    let status = StreamStatus::new();
    status.set_platform(PlatformId::VK_VIDEO_LIVE, true);
    assert!(status.is_online());
    status.set_platform(PlatformId::VK_VIDEO_LIVE, false);
    assert!(!status.is_online());
}

#[test]
fn any_online_platform_keeps_status_online() {
    let status = StreamStatus::new();
    status.set_platform(PlatformId::TWITCH, false);
    status.set_platform(PlatformId::VK_VIDEO_LIVE, true);
    assert!(status.is_online());
}
