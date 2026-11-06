use std::sync::Arc;

use domain::actions::service::ActionService;
use domain::admin::service::AdminService;
use domain::config::settings::SharedSettings;
use domain::orders::service::{GameOrderService, MovieOrderService, VipService};
use domain::queue::publisher::BroadcastEventPublisher;
use domain::queue::service::QueueService;
use domain::random::StandartRandomProvider;
use domain::roulette::machine::RouletteService;
use domain::roulette::rarity_service::RarityService;
use domain::roulette::slot_service::RouletteSlotService;
use domain::rules::service::RuleService;
use domain::session::service::SessionService;
use domain::sheets::client::SheetsClient;
use domain::sheets::service::SheetsService;
use domain::sheets::settings::SheetsSettings;
use domain::user::service::UserService;
use model::actions::repository::ActionRepository;
use model::admin::repository::AdminRepository;
use model::error::RepositoryError;
use model::orders::game::{GameOrder, GameOrderId, GameOrderKind, NewGameOrder};
use model::orders::movie::{MovieKind, MovieOrder, MovieOrderId, NewMovieOrder};
use model::orders::repository::{OrderRepository, VipRecordRepository};
use model::platform::PlatformRepository;
use model::queue::repository::QueueRepository;
use model::roulette::rarity::RarityRepository;
use model::roulette::repository::RouletteSlotRepository;
use model::rules::repository::RuleRepository;
use model::session::repository::SessionRepository;
use model::user::repository::UserRepository;

#[non_exhaustive]
pub struct UniAppState<Q, R, U, P, S, A, Se, L, M, Go, Mo, V, CS, SC, CR, AA, VA, T, W, IN, PR, ST>
where
    Q: QueueRepository,
    R: RarityRepository,
    U: UserRepository,
    P: PlatformRepository,
    S: RouletteSlotRepository,
    A: AdminRepository,
    Se: SessionRepository,
    L: RuleRepository,
    M: ActionRepository,
    Go: OrderRepository<
            Order = GameOrder,
            New = NewGameOrder,
            Id = GameOrderId,
            Kind = GameOrderKind,
        >,
    Mo: OrderRepository<
            Order = MovieOrder,
            New = NewMovieOrder,
            Id = MovieOrderId,
            Kind = MovieKind,
        >,
    V: VipRecordRepository,
    CS: SheetsSettings,
    SC: SheetsClient,
{
    pub slot_service: Arc<RouletteSlotService<Arc<S>>>,
    pub rarity_service: Arc<RarityService<Arc<R>>>,
    pub user_service: Arc<UserService<U, P>>,
    pub admin_service: Arc<AdminService<A>>,
    pub session_service: Arc<SessionService<Se, A>>,
    pub queue_service: Arc<QueueService<Q, R, S>>,
    pub config: Arc<CS>,
    pub event_publisher: BroadcastEventPublisher,
    pub stream_status: Arc<ST>,
    pub ingress: Arc<IN>,
    pub presence: Arc<PR>,
    pub admin_auth: Arc<AA>,
    pub vk_admin_auth: Arc<VA>,
    pub credentials: Arc<CR>,
    pub rule_service: Arc<RuleService<L, M>>,
    pub action_service: Arc<ActionService<M>>,
    pub twitch_api: Option<Arc<T>>,
    pub vk_api: Option<Arc<W>>,
    pub game_order_service: Arc<GameOrderService<Arc<Go>>>,
    pub movie_order_service: Arc<MovieOrderService<Arc<Mo>>>,
    pub vip_service: Arc<VipService<Arc<V>>>,
    #[allow(clippy::type_complexity)]
    pub sheets: Arc<SheetsService<SC, Arc<Go>, Arc<Mo>, Arc<V>, CS>>,
}

impl<Q, R, U, P, S, A, Se, L, M, Go, Mo, V, CS, SC, CR, AA, VA, T, W, IN, PR, ST> Clone
    for UniAppState<Q, R, U, P, S, A, Se, L, M, Go, Mo, V, CS, SC, CR, AA, VA, T, W, IN, PR, ST>
where
    Q: QueueRepository,
    R: RarityRepository,
    U: UserRepository,
    P: PlatformRepository,
    S: RouletteSlotRepository,
    A: AdminRepository,
    Se: SessionRepository,
    L: RuleRepository,
    M: ActionRepository,
    Go: OrderRepository<
            Order = GameOrder,
            New = NewGameOrder,
            Id = GameOrderId,
            Kind = GameOrderKind,
        >,
    Mo: OrderRepository<
            Order = MovieOrder,
            New = NewMovieOrder,
            Id = MovieOrderId,
            Kind = MovieKind,
        >,
    V: VipRecordRepository,
    CS: SheetsSettings,
    SC: SheetsClient,
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
            presence: Arc::clone(&self.presence),
            admin_auth: Arc::clone(&self.admin_auth),
            vk_admin_auth: Arc::clone(&self.vk_admin_auth),
            credentials: Arc::clone(&self.credentials),
            rule_service: Arc::clone(&self.rule_service),
            action_service: Arc::clone(&self.action_service),
            twitch_api: self.twitch_api.clone(),
            vk_api: self.vk_api.clone(),
            game_order_service: Arc::clone(&self.game_order_service),
            movie_order_service: Arc::clone(&self.movie_order_service),
            vip_service: Arc::clone(&self.vip_service),
            sheets: Arc::clone(&self.sheets),
        }
    }
}

