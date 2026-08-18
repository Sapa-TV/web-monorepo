#![feature(sync_nonpoison)]
#![feature(nonpoison_mutex)]
#![feature(nonpoison_rwlock)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::new_ret_no_self)]

pub mod actions;
pub mod admin;
pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod event;
pub mod ingress;
pub mod platform;
pub mod queue;
pub mod random;
pub mod roulette;
pub mod rules;
pub mod runtime;
pub mod session;
pub mod state;
pub mod stream;
#[cfg(test)]
pub mod test_fixtures;
pub mod user;
pub mod widget_api;

use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiSchema;

struct MergeSubdocs;

impl Modify for MergeSubdocs {
    fn modify(&self, openapi: &mut OpenApiSchema) {
        let mut widget = OpenApiSchema::default();
        widget.merge(widget_api::rarities::RaritiesApiDoc::openapi());
        widget.merge(widget_api::roulette_slots::SlotsApiDoc::openapi());
        widget.merge(widget_api::users::UsersApiDoc::openapi());
        widget.merge(widget_api::queue::QueueApiDoc::openapi());
        widget.merge(widget_api::stream::StreamApiDoc::openapi());
        *openapi = openapi.clone().nest("/wapi", widget);

        let mut main = OpenApiSchema::default();
        main.merge(api::stream::StreamApiDoc::openapi());
        main.merge(api::admin::twitch::AdminTwitchApiDoc::openapi());
        main.merge(api::admin::ingress::AdminIngressApiDoc::openapi());
        main.merge(api::admin::AdminApiDoc::openapi());
        main.merge(api::session::SessionApiDoc::openapi());
        *openapi = openapi.clone().nest("/api", main);
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = concat!(env!("CARGO_PKG_NAME"), " API"),
        version = env!("CARGO_PKG_VERSION")
    ),
    tags(
        (name = "slots", description = "Roulette slot management"),
        (name = "rarities", description = "Rarity management"),
        (name = "roulette", description = "Roulette gameplay"),
        (name = "users", description = "User management"),
        (name = "queue", description = "Spin queue"),
        (name = "stream", description = "Stream status"),
        (name = "auth", description = "Sessions and login"),
        (name = "admin", description = "Administrative endpoints")
    ),
    modifiers(&MergeSubdocs)
)]
#[non_exhaustive]
pub struct ApiDoc;
