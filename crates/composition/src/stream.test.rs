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

#[tokio::test]
async fn reports_only_online_transitions() {
    let status = StreamStatus::new();
    status.mark_ready();
    let mut changes = status.subscribe();
    status.set_online(true);
    assert!(changes.recv().await.expect("online event"));
    status.set_online(true);
    assert!(changes.try_recv().is_err());
    status.set_online(false);
    assert!(!changes.recv().await.expect("offline event"));
}

#[test]
fn rtmp_flag_brings_online() {
    let status = StreamStatus::new();
    status.set_rtmp_online(true);
    assert!(status.is_online());
    status.set_rtmp_online(false);
    assert!(!status.is_online());
}
