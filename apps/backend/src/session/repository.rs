use std::future::Future;

use chrono::{DateTime, Utc};

use crate::error::RepositoryError;
use crate::session::{LoginTicket, LoginTicketToken, Session, SessionToken};

pub trait SessionRepository: Send + Sync {
    fn save_session(
        &self,
        session: &Session,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;

    fn get_session(
        &self,
        token: &SessionToken,
    ) -> impl Future<Output = Result<Option<Session>, RepositoryError>> + Send;

    fn delete_session(
        &self,
        token: &SessionToken,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;

    fn purge_expired_sessions(
        &self,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<usize, RepositoryError>> + Send;

    fn save_ticket(
        &self,
        ticket: &LoginTicket,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;

    fn take_ticket(
        &self,
        ticket: &LoginTicketToken,
    ) -> impl Future<Output = Result<Option<LoginTicket>, RepositoryError>> + Send;

    fn purge_expired_tickets(
        &self,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<usize, RepositoryError>> + Send;
}
