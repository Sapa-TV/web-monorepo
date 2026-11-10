use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;

use crate::admin::auth::AdminAuthService;
use crate::admin::vk_auth::VkVideoLiveAdminAuthService;
use crate::admin::vk_id_auth::VkIdAuthService;
use crate::config::store::ConfigStore;
use crate::ingress::twitch_auth::TwitchAuthService;
use crate::ingress::vk_video_live_auth::VkVideoLiveAuthService;
use crate::ingress::{EventIngress, spawn_logging_handler};
use crate::platform::PlatformCredentialService;
use crate::presence::Presence;
use crate::push::WebPushSender;
use crate::sheets::client::GoogleSheetsClient;
use crate::stream::StreamStatus;
use domain::actions::service::ActionService;
use domain::admin::service::AdminService;
use domain::error::RepositoryError;
use domain::orders::service::{GameOrderService, MovieOrderService, VipService};
use domain::push::service::PushService;
use domain::queue::publisher::BroadcastEventPublisher;
use domain::queue::service::QueueService;
use domain::random::StandartRandomProvider;
use domain::roulette::machine::RouletteService;
use domain::roulette::rarity_service::RarityService;
use domain::roulette::slot_service::RouletteSlotService;
use domain::rules::service::RuleService;
use domain::session::account::AccountService;
use domain::session::service::SessionService;
use domain::sheets::service::SheetsService;
use domain::user::service::UserService;
use storage_sqlite::action::SqliteActionRepository;
use storage_sqlite::admin::SqliteAdminRepository;
use storage_sqlite::config::SqliteConfigRepository;
use storage_sqlite::game_order::SqliteGameOrderRepository;
use storage_sqlite::movie_order::SqliteMovieOrderRepository;
use storage_sqlite::platform::SqlitePlatformRepository;
use storage_sqlite::platform_credential::SqlitePlatformCredentialRepository;
use storage_sqlite::push::SqlitePushSubscriptionRepository;
use storage_sqlite::queue::SqliteQueueRepository;
use storage_sqlite::rarity::SqliteRarityRepository;
use storage_sqlite::roulette_slot::SqliteRouletteSlotRepository;
use storage_sqlite::rule::SqliteRuleRepository;
use storage_sqlite::session::SqliteSessionRepository;
use storage_sqlite::user::SqliteUserRepository;
use storage_sqlite::vip_record::SqliteVipRecordRepository;

pub type AppQueueService =
    QueueService<SqliteQueueRepository, SqliteRarityRepository, SqliteRouletteSlotRepository>;

pub type AppSessionService = SessionService<SqliteSessionRepository, SqliteAdminRepository>;

pub type AppAccountService = AccountService<
    SqliteSessionRepository,
    SqliteAdminRepository,
    SqliteUserRepository,
    SqlitePlatformRepository,
>;

pub type AppPushService = PushService<SqlitePushSubscriptionRepository, WebPushSender>;

pub type AppConfigStore = ConfigStore<SqliteConfigRepository>;

pub type AppSheetsService = SheetsService<
    GoogleSheetsClient,
    Arc<SqliteGameOrderRepository>,
    Arc<SqliteMovieOrderRepository>,
    Arc<SqliteVipRecordRepository>,
    AppConfigStore,
>;

