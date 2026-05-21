//! Compact task-state memory used for bounded context assembly.

use chrono::Utc;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TaskSummary {
    pub project_root: String,
    pub goal: String,
    pub done: String,
    pub plan: String,
    pub last_changes: String,
    pub next_step: String,
    pub active_scope: String,
    pub constraints: String,
    pub architecture: String,
    pub updated_at: String,
}

impl TaskSummary {
    pub fn new(project_root: &str) -> Self {
        Self {
            project_root: project_root.to_string(),
            updated_at: Utc::now().to_rfc3339(),
            ..Self::default()
        }
    }

    pub fn compact_text(&self) -> String {
        let fields = [
            ("Goal", self.goal.as_str()),
            ("Done", self.done.as_str()),
            ("Plan", self.plan.as_str()),
            ("Last changes", self.last_changes.as_str()),
            ("Next step", self.next_step.as_str()),
            ("Active scope", self.active_scope.as_str()),
            ("Constraints", self.constraints.as_str()),
            ("Architecture", self.architecture.as_str()),
        ];

        fields
            .into_iter()
            .filter(|(_, value)| !value.trim().is_empty())
            .map(|(name, value)| format!("{name}: {}", value.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub async fn load_task_summary(
    pool: &SqlitePool,
    project_root: &str,
) -> anyhow::Result<Option<TaskSummary>> {
    let row = sqlx::query_as::<_, TaskSummaryRow>(
        r#"
        SELECT project_root, goal, done, plan, last_changes, next_step,
               active_scope, constraints, architecture, updated_at
        FROM task_summaries
        WHERE project_root = ?
        "#,
    )
    .bind(project_root)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Into::into))
}

pub async fn upsert_task_summary(pool: &SqlitePool, summary: &TaskSummary) -> anyhow::Result<()> {
    let updated_at = if summary.updated_at.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        summary.updated_at.clone()
    };

    sqlx::query(
        r#"
        INSERT INTO task_summaries (
            project_root, goal, done, plan, last_changes, next_step,
            active_scope, constraints, architecture, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(project_root) DO UPDATE SET
            goal = excluded.goal,
            done = excluded.done,
            plan = excluded.plan,
            last_changes = excluded.last_changes,
            next_step = excluded.next_step,
            active_scope = excluded.active_scope,
            constraints = excluded.constraints,
            architecture = excluded.architecture,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&summary.project_root)
    .bind(&summary.goal)
    .bind(&summary.done)
    .bind(&summary.plan)
    .bind(&summary.last_changes)
    .bind(&summary.next_step)
    .bind(&summary.active_scope)
    .bind(&summary.constraints)
    .bind(&summary.architecture)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(sqlx::FromRow)]
struct TaskSummaryRow {
    project_root: String,
    goal: String,
    done: String,
    plan: String,
    last_changes: String,
    next_step: String,
    active_scope: String,
    constraints: String,
    architecture: String,
    updated_at: String,
}

impl From<TaskSummaryRow> for TaskSummary {
    fn from(row: TaskSummaryRow) -> Self {
        Self {
            project_root: row.project_root,
            goal: row.goal,
            done: row.done,
            plan: row.plan,
            last_changes: row.last_changes,
            next_step: row.next_step,
            active_scope: row.active_scope,
            constraints: row.constraints,
            architecture: row.architecture,
            updated_at: row.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stores_and_loads_task_summary() {
        let db_path =
            std::env::temp_dir().join(format!("dsx_task_summary_{}.db", uuid::Uuid::new_v4()));
        let pool = crate::open(&db_path).await.unwrap();
        let mut summary = TaskSummary::new("/tmp/project");
        summary.goal = "goal".into();
        summary.done = "done".into();
        summary.next_step = "next".into();

        upsert_task_summary(&pool, &summary).await.unwrap();
        let loaded = load_task_summary(&pool, "/tmp/project")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded.goal, "goal");
        assert_eq!(loaded.done, "done");
        assert_eq!(loaded.next_step, "next");

        let _ = pool.close().await;
        let _ = std::fs::remove_file(db_path);
    }
}
