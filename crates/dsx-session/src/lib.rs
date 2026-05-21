//! DSX Session Manager — create, persist, and resume agent sessions.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SessionManager {
    pool: SqlitePool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub id: String,
    pub project_root: String,
    pub mode: String,
    pub created_at: String,
    pub message_count: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub id: String,
    pub session_id: String,
    pub ts: String,
    pub type_: String,
    pub data_json: String,
}

impl SessionManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new session.
    pub async fn create(&self, project_root: &str, mode: &str) -> anyhow::Result<Session> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO sessions (id, project_root, mode, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(project_root)
        .bind(mode)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(Session {
            id,
            project_root: project_root.into(),
            mode: mode.into(),
            created_at: now,
            message_count: 0,
        })
    }

    /// Get a session by its ID.
    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Session>> {
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT id, project_root, mode, created_at, message_count FROM sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| Session {
            id: r.id,
            project_root: r.project_root,
            mode: r.mode,
            created_at: r.created_at,
            message_count: r.message_count,
        }))
    }

    /// List recent sessions.
    pub async fn list(&self, limit: u32) -> anyhow::Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT id, project_root, mode, created_at, message_count FROM sessions ORDER BY updated_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| Session {
                id: r.id,
                project_root: r.project_root,
                mode: r.mode,
                created_at: r.created_at,
                message_count: r.message_count,
            })
            .collect())
    }

    /// Record an event in a session.
    pub async fn record_event(
        &self,
        session_id: &str,
        event_type: &str,
        data: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO events (id, session_id, ts, type, data_json) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(session_id)
        .bind(&now)
        .bind(event_type)
        .bind(serde_json::to_string(data)?)
        .execute(&self.pool)
        .await?;
        // Bump session message count and updated_at
        sqlx::query(
            "UPDATE sessions SET message_count = message_count + 1, updated_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Retrieve events for a session, ordered by timestamp ascending.
    pub async fn get_events(&self, session_id: &str) -> anyhow::Result<Vec<Event>> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT id, session_id, ts, type, data_json FROM events WHERE session_id = ? ORDER BY ts ASC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| Event {
                id: r.id,
                session_id: r.session_id,
                ts: r.ts,
                type_: r.type_,
                data_json: r.data_json,
            })
            .collect())
    }

    /// Retrieve the latest events without loading an entire long session.
    pub async fn get_recent_events(
        &self,
        session_id: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<Event>> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT id, session_id, ts, type, data_json
             FROM events
             WHERE session_id = ?
             ORDER BY ts DESC
             LIMIT ?",
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let mut events = rows
            .into_iter()
            .map(|r| Event {
                id: r.id,
                session_id: r.session_id,
                ts: r.ts,
                type_: r.type_,
                data_json: r.data_json,
            })
            .collect::<Vec<_>>();
        events.reverse();
        Ok(events)
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: String,
    project_root: String,
    mode: String,
    created_at: String,
    message_count: i64,
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: String,
    session_id: String,
    ts: String,
    #[sqlx(rename = "type")]
    type_: String,
    data_json: String,
}
