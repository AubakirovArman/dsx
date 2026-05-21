use super::*;

#[test]
fn test_extract_rust_symbols() {
    let code = r#"
        pub struct AppState {
            pub count: usize,
        }

        impl AppState {
            pub fn new() -> Self {
                AppState { count: 0 }
            }
        }
    "#;
    let symbols = extract_symbols(code, "rs");
    assert_eq!(symbols.len(), 2);

    assert_eq!(symbols[0].name, "AppState");
    assert_eq!(symbols[0].kind, "struct");
    assert_eq!(symbols[0].start_line, 2);
    assert_eq!(symbols[0].end_line, 4);

    assert_eq!(symbols[1].name, "new");
    assert_eq!(symbols[1].kind, "function");
    assert_eq!(symbols[1].start_line, 7);
    assert_eq!(symbols[1].end_line, 9);
}

#[test]
fn test_search_files() {
    let tmp = std::env::temp_dir().join("dsx_index_search_files");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(tmp.join("src/lib.rs"), "pub fn target_symbol() {}\n").unwrap();

    let matches = search_files(&tmp, "TARGET", 10).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].path, "src/lib.rs");
    assert_eq!(matches[0].line, 1);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn test_search_symbols() {
    let tmp = std::env::temp_dir().join("dsx_index_search_symbols");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(
        tmp.join("src/lib.rs"),
        "pub struct Thing {}\nimpl Thing { pub fn make() -> Self { Thing {} } }\n",
    )
    .unwrap();

    let db = tmp.join(".dsx/sessions.db");
    let pool = dsx_memory::open(&db).await.unwrap();
    build_symbol_index(&tmp, &pool).await.unwrap();
    let matches = search_symbols(&tmp, &pool, "Thing", 10).await.unwrap();
    assert!(matches.iter().any(|symbol| symbol.name == "Thing"));

    let _ = std::fs::remove_dir_all(&tmp);
}
