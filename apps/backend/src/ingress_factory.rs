use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use composition::config::twitch::TwitchConfig;
use composition::config::vk_video_live::VkVideoLiveConfig;
use composition::ingress::PlatformService;
use composition::ingress::platform::EventSink;
use composition::ingress::twitch::TwitchPlatformService;
use composition::ingress::vk_video_live::VkVideoLivePlatformService;
use composition::ingress::vk_video_live_auth::VkVideoLiveAuthService;
use composition::platform::{PlatformCredentialService, PlatformId};
use composition::state::AppConfigStore;
use storage_sqlite::platform_credential::SqlitePlatformCredentialRepository;

pub fn log_platform_config(config: &AppConfigStore) {
    match config.twitch() {
        Some(twitch) => {
            tracing::info!(
                "twitch config ready: client_id={}, broadcaster_id={}, redirect_uri={}, credentials_redirect_uri={}",
                twitch.client_id,
                twitch.broadcaster_id,
                twitch.redirect_uri,
                twitch.credentials_redirect_uri
            );
        }
        None => {
            tracing::info!("twitch config NOT configured: twitch login and ingress will not work");
        }
    }

    match config.vk_video_live() {
        Some(vk) => {
            tracing::info!(
                "vk video live config ready: client_id={}, redirect_uri={}, credentials_redirect_uri={}",
                vk.client_id,
                vk.redirect_uri,
                vk.credentials_redirect_uri
            );
        }
        None => {
            tracing::info!("vk video live config NOT configured: vk ingress will not work");
        }
    }
}

pub trait IngressSpawner:
    Fn(
        PlatformId,
        Arc<PlatformCredentialService<SqlitePlatformCredentialRepository>>,
        EventSink,
        CancellationToken,
    ) -> Option<JoinHandle<()>>
    + Send
    + Sync
{
}

impl<F> IngressSpawner for F where
    F: Fn(
            PlatformId,
            Arc<PlatformCredentialService<SqlitePlatformCredentialRepository>>,
            EventSink,
            CancellationToken,
        ) -> Option<JoinHandle<()>>
        + Send
        + Sync
{
}

pub fn ingress_factory(
    twitch_config: Option<Arc<TwitchConfig>>,
    vk_config: Option<Arc<VkVideoLiveConfig>>,
) -> (&'static [PlatformId], Box<dyn IngressSpawner>) {
    let mut platform_ids = Vec::new();
    if twitch_config.is_some() {
        platform_ids.push(PlatformId::TWITCH);
    }
    if vk_config.is_some() {
        platform_ids.push(PlatformId::VK_VIDEO_LIVE);
    }
    let platforms: &'static [PlatformId] = Box::leak(platform_ids.into_boxed_slice());

    let build =
        move |platform: PlatformId,
              credentials: Arc<PlatformCredentialService<SqlitePlatformCredentialRepository>>,
              sink: EventSink,
              token: CancellationToken|
              -> Option<JoinHandle<()>> {
            match platform {
                PlatformId::TWITCH => match &twitch_config {
                    Some(config) => {
                        let config = Arc::clone(config);
                        Some(tokio::spawn(async move {
                            let service = TwitchPlatformService::new(config, credentials);
                            if let Err(e) = service.run(sink, token).await {
                                tracing::error!(
                                    "{} ingress stopped: {e}",
                                    service.platform().as_name()
                                );
                            }
                        }))
                    }
                    None => None,
                },
                PlatformId::VK_VIDEO_LIVE => match &vk_config {
                    Some(config) => {
                        let config = Arc::clone(config);
                        Some(tokio::spawn(async move {
                            let auth = Arc::new(VkVideoLiveAuthService::new(config, credentials));
                            let service = VkVideoLivePlatformService::new(auth);
                            if let Err(e) = service.run(sink, token).await {
                                tracing::error!(
                                    "{} ingress stopped: {e}",
                                    service.platform().as_name()
                                );
                            }
                        }))
                    }
                    None => None,
                },
                _ => None,
            }
        };

    (platforms, Box::new(build))
}
