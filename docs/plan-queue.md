# Queue

## Table

```sql
CREATE TABLE IF NOT EXISTS queue_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    reward_title TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    result_slot_id INTEGER REFERENCES roulette_slots(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

## Status Flow

```
                    POST /api/queue
                    (external svc)
                         │
                         ▼
                   ┌─────────┐
                   │ Pending │
                   └────┬────┘
                        │
              ┌─────────┼──────────┐
              │         │          │
              │  POST /queue/next  │
              │    (dock) │        │
              │         │          │
              │         ▼          │
              │  ┌───────────┐     │
              │  │  Spinning │     │
              │  └─────┬─────┘     │
              │        │           │
              │        │           │
              │        ├───────────────┐
              │        │           │   │
              │        │    ┌──────┴──┐│
              │        │    │  Error  ││
              │        │    └─────────┘│
              │        │   (background │
              │        │    task)      │
              │        │              │
              │        ▼              │
              │ ┌───────────┐         │
              │ │ Completed │         │
              │ └───────────┘         │
              │ POST /queue/{id}/compl│
              │     (widget)          │
              │                      │
              └──────────────────────┘

              POST /queue/{id}/cancel
                (dock, only Pending)
                        │
                        ▼
                 ┌───────────┐
                 │ Cancelled │
                 └───────────┘
```

## API Endpoints

| Method | Path | Caller | Description |
|--------|------|--------|-------------|
| `POST` | `/api/queue` | External service | Enqueue. Body: `{ platform, platform_user_id, platform_username, reward_title? }` |
| `GET` | `/api/queue` | Dock / Widget | List entries, `?status=` filter |
| `GET` | `/api/queue/{id}` | Dock / Widget | Get entry with user info |
| `GET` | `/api/queue/next` | Dock | Peek — first in queue (no state change) |
| `POST` | `/api/queue/next` | Dock | Dequeue → roll → Spinning → return result |
| `POST` | `/api/queue/{id}/complete` | Widget | Confirm → Completed |
| `POST` | `/api/queue/{id}/cancel` | Dock | Cancel (only Pending) |
| `GET` | `/api/queue/stats` | Dock | Counts by status |

## Domain Model

```rust
pub struct QueueEntryId(u32);

pub enum QueueStatus {
    Pending,
    Spinning,
    Completed,
    Error,
    Cancelled,
}

pub struct QueueEntry {
    pub id: QueueEntryId,
    pub user_id: UserId,
    pub reward_title: Option<String>,
    pub status: QueueStatus,
    pub result_slot_id: Option<RouletteSlotId>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub struct QueueStats {
    pub pending: u32,
    pub spinning: u32,
    pub completed: u32,
    pub error: u32,
    pub cancelled: u32,
}
```

## Traits

```rust
pub trait QueueRepository: Send + Sync {
    fn enqueue(&self, entry: QueueEntry) -> impl Future<Output = Result<QueueEntry, RepositoryError>> + Send;
    fn peek_oldest_pending(&self) -> impl Future<Output = Result<Option<QueueEntry>, RepositoryError>> + Send;
    fn dequeue_oldest_pending(&self) -> impl Future<Output = Result<Option<QueueEntry>, RepositoryError>> + Send;
    fn list(&self, status: Option<QueueStatus>) -> impl Future<Output = Result<Vec<QueueEntry>, RepositoryError>> + Send;
    fn get_by_id(&self, id: QueueEntryId) -> impl Future<Output = Result<QueueEntry, RepositoryError>> + Send;
    fn update_status(&self, id: QueueEntryId, status: QueueStatus, result_slot_id: Option<RouletteSlotId>) -> impl Future<Output = Result<QueueEntry, RepositoryError>> + Send;
    fn count_by_status(&self) -> impl Future<Output = Result<QueueStats, RepositoryError>> + Send;
    fn find_spinning_older_than(&self, cutoff: NaiveDateTime) -> impl Future<Output = Result<Vec<QueueEntry>, RepositoryError>> + Send;
}

pub trait EventPublisher: Send + Sync {
    fn publish_spin_started(&self, entry: &QueueEntry, slot: &RouletteSlot, user: &User) -> impl Future<Output = Result<(), EventError>> + Send;
    fn publish_spin_completed(&self, entry: &QueueEntry) -> impl Future<Output = Result<(), EventError>> + Send;
    fn publish_spin_error(&self, entry: &QueueEntry) -> impl Future<Output = Result<(), EventError>> + Send;
}

pub trait TimeoutConfig: Send + Sync {
    fn roulette_timeout_secs(&self) -> u64;
}
```

## Behaviours

**`POST /api/queue/next`:**
- Если есть Spinning → `409 Conflict`
- Если нет Pending → `404 Not Found`
- Иначе: dequeue oldest Pending → roll → Spinning → `200` + entry + slot
- Публикует `publish_spin_started`

**`POST /api/queue/{id}/complete`:**
- Если не Spinning → `409 Conflict`
- Иначе: Completed → `200`
- Публикует `publish_spin_completed`

**`POST /api/queue/{id}/cancel`:**
- Если не Pending → `409 Conflict`
- Иначе: Cancelled → `200`

**Timeout:** background task проверяет Spinning старше `roulette_timeout_secs` → Error + `publish_spin_error`
