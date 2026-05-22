//! CLI handlers for scope previews and session listing.

use std::path::Path;

pub fn run_scope_preview(project_root: &Path, task: &str) {
    let scope = crate::task_scope::resolve_task_scope(project_root, task);
    println!("Task scope preview:");
    println!("  Task: {}", task_preview(task));
    println!("  Launch: {}", scope.launch_label);
    println!("  Active: {}", scope.active_label);
    println!(
        "  Status: {}",
        if scope.narrowed { "NARROWED" } else { "WIDE" }
    );
    println!(
        "  Reason: {}",
        if scope.narrowed {
            "Task selected a subfolder; tools and indexing will be locked there."
        } else {
            "No explicit subfolder was selected; launch workspace remains active."
        }
    );
    if !scope.narrowed {
        println!("  Warning: add an explicit folder like ./1234 to narrow scope.");
    }
    println!(
        "  Active exists: {}",
        if scope.active_root.exists() {
            "yes"
        } else {
            "no"
        }
    );
}

pub fn task_preview(task: &str) -> String {
    const MAX_CHARS: usize = 240;
    let cleaned = dsx_agent::brief::clean_task_input(task);
    let mut preview: String = cleaned.chars().take(MAX_CHARS).collect();
    if cleaned.chars().count() > MAX_CHARS {
        preview.push_str("...");
    }
    preview
}

pub async fn list_sessions(project_root: &Path) {
    let db_path = project_root.join(".dsx").join("sessions.db");
    match dsx_memory::open(&db_path).await {
        Ok(pool) => {
            let sm = dsx_session::SessionManager::new(pool);
            match sm.list(20).await {
                Ok(sessions) => print_sessions(&sessions),
                Err(e) => println!("Error: {e}"),
            }
        }
        Err(e) => println!("DB error: {e}"),
    }
}

fn print_sessions(sessions: &[dsx_session::Session]) {
    if sessions.is_empty() {
        println!("No sessions yet.");
        return;
    }
    println!("Recent sessions:");
    for s in sessions {
        println!(
            "  {}  {}  {}  {} msgs",
            &s.id[..8.min(s.id.len())],
            s.mode,
            &s.created_at[..19],
            s.message_count,
        );
    }
}
