//! Shared guardrails for refusing accidental wide container-workspace tasks.

use std::path::Path;

pub(crate) fn wide_scope_blocker(
    launch_root: &Path,
    task: &str,
    narrowed: bool,
) -> Option<&'static str> {
    if narrowed || explicit_wide_intent(task) || !looks_like_container(launch_root) {
        return None;
    }
    Some(
        "Wide container workspace blocked. Add an explicit child folder like ./1234, or say whole workspace for an intentional wide task.",
    )
}

fn looks_like_container(root: &Path) -> bool {
    if has_project_marker(root) {
        return false;
    }
    direct_child_dirs(root)
        .iter()
        .any(|name| !project_structure_dir(name))
}

fn has_project_marker(root: &Path) -> bool {
    [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "deno.json",
        "composer.json",
        "Gemfile",
        "Makefile",
        "index.html",
        "tsconfig.json",
        "vite.config.js",
        "vite.config.ts",
        "next.config.js",
        "next.config.ts",
    ]
    .iter()
    .any(|marker| root.join(marker).exists())
}

fn direct_child_dirs(root: &Path) -> Vec<String> {
    std::fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
                .filter(|name| !skip_dir(name))
                .collect()
        })
        .unwrap_or_default()
}

fn project_structure_dir(name: &str) -> bool {
    matches!(
        name,
        "app"
            | "assets"
            | "components"
            | "css"
            | "docs"
            | "images"
            | "img"
            | "js"
            | "lib"
            | "pages"
            | "public"
            | "scripts"
            | "src"
            | "static"
            | "styles"
            | "test"
            | "tests"
    )
}

fn skip_dir(name: &str) -> bool {
    matches!(name, ".git" | ".dsx" | "target" | "node_modules") || name.starts_with('.')
}

fn explicit_wide_intent(task: &str) -> bool {
    let lower = task.to_lowercase();
    [
        "whole workspace",
        "entire workspace",
        "all workspace",
        "whole repo",
        "entire repo",
        "workspace-wide",
        "весь workspace",
        "весь репозитор",
        "весь воркспейс",
        "всю текущую папку",
        "всю рабочую папку",
        "все дочерние папки",
    ]
    .iter()
    .any(|hint| lower.contains(hint))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_wide_container_workspace() {
        let root = temp_root("dsx_scope_container_block");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();

        let blocked = wide_scope_blocker(&root, "почини проект", false);

        assert!(blocked.unwrap().contains("child folder"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn allows_project_root_workspace() {
        let root = temp_root("dsx_scope_project_root");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\n").unwrap();

        assert!(wide_scope_blocker(&root, "почини проект", false).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn allows_explicit_wide_intent() {
        let root = temp_root("dsx_scope_wide_intent");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();

        assert!(wide_scope_blocker(&root, "проверь весь воркспейс целиком", false).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn allows_markerless_static_project_root() {
        let root = temp_root("dsx_scope_static_project");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("index.html"), "<main></main>").unwrap();

        assert!(wide_scope_blocker(&root, "доработай сайт", false).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn allows_narrowed_scope() {
        let root = temp_root("dsx_scope_narrowed");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();

        assert!(wide_scope_blocker(&root, "почини 1234", true).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