pub struct UniStateParams<
    Q,
    R,
    U,
    P,
    S,
    A,
    Se,
    L,
    M,
    Go,
    Mo,
    V,
    CS,
    SC,
    CR,
    AA,
    VA,
    T,
    W,
    IN,
    PR,
    ST,
> where
    Q: QueueRepository,
    R: RarityRepository,
    U: UserRepository,
    P: PlatformRepository,
    S: RouletteSlotRepository,
    A: AdminRepository,
    Se: SessionRepository,
    L: RuleRepository,
    M: ActionRepository,
    Go: OrderRepository<
            Order = GameOrder,
            New = NewGameOrder,
            Id = GameOrderId,
            Kind = GameOrderKind,
        >,
    Mo: OrderRepository<
            Order = MovieOrder,
            New = NewMovieOrder,
            Id = MovieOrderId,
            Kind = MovieKind,
        >,
    V: VipRecordRepository,
    CS: SheetsSettings,
    SC: SheetsClient,
{
    pub random: StandartRandomProvider,
    pub settings: SharedSettings,
    pub config: Arc<CS>,
    pub sheets_client: Option<SC>,
    pub credential_service: Arc<CR>,
    pub admin_auth: Arc<AA>,
    pub vk_admin_auth: Arc<VA>,
    pub twitch_api: Option<Arc<T>>,
    pub vk_api: Option<Arc<W>>,
    pub ingress: Arc<IN>,
    pub presence: Arc<PR>,
    pub stream_status: Arc<ST>,
    pub slot_repo: Arc<S>,
    pub rarity_repo: Arc<R>,
    pub user_repo: Arc<U>,
    pub platform_repo: Arc<P>,
    pub queue_repo: Arc<Q>,
    pub admin_repo: Arc<A>,
    pub session_repo: Arc<Se>,
    pub rule_repo: Arc<L>,
    pub action_repo: Arc<M>,
    pub game_order_repo: Arc<Go>,
    pub movie_order_repo: Arc<Mo>,
    pub vip_record_repo: Arc<V>,
    pub admin_twitch_id: Option<String>,
    pub extra_admin_twitch_ids: Vec<String>,
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub async fn assemble_uni_state<
    Q,
    R,
    U,
    P,
    S,
    A,
    Se,
    L,
    M,
    Go,
    Mo,
    V,
    CS,
    SC,
    CR,
    AA,
    VA,
    T,
    W,
    IN,
    PR,
    ST,
>(
    params: UniStateParams<
        Q,
        R,
        U,
        P,
        S,
        A,
        Se,
        L,
        M,
        Go,
        Mo,
        V,
        CS,
        SC,
        CR,
        AA,
        VA,
        T,
        W,
        IN,
        PR,
        ST,
    >,
) -> Result<
    UniAppState<Q, R, U, P, S, A, Se, L, M, Go, Mo, V, CS, SC, CR, AA, VA, T, W, IN, PR, ST>,
    RepositoryError,
>
where
    Q: QueueRepository,
    R: RarityRepository,
    U: UserRepository,
    P: PlatformRepository,
    S: RouletteSlotRepository,
    A: AdminRepository,
    Se: SessionRepository,
    L: RuleRepository,
    M: ActionRepository,
    Go: OrderRepository<
            Order = GameOrder,
            New = NewGameOrder,
            Id = GameOrderId,
            Kind = GameOrderKind,
        >,
    Mo: OrderRepository<
            Order = MovieOrder,
            New = NewMovieOrder,
            Id = MovieOrderId,
            Kind = MovieKind,
        >,
    V: VipRecordRepository,
    CS: SheetsSettings,
    SC: SheetsClient,
{
    let UniStateParams {
        random,
        settings,
        config,
        sheets_client,
        credential_service,
        admin_auth,
        vk_admin_auth,
        twitch_api,
        vk_api,
        ingress,
        presence,
        stream_status,
        slot_repo,
        rarity_repo,
        user_repo,
        platform_repo,
        queue_repo,
        admin_repo,
        session_repo,
        rule_repo,
        action_repo,
        game_order_repo,
        movie_order_repo,
        vip_record_repo,
        admin_twitch_id,
        extra_admin_twitch_ids,
    } = params;

    let event_publisher = BroadcastEventPublisher::new();

    let slot_service = Arc::new(RouletteSlotService::build(slot_repo).await?);
    let rarity_service = Arc::new(RarityService::build(rarity_repo).await?);
    let roulette = RouletteService::new(Arc::clone(&slot_service), random);
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
    let action_service = Arc::new(ActionService::new(action_repo));
    let rule_service = Arc::new(RuleService::new(rule_repo, Arc::clone(&action_service)));

    let game_order_service = Arc::new(GameOrderService::new(Arc::clone(&game_order_repo)));
    let movie_order_service = Arc::new(MovieOrderService::new(Arc::clone(&movie_order_repo)));
    let vip_service = Arc::new(VipService::new(Arc::clone(&vip_record_repo)));

    let sheets = Arc::new(SheetsService::new(
        sheets_client,
        Arc::clone(&game_order_service),
        Arc::clone(&movie_order_service),
        Arc::clone(&vip_service),
        Arc::clone(&config),
    ));

    Ok(UniAppState {
        slot_service,
        rarity_service,
        user_service,
        admin_service,
        session_service,
        queue_service,
        config,
        event_publisher,
        stream_status,
        ingress,
        presence,
        admin_auth,
        vk_admin_auth,
        credentials: credential_service,
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
