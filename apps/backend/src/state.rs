use std::sync::Arc;

use sqlx::SqlitePool;

pub use app::UniAppState;
pub use app::assemble_uni_state;
pub use app::{UniStateParams, UniStateParams as AppStateParams};

use crate::admin::auth::AdminAuthService;
use crate::admin::vk_auth::VkVideoLiveAdminAuthService;
use crate::config::store::ConfigStore;
use crate::db::sqlite::action::SqliteActionRepository;
use crate::db::sqlite::admin::SqliteAdminRepository;
use crate::db::sqlite::config::SqliteConfigRepository;
use crate::db::sqlite::game_order::SqliteGameOrderRepository;
use crate::db::sqlite::movie_order::SqliteMovieOrderRepository;
use crate::db::sqlite::platform::SqlitePlatformRepository;
use crate::db::sqlite::platform_credential::SqlitePlatformCredentialRepository;
use crate::db::sqlite::queue::SqliteQueueRepository;
use crate::db::sqlite::rarity::SqliteRarityRepository;
use crate::db::sqlite::roulette_slot::SqliteRouletteSlotRepository;
use crate::db::sqlite::rule::SqliteRuleRepository;
use crate::db::sqlite::session::SqliteSessionRepository;
use crate::db::sqlite::user::SqliteUserRepository;
use crate::db::sqlite::vip_record::SqliteVipRecordRepository;
use crate::error::RepositoryError;
use crate::ingress::twitch_auth::TwitchAuthService;
use crate::ingress::vk_video_live_auth::VkVideoLiveAuthService;
use crate::ingress::{EventIngress, spawn_logging_handler};
use crate::platform::PlatformCredentialService;
use crate::presence::Presence;
use crate::random::StandartRandomProvider;
use crate::sheets::client::GoogleSheetsClient;
use crate::sheets::service::SheetsService;
use crate::stream::StreamStatus;

pub type AppQueueService =
    QueueService<SqliteQueueRepository, SqliteRarityRepository, SqliteRouletteSlotRepository>;

pub type AppSessionService = SessionService<SqliteSessionRepository, SqliteAdminRepository>;

pub type AppConfigStore = ConfigStore<SqliteConfigRepository>;

#[allow(clippy::type_complexity)]
pub type AppSheetsService = SheetsService<
    GoogleSheetsClient,
    Arc<SqliteGameOrderRepository>,
    Arc<SqliteMovieOrderRepository>,
    Arc<SqliteVipRecordRepository>,
    AppConfigStore,
>;

pub type AppState = UniAppState<
    SqliteQueueRepository,
    SqliteRarityRepository,
    SqliteUserRepository,
    SqlitePlatformRepository,
    SqliteRouletteSlotRepository,
    SqliteAdminRepository,
    SqliteSessionRepository,
    SqliteRuleRepository,
    SqliteActionRepository,
    SqliteGameOrderRepository,
    SqliteMovieOrderRepository,
    SqliteVipRecordRepository,
    AppConfigStore,
    GoogleSheetsClient,
    PlatformCredentialService<SqlitePlatformCredentialRepository>,
    AdminAuthService<SqlitePlatformCredentialRepository>,
    VkVideoLiveAdminAuthService<SqlitePlatformCredentialRepository>,
    TwitchAuthService<SqlitePlatformCredentialRepository>,
    VkVideoLiveAuthService<SqlitePlatformCredentialRepository>,
    EventIngress,
    Presence,
    StreamStatus,
>;

use crate::queue::service::QueueService;
use crate::session::service::SessionService;

#[non_exhaustive]
pub struct AppStateBuilder {
    random: StandartRandomProvider,
    pool: SqlitePool,
    config: Arc<AppConfigStore>,
    credentials_repo: Arc<SqlitePlatformCredentialRepository>,
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
    #[cfg(test)]
    pub fn with_empty_repos(mut self) -> Self {
        self.seeded = false;
        self
    }

    #[cfg(test)]
    pub fn with_queue_repo(mut self, queue_repo: Arc<SqliteQueueRepository>) -> Self {
        self.queue_repo = Some(queue_repo);
        self
    }

    pub async fn build(self) -> Result<AppState, RepositoryError> {
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

        assemble_uni_state(UniStateParams {
            random: self.random,
            settings,
            sheets_client,
            config: self.config,
            credential_service: credentials,
            admin_auth,
            vk_admin_auth,
            twitch_api,
            vk_api,
            ingress,
            presence: Presence::new(),
            stream_status: Arc::new(StreamStatus::new()),
            slot_repo: Arc::new(SqliteRouletteSlotRepository::new(self.pool.clone())),
            rarity_repo: Arc::new(SqliteRarityRepository::new(self.pool.clone())),
            user_repo: Arc::new(SqliteUserRepository::new(self.pool.clone())),
            platform_repo: Arc::new(SqlitePlatformRepository::new(self.pool.clone())),
            queue_repo: self
                .queue_repo
                .unwrap_or_else(|| Arc::new(SqliteQueueRepository::new(self.pool.clone()))),
            admin_repo: Arc::new(SqliteAdminRepository::new(self.pool.clone())),
            session_repo: Arc::new(SqliteSessionRepository::new(self.pool.clone())),
            rule_repo: Arc::new(SqliteRuleRepository::new(self.pool.clone())),
            action_repo: Arc::new(SqliteActionRepository::new(self.pool.clone())),
            game_order_repo: Arc::new(SqliteGameOrderRepository::new(self.pool.clone())),
            movie_order_repo: Arc::new(SqliteMovieOrderRepository::new(self.pool.clone())),
            vip_record_repo: Arc::new(SqliteVipRecordRepository::new(self.pool.clone())),
            admin_twitch_id,
            extra_admin_twitch_ids,
        })
        .await
    }
}
