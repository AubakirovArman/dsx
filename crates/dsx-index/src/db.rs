//! SQLite-backed symbol index build and search.

use crate::extract::extract_symbols;
use crate::scan::scan_project;
use crate::types::IndexedSymbol;
use std::path::Path;

pub async fn build_symbol_index(root: &Path, pool: &sqlx::SqlitePool) -> anyhow::Result<usize> {
    let files = scan_project(root)?;
    let mut count = 0;
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM symbols WHERE project_root = ?")
        .bind(root.display().to_string())
        .execute(&mut *tx)
        .await?;

    for file_path_str in files {
        count += index_file(root, &file_path_str, &mut tx).await?;
    }

    tx.commit().await?;
    Ok(count)
}

pub async fn search_symbols(
    root: &Path,
    pool: &sqlx::SqlitePool,
    query: &str,
    limit: u32,
) -> anyhow::Result<Vec<IndexedSymbol>> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let pattern = format!("%{}%", query.trim());
    let prefix = format!("{}%", query.trim());
    let rows = sqlx::query_as::<_, IndexedSymbol>(
        r#"
        SELECT path, language, kind, name, start_line, end_line, signature
        FROM symbols
        WHERE project_root = ?
          AND (name LIKE ? OR signature LIKE ? OR path LIKE ?)
        ORDER BY
          CASE
            WHEN name = ? THEN 0
            WHEN name LIKE ? THEN 1
            WHEN signature LIKE ? THEN 2
            ELSE 3
          END,
          path ASC,
          start_line ASC
        LIMIT ?
        "#,
    )
    .bind(root.display().to_string())
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .bind(query.trim())
    .bind(&prefix)
    .bind(&pattern)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn index_file(
    root: &Path,
    file_path_str: &str,
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> anyhow::Result<usize> {
    let full_path = root.join(file_path_str);
    let Some(ext) = full_path.extension().and_then(|s| s.to_str()) else {
        return Ok(0);
    };
    if !matches!(ext, "rs" | "ts" | "js" | "py") {
        return Ok(0);
    }
    let Ok(content) = tokio::fs::read_to_string(&full_path).await else {
        return Ok(0);
    };

    let mut count = 0;
    for symbol in extract_symbols(&content, ext) {
        insert_symbol(root, file_path_str, ext, &symbol, tx).await?;
        count += 1;
    }
    Ok(count)
}

async fn insert_symbol(
    root: &Path,
    file_path: &str,
    ext: &str,
    symbol: &crate::types::Symbol,
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO symbols
         (id, project_root, path, file_hash, language, kind, name, start_line, end_line, signature)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(root.display().to_string())
    .bind(file_path)
    .bind("")
    .bind(ext)
    .bind(&symbol.kind)
    .bind(&symbol.name)
    .bind(symbol.start_line as i32)
    .bind(symbol.end_line as i32)
    .bind(&symbol.signature)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
