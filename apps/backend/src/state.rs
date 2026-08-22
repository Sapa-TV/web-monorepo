use sqlx::SqlitePool;
use std::sync::Arc;

use crate::actions::repository::ActionRepository;
use crate::actions::service::ActionService;
use crate::admin::auth::AdminAuthService;
use crate::admin::repository::AdminRepository;
use crate::admin::service::AdminService;
use crate::config::repository::ConfigRepository;
use crate::config::store::ConfigStore;
use crate::db::sqlite::action::SqliteActionRepository;
use crate::db::sqlite::admin::SqliteAdminRepository;
use crate::db::sqlite::config::SqliteConfigRepository;
use crate::db::sqlite::platform::SqlitePlatformRepository;
use crate::db::sqlite::platform_credential::SqlitePlatformCredentialRepository;
use crate::db::sqlite::queue::SqliteQueueRepository;
use crate::db::sqlite::rarity::SqliteRarityRepository;
use crate::db::sqlite::roulette_slot::SqliteRouletteSlotRepository;
use crate::db::sqlite::rule::SqliteRuleRepository;
use crate::db::sqlite::session::SqliteSessionRepository;
use crate::db::sqlite::user::SqliteUserRepository;
use crate::error::RepositoryError;
use crate::event::BroadcastEventPublisher;
use crate::ingress::twitch_auth::TwitchAuthService;
use crate::ingress::{EventIngress, spawn_logging_handler};
use crate::platform::{
    PlatformCredentialRepository, PlatformCredentialService, PlatformRepository,
};
use crate::queue::repository::QueueRepository;
use crate::queue::service::QueueService;
use crate::random::StandartRandomProvider;
use crate::roulette::machine::RouletteService;
use crate::roulette::rarity::RarityRepository;
use crate::roulette::rarity_service::RarityService;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::RouletteSlotService;
use crate::rules::repository::RuleRepository;
use crate::rules::service::RuleService;
use crate::session::repository::SessionRepository;
use crate::session::service::SessionService;
use crate::stream::StreamStatus;
use crate::user::repository::UserRepository;
use crate::user::service::UserService;

#[non_exhaustive]
pub struct UniAppState<Q, R, U, P, S, A, Se, C, K, L, M>
where
    Q: QueueRepository,
    R: RarityRepository,
    U: UserRepository,
    P: PlatformRepository,
    S: RouletteSlotRepository,
    A: AdminRepository,
    Se: SessionRepository,
    C: PlatformCredentialRepository,
    K: ConfigRepository,
    L: RuleRepository,
    M: ActionRepository,
{
    pub slot_service: Arc<RouletteSlotService<Arc<S>>>,
    pub rarity_service: Arc<RarityService<Arc<R>>>,
    pub user_service: Arc<UserService<U, P>>,
    pub admin_service: Arc<AdminService<A>>,
    pub session_service: Arc<SessionService<Se, A>>,
    pub queue_service: Arc<QueueService<Q, R, S>>,
    pub config: Arc<ConfigStore<K>>,
    pub event_publisher: BroadcastEventPublisher,
    pub stream_status: Arc<StreamStatus>,
    pub ingress: Arc<EventIngress>,
    pub admin_auth: Arc<AdminAuthService<C>>,
    pub credentials: Arc<PlatformCredentialService<C>>,
    pub rule_service: Arc<RuleService<L, M>>,
    pub action_service: Arc<ActionService<M>>,
    pub twitch_api: Option<Arc<TwitchAuthService<C>>>,
}

impl<Q, R, U, P, S, A, Se, C, K, L, M> Clone for UniAppState<Q, R, U, P, S, A, Se, C, K, L, M>
where
    Q: QueueRepository,
    R: RarityRepository,
    U: UserRepository,
    P: PlatformRepository,
    S: RouletteSlotRepository,
    A: AdminRepository,
    Se: SessionRepository,
    C: PlatformCredentialRepository,
    K: ConfigRepository,
    L: RuleRepository,
    M: ActionRepository,
{
    fn clone(&self) -> Self {
        Self {
            slot_service: Arc::clone(&self.slot_service),
            rarity_service: Arc::clone(&self.rarity_service),
            user_service: Arc::clone(&self.user_service),
            admin_service: Arc::clone(&self.admin_service),
            session_service: Arc::clone(&self.session_service),
            queue_service: Arc::clone(&self.queue_service),
            config: Arc::clone(&self.config),
            event_publisher: self.event_publisher.clone(),
            stream_status: Arc::clone(&self.stream_status),
            ingress: Arc::clone(&self.ingress),
            admin_auth: Arc::clone(&self.admin_auth),
            credentials: Arc::clone(&self.credentials),
            rule_service: Arc::clone(&self.rule_service),
            action_service: Arc::clone(&self.action_service),
            twitch_api: self.twitch_api.clone(),
        }
    }
}

pub type AppQueueService =
    QueueService<SqliteQueueRepository, SqliteRarityRepository, SqliteRouletteSlotRepository>;

pub type AppSessionService = SessionService<SqliteSessionRepository, SqliteAdminRepository>;

pub type AppConfigStore = ConfigStore<SqliteConfigRepository>;

pub type AppState = UniAppState<
    SqliteQueueRepository,
    SqliteRarityRepository,
    SqliteUserRepository,
    SqlitePlatformRepository,
    SqliteRouletteSlotRepository,
    SqliteAdminRepository,
    SqliteSessionRepository,
    SqlitePlatformCredentialRepository,
    SqliteConfigRepository,
    SqliteRuleRepository,
    SqliteActionRepository,
>;

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
            sqlx::query("DELETE FROM roulette_slots")
                .execute(&self.pool)
                .await?;
            sqlx::query("DELETE FROM rarities")
                .execute(&self.pool)
                .await?;
        }

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

        let event_publisher = BroadcastEventPublisher::new();
        let ingress = Arc::new(EventIngress::new());
        spawn_logging_handler(ingress.subscribe());

        let slot_service = Arc::new(RouletteSlotService::build(Arc::clone(&slot_repo)).await?);
        let rarity_service = Arc::new(RarityService::build(Arc::clone(&rarity_repo)).await?);
        let settings = self.config.source();
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
        if let Some(admin_id) = self.config.admin_twitch_id() {
            tracing::info!("seeding root admin: twitch_user_id={admin_id}");
            admin_service.seed(admin_id).await?;
        }
        let session_service = Arc::new(SessionService::new(
            session_repo,
            Arc::clone(&admin_service),
            settings,
        ));
        let credentials = Arc::new(PlatformCredentialService::new(Arc::clone(
            &self.credentials_repo,
        )));
        let admin_auth = Arc::new(AdminAuthService::new(
            self.config.twitch().map(|twitch| Arc::new(twitch.clone())),
            Arc::clone(&credentials),
        ));

        let action_service = Arc::new(ActionService::new(action_repo));
        let rule_service = Arc::new(RuleService::new(rule_repo, Arc::clone(&action_service)));

        let twitch_api = self.config.twitch().map(|twitch| {
            Arc::new(TwitchAuthService::new(
                Arc::new(twitch.clone()),
                Arc::clone(&credentials),
            ))
        });

        Ok(AppState {
            slot_service,
            rarity_service,
            user_service,
            admin_service,
            session_service,
            queue_service,
            config: self.config,
            event_publisher,
            stream_status: Arc::new(StreamStatus::new()),
            ingress,
            admin_auth,
            credentials,
            rule_service,
            action_service,
            twitch_api,
        })
    }
}
