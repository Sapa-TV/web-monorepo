use std::sync::Arc;
use std::time::Duration;

use crate::admin::service::AdminService;
use crate::config::settings::SharedSettings;
use crate::error::SessionServiceError;
use crate::session::account::AccountService;
use crate::session::service::SessionService;
use crate::user::service::UserService;
use model::config::runtime::RuntimeConfig;
use model::platform::PlatformId;
use model::user::repository::UserRepository;
use storage_inmemory::inmemory_admin::InMemoryAdminRepository;
use storage_inmemory::inmemory_platform::InMemoryPlatformRepository;
use storage_inmemory::inmemory_session::InMemorySessionRepository;
use storage_inmemory::inmemory_user::InMemoryUserRepository;

type TestAccount = AccountService<
    InMemorySessionRepository,
    InMemoryAdminRepository,
    InMemoryUserRepository,
    InMemoryPlatformRepository,
>;

struct Harness {
    account: Arc<TestAccount>,
    admin: Arc<AdminService<InMemoryAdminRepository>>,
    users: Arc<InMemoryUserRepository>,
}

fn harness() -> Harness {
    let sessions = Arc::new(InMemorySessionRepository::new());
    let admin = Arc::new(AdminService::new(Arc::new(InMemoryAdminRepository::new())));
    let users = Arc::new(InMemoryUserRepository::new());
    let platforms = Arc::new(InMemoryPlatformRepository::new_seeded());
    let settings = SharedSettings::test_new(RuntimeConfig::test_runtime("test-key"));
    let session_service = Arc::new(SessionService::new(
        Arc::clone(&sessions),
        Arc::clone(&admin),
        settings,
    ));
    let user_service = Arc::new(UserService::new(Arc::clone(&users), platforms));
    let account = Arc::new(AccountService::new(
        session_service,
        user_service,
        Duration::from_secs(365 * 24 * 60 * 60),
    ));
    Harness {
        account,
        admin,
        users,
    }
}

async fn login_ticket(account: &TestAccount, twitch_id: &str, name: Option<&str>) -> String {
    account
        .sessions()
        .create_login_ticket(PlatformId::TWITCH, twitch_id, name)
        .await
        .unwrap()
        .ticket
        .as_str()
        .to_string()
}

#[tokio::test]
async fn guest_creates_user_and_reuses_session() {
    let h = harness();
    let (session, is_root) = h.account.guest(None).await.unwrap();
    assert!(!is_root);
    assert_eq!(session.platform_id, None);
    assert_eq!(session.platform_user_id, None);
    assert!(h.users.get_by_id(session.user_id).await.unwrap().is_some());

    let (same, _) = h.account.guest(Some(session.token.as_str())).await.unwrap();
    assert_eq!(same.token.as_str(), session.token.as_str());
    assert_eq!(same.user_id, session.user_id);
}

#[tokio::test]
async fn login_links_twitch_to_guest_without_changing_token() {
    let h = harness();
    let (guest, _) = h.account.guest(None).await.unwrap();
    let ticket = login_ticket(&h.account, "123", Some("viewer")).await;

    let (session, is_root) = h
        .account
        .login(Some(guest.token.as_str()), Some(&ticket), ticket.as_str())
        .await
        .unwrap();

    assert!(!is_root);
    assert_eq!(session.token.as_str(), guest.token.as_str());
    assert_eq!(session.user_id, guest.user_id);
    assert_eq!(session.platform_id, Some(PlatformId::TWITCH));
    assert_eq!(session.platform_user_id.as_deref(), Some("123"));
    let platforms = h.users.get_platforms(guest.user_id).await.unwrap();
    assert_eq!(platforms.len(), 1);
    assert_eq!(platforms[0].platform_id, PlatformId::TWITCH);
    assert_eq!(platforms[0].platform_user_id, "123");
}

#[tokio::test]
async fn login_existing_identity_merges_guest_into_it() {
    let h = harness();
    let (first_guest, _) = h.account.guest(None).await.unwrap();
    let first_ticket = login_ticket(&h.account, "123", Some("viewer")).await;
    let (first, _) = h
        .account
        .login(
            Some(first_guest.token.as_str()),
            Some(&first_ticket),
            first_ticket.as_str(),
        )
        .await
        .unwrap();

    let (second_guest, _) = h.account.guest(None).await.unwrap();
    let second_ticket = login_ticket(&h.account, "123", Some("viewer")).await;
    let (second, _) = h
        .account
        .login(
            Some(second_guest.token.as_str()),
            Some(&second_ticket),
            second_ticket.as_str(),
        )
        .await
        .unwrap();

    assert_eq!(second.token.as_str(), second_guest.token.as_str());
    assert_eq!(second.user_id, first.user_id);
    assert!(
        h.users
            .get_by_id(second_guest.user_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn login_rejects_ticket_without_matching_cookie() {
    let h = harness();
    let ticket = login_ticket(&h.account, "123", None).await;

    let err = h
        .account
        .login(None, None, ticket.as_str())
        .await
        .unwrap_err();
    assert!(matches!(err, SessionServiceError::InvalidTicket));

    let err = h
        .account
        .login(None, Some("stolen"), ticket.as_str())
        .await
        .unwrap_err();
    assert!(matches!(err, SessionServiceError::InvalidTicket));
}

#[tokio::test]
async fn login_conflicts_when_current_user_has_platforms() {
    let h = harness();
    let (current, _) = h.account.guest(None).await.unwrap();
    h.users
        .link_platform(current.user_id, PlatformId::YOUTUBE, "yt-1", "yt")
        .await
        .unwrap();

    let (other_guest, _) = h.account.guest(None).await.unwrap();
    let first_ticket = login_ticket(&h.account, "123", None).await;
    h.account
        .login(
            Some(other_guest.token.as_str()),
            Some(&first_ticket),
            first_ticket.as_str(),
        )
        .await
        .unwrap();

    let ticket = login_ticket(&h.account, "123", None).await;
    let err = h
        .account
        .login(Some(current.token.as_str()), Some(&ticket), ticket.as_str())
        .await
        .unwrap_err();
    assert!(matches!(err, SessionServiceError::AccountConflict));
}

#[tokio::test]
async fn login_updates_admin_display_name() {
    let h = harness();
    h.admin.add("123", Some("old_name")).await.unwrap();

    let ticket = login_ticket(&h.account, "123", Some("new_name")).await;
    h.account
        .login(None, Some(&ticket), ticket.as_str())
        .await
        .unwrap();

    assert_eq!(
        h.admin
            .get("123")
            .await
            .unwrap()
            .unwrap()
            .display_name
            .as_deref(),
        Some("new_name")
    );
}

#[tokio::test]
async fn login_reports_root() {
    let h = harness();
    h.admin.add("123", None).await.unwrap();
    h.admin.set_root("123", true).await.unwrap();

    let ticket = login_ticket(&h.account, "123", None).await;
    let (_, is_root) = h
        .account
        .login(None, Some(&ticket), ticket.as_str())
        .await
        .unwrap();
    assert!(is_root);
}