#[non_exhaustive]
#[derive(Clone)]
pub struct AppState {
    pub slot_service: Arc<RouletteSlotService<Arc<SqliteRouletteSlotRepository>>>,
    pub rarity_service: Arc<RarityService<Arc<SqliteRarityRepository>>>,
    pub user_service: Arc<UserService<SqliteUserRepository, SqlitePlatformRepository>>,
    pub admin_service: Arc<AdminService<SqliteAdminRepository>>,
    pub session_service: Arc<AppSessionService>,
    pub account_service: Arc<AppAccountService>,
    pub queue_service: Arc<AppQueueService>,
    pub push_service: Arc<AppPushService>,
    pub config: Arc<AppConfigStore>,
    pub event_publisher: BroadcastEventPublisher,
    pub stream_status: Arc<StreamStatus>,
    pub ingress: Arc<EventIngress>,
    pub presence: Arc<Presence>,
    pub admin_auth: Arc<AdminAuthService<SqlitePlatformCredentialRepository>>,
    pub vk_id_auth: Arc<VkIdAuthService>,
    pub vk_admin_auth: Arc<VkVideoLiveAdminAuthService<SqlitePlatformCredentialRepository>>,
    pub credentials: Arc<PlatformCredentialService<SqlitePlatformCredentialRepository>>,
    pub rule_service: Arc<RuleService<SqliteRuleRepository, SqliteActionRepository>>,
    pub action_service: Arc<ActionService<SqliteActionRepository>>,
    pub twitch_api: Option<Arc<TwitchAuthService<SqlitePlatformCredentialRepository>>>,
    pub vk_api: Option<Arc<VkVideoLiveAuthService<SqlitePlatformCredentialRepository>>>,
    pub game_order_service: Arc<GameOrderService<Arc<SqliteGameOrderRepository>>>,
    pub movie_order_service: Arc<MovieOrderService<Arc<SqliteMovieOrderRepository>>>,
    pub vip_service: Arc<VipService<Arc<SqliteVipRecordRepository>>>,
    pub sheets: Arc<AppSheetsService>,
}

#[non_exhaustive]
pub struct AppStateBuilder {
    random: StandartRandomProvider,
    pool: SqlitePool,
    config: Arc<AppConfigStore>,
    credentials_repo: Arc<SqlitePlatformCredentialRepository>,
    #[cfg_attr(not(any(test, feature = "test-support")), allow(dead_code))]
    seeded: bool,
    queue_repo: Option<Arc<SqliteQueueRepository>>,
}

impl AppStateBuilder {
    pub fn new(
        random: StandartRandomProvider,
        config: Arc<AppConfigStore>,
        credentials_repo: Arc<SqlitePlatformCredentialRepository>,
        pool: SqlitePool,
    ) -> Self {
        Self {
            random,
            pool,
            config,
            credentials_repo,
            seeded: true,
            queue_repo: None,
        }
    }

