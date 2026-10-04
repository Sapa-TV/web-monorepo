use std::collections::BTreeMap;
use std::sync::nonpoison::Mutex;

use chrono::{DateTime, Utc};

use crate::error::RepositoryError;
use crate::session::repository::SessionRepository;
use crate::session::{LoginTicket, LoginTicketToken, Session, SessionToken};

#[non_exhaustive]
pub struct InMemorySessionRepository {
    sessions: Mutex<BTreeMap<String, Session>>,
    tickets: Mutex<BTreeMap<String, LoginTicket>>,
}

impl InMemorySessionRepository {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(BTreeMap::new()),
            tickets: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Default for InMemorySessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRepository for InMemorySessionRepository {
    async fn save_session(&self, session: &Session) -> Result<(), RepositoryError> {
        self.sessions
            .lock()
            .insert(session.token.as_str().to_string(), session.clone());
        Ok(())
    }

    async fn get_session(&self, token: &SessionToken) -> Result<Option<Session>, RepositoryError> {
        Ok(self.sessions.lock().get(token.as_str()).cloned())
    }

    async fn delete_session(&self, token: &SessionToken) -> Result<bool, RepositoryError> {
        Ok(self.sessions.lock().remove(token.as_str()).is_some())
    }

    async fn purge_expired_sessions(&self, now: DateTime<Utc>) -> Result<usize, RepositoryError> {
        let mut sessions = self.sessions.lock();
        let len_before = sessions.len();
        sessions.retain(|_, s| s.expires_at > now);
        Ok(len_before - sessions.len())
    }

    async fn save_ticket(&self, ticket: &LoginTicket) -> Result<(), RepositoryError> {
        self.tickets
            .lock()
            .insert(ticket.ticket.as_str().to_string(), ticket.clone());
        Ok(())
    }

    async fn take_ticket(
        &self,
        ticket: &LoginTicketToken,
    ) -> Result<Option<LoginTicket>, RepositoryError> {
        Ok(self.tickets.lock().remove(ticket.as_str()))
    }

    async fn purge_expired_tickets(&self, now: DateTime<Utc>) -> Result<usize, RepositoryError> {
        let mut tickets = self.tickets.lock();
        let len_before = tickets.len();
        tickets.retain(|_, t| t.expires_at > now);
        Ok(len_before - tickets.len())
    }
}

#[cfg(test)]
#[path = "inmemory_session.test.rs"]
mod tests;
