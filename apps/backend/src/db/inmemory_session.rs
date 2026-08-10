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
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn session_roundtrip() {
        let repo = InMemorySessionRepository::new();
        let session = Session {
            token: SessionToken::new("tok"),
            twitch_user_id: "123".to_string(),
            twitch_user_name: Some("sapushka_".to_string()),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::from_secs(3600),
        };
        repo.save_session(&session).await.unwrap();

        let fetched = repo.get_session(&session.token).await.unwrap().unwrap();
        assert_eq!(fetched.twitch_user_id, "123");

        assert!(repo.delete_session(&session.token).await.unwrap());
        assert!(!repo.delete_session(&session.token).await.unwrap());
    }

    #[tokio::test]
    async fn ticket_take_is_destructive() {
        let repo = InMemorySessionRepository::new();
        let ticket = LoginTicket {
            ticket: LoginTicketToken::new("tic"),
            twitch_user_id: "123".to_string(),
            twitch_user_name: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::from_secs(600),
        };
        repo.save_ticket(&ticket).await.unwrap();

        let taken = repo.take_ticket(&ticket.ticket).await.unwrap().unwrap();
        assert_eq!(taken.twitch_user_id, "123");
        assert!(repo.take_ticket(&ticket.ticket).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn purge_removes_only_expired() {
        let repo = InMemorySessionRepository::new();
        let now = Utc::now();

        let stale = Session {
            token: SessionToken::new("stale"),
            twitch_user_id: "1".to_string(),
            twitch_user_name: None,
            created_at: now,
            expires_at: now - Duration::from_secs(1),
        };
        let fresh = Session {
            token: SessionToken::new("fresh"),
            twitch_user_id: "2".to_string(),
            twitch_user_name: None,
            created_at: now,
            expires_at: now + Duration::from_secs(1),
        };
        repo.save_session(&stale).await.unwrap();
        repo.save_session(&fresh).await.unwrap();

        assert_eq!(repo.purge_expired_sessions(now).await.unwrap(), 1);
        assert!(repo.get_session(&stale.token).await.unwrap().is_none());
        assert!(repo.get_session(&fresh.token).await.unwrap().is_some());
    }
}