    /// Seeds come from migrations; tests that need an empty slate drop them.
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_empty_repos(mut self) -> Self {
        self.seeded = false;
        self
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn with_queue_repo(mut self, queue_repo: Arc<SqliteQueueRepository>) -> Self {
        self.queue_repo = Some(queue_repo);
        self
    }

    pub async fn build(self) -> Result<AppState, RepositoryError> {
        #[cfg(any(test, feature = "test-support"))]
        if !self.seeded {
            storage_sqlite::reset_roulette_seed_data(&self.pool)
                .await
                .map_err(|e| RepositoryError::Database(e.to_string()))?;
        }

        let credentials = Arc::new(PlatformCredentialService::new(self.credentials_repo));

        let admin_auth = Arc::new(AdminAuthService::new(
            self.config.twitch().map(|twitch| Arc::new(twitch.clone())),
            Arc::clone(&credentials),
        ));

        let vk_id_auth = Arc::new(VkIdAuthService::new(
            self.config.vk_id().map(|vk| Arc::new(vk.clone())),
        ));

        let vk_config = self.config.vk_video_live().map(|vk| Arc::new(vk.clone()));
        let vk_api = vk_config
            .clone()
            .map(|cfg| Arc::new(VkVideoLiveAuthService::new(cfg, Arc::clone(&credentials))));
        let vk_admin_auth = Arc::new(VkVideoLiveAdminAuthService::new(vk_config, vk_api.clone()));

        let twitch_api = self.config.twitch().map(|twitch| {
            Arc::new(TwitchAuthService::new(
                Arc::new(twitch.clone()),
                Arc::clone(&credentials),
            ))
        });

        let sheets_client = self
            .config
            .google_service_account_key_base64()
            .and_then(|encoded| {
                GoogleSheetsClient::from_key_base64(encoded)
                    .map_err(|e| tracing::error!("google sheets disabled: {e}"))
                    .ok()
            });

        let ingress = Arc::new(EventIngress::new());
        spawn_logging_handler(ingress.subscribe());

        let settings = self.config.source();
        let admin_twitch_id = self.config.admin_twitch_id().map(str::to_string);
        let extra_admin_twitch_ids = self
            .config
            .admin_twitch_ids()
            .map(|ids| ids.to_vec())
            .unwrap_or_default();

        let event_publisher = BroadcastEventPublisher::new();

        let slot_repo = Arc::new(SqliteRouletteSlotRepository::new(self.pool.clone()));
        let rarity_repo = Arc::new(SqliteRarityRepository::new(self.pool.clone()));
        let user_repo = Arc::new(SqliteUserRepository::new(self.pool.clone()));
        let platform_repo = Arc::new(SqlitePlatformRepository::new(self.pool.clone()));
        let queue_repo = self
            .queue_repo
            .unwrap_or_else(|| Arc::new(SqliteQueueRepository::new(self.pool.clone())));
        let admin_repo = Arc::new(SqliteAdminRepository::new(self.pool.clone()));
        let session_repo = Arc::new(SqliteSessionRepository::new(self.pool.clone()));
        let rule_repo = Arc::new(SqliteRuleRepository::new(self.pool.clone()));
        let action_repo = Arc::new(SqliteActionRepository::new(self.pool.clone()));
        let game_order_repo = Arc::new(SqliteGameOrderRepository::new(self.pool.clone()));
        let movie_order_repo = Arc::new(SqliteMovieOrderRepository::new(self.pool.clone()));
        let vip_record_repo = Arc::new(SqliteVipRecordRepository::new(self.pool.clone()));
        let push_repo = Arc::new(SqlitePushSubscriptionRepository::new(self.pool.clone()));

        let slot_service = Arc::new(RouletteSlotService::build(Arc::clone(&slot_repo)).await?);
        let rarity_service = Arc::new(RarityService::build(Arc::clone(&rarity_repo)).await?);
        let roulette = RouletteService::new(Arc::clone(&slot_service), self.random);
        let queue_service = Arc::new(QueueService::new(
            queue_repo,
            Arc::clone(&rarity_service),
            roulette,
            event_publisher.clone(),
            settings.clone(),
        ));
        let user_service = Arc::new(UserService::new(user_repo, platform_repo));
        let admin_service = Arc::new(AdminService::new(admin_repo));
        if let Some(admin_id) = admin_twitch_id {
            tracing::info!("seeding root admin: twitch_user_id={admin_id}");
            admin_service.seed(&admin_id).await?;
        }
        for admin_id in extra_admin_twitch_ids {
            admin_service.seed_non_root(&admin_id).await?;
        }
        let session_service = Arc::new(SessionService::new(
            session_repo,
            Arc::clone(&admin_service),
            settings,
        ));
        let account_service = Arc::new(AccountService::new(
            Arc::clone(&session_service),
            Arc::clone(&user_service),
            Duration::from_secs(self.config.guest_session_ttl_secs()),
        ));
        let action_service = Arc::new(ActionService::new(action_repo));
        let rule_service = Arc::new(RuleService::new(rule_repo, Arc::clone(&action_service)));

        let game_order_service = Arc::new(GameOrderService::new(Arc::clone(&game_order_repo)));
        let movie_order_service = Arc::new(MovieOrderService::new(Arc::clone(&movie_order_repo)));
        let vip_service = Arc::new(VipService::new(Arc::clone(&vip_record_repo)));

        let push_sender = Arc::new(WebPushSender::new(
            self.config.vapid_private_key(),
            self.config.vapid_subject(),
        ));
        let push_service = Arc::new(PushService::new(push_repo, push_sender));

        let sheets = Arc::new(SheetsService::new(
            sheets_client,
            Arc::clone(&game_order_service),
            Arc::clone(&movie_order_service),
            Arc::clone(&vip_service),
            Arc::clone(&self.config),
        ));

        Ok(AppState {
            slot_service,
            rarity_service,
            user_service,
            admin_service,
            session_service,
            account_service,
            queue_service,
            push_service,
            config: self.config,
            event_publisher,
            stream_status: Arc::new(StreamStatus::new()),
            ingress,
            presence: Presence::new(),
            admin_auth,
            vk_id_auth,
            vk_admin_auth,
            credentials,
            rule_service,
            action_service,
            twitch_api,
            vk_api,
            game_order_service,
            movie_order_service,
            vip_service,
            sheets,
        })
    }
}
