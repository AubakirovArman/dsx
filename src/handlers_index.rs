//! CLI handlers for symbol indexing.

use std::path::Path;

pub async fn run_index_build(project_root: &Path) -> anyhow::Result<()> {
    let db_path = project_root.join(".dsx").join("sessions.db");
    let pool = dsx_memory::open(&db_path).await?;
    let count = dsx_index::build_symbol_index(project_root, &pool).await?;
    println!("Indexed {count} symbols into {}", db_path.display());
    Ok(())
}

pub async fn run_index_search(project_root: &Path, query: &str, limit: u32) -> anyhow::Result<()> {
    let db_path = project_root.join(".dsx").join("sessions.db");
    let pool = dsx_memory::open(&db_path).await?;
    let symbols = dsx_index::search_symbols(project_root, &pool, query, limit).await?;
    let files = dsx_index::search_files(project_root, query, limit as usize)?;

    println!("Symbols:");
    if symbols.is_empty() {
        println!("  (none)");
    } else {
        for symbol in &symbols {
            println!(
                "  {}:{}  {} {}  {}",
                symbol.path, symbol.start_line, symbol.kind, symbol.name, symbol.signature
            );
        }
    }

    println!("File matches:");
    if files.is_empty() {
        println!("  (none)");
    } else {
        for file_match in &files {
            println!(
                "  {}:{}  {}",
                file_match.path, file_match.line, file_match.text
            );
        }
    }
    Ok(())
}
