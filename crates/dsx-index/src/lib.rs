//! DSX Index — codebase indexing, symbol extraction, and search.

mod db;
mod extract;
mod scan;
mod types;

pub use db::{build_symbol_index, search_symbols};
pub use extract::extract_symbols;
pub use scan::{detect_language, scan_project, search_files};
pub use types::{FileMatch, IndexedSymbol, Symbol};

#[cfg(test)]
mod tests;
