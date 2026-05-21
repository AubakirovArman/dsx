use super::*;

#[test]
fn test_is_complete_json() {
    assert!(!is_complete_json(""));
    assert!(!is_complete_json("{"));
    assert!(is_complete_json("{}"));
    assert!(is_complete_json(r#"{"path": "src/main.rs"}"#));
    assert!(is_complete_json(r#"{"pattern": "fn main"}"#));
}

#[test]
fn test_tool_accumulator_basic() {
    let mut acc = ToolAccumulator::default();
    let deltas1 = vec![StreamToolCallDelta {
        index: 0,
        id: Some("call_1".into()),
        type_: None,
        function: Some(FunctionDelta {
            name: Some("read_file".into()),
            arguments: Some(r#"{"path":"#.into()),
        }),
    }];
    let r1 = acc.ingest(&deltas1);
    assert!(r1.is_empty(), "should not be ready yet");

    let deltas2 = vec![StreamToolCallDelta {
        index: 0,
        id: None,
        type_: None,
        function: Some(FunctionDelta {
            name: None,
            arguments: Some(r#""src/main.rs"}"#.into()),
        }),
    }];
    let r2 = acc.ingest(&deltas2);
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].name, "read_file");
    assert_eq!(r2[0].arguments, r#"{"path":"src/main.rs"}"#);
}

#[test]
fn test_tool_accumulator_multiple_tools() {
    let mut acc = ToolAccumulator::default();
    let deltas = vec![
        StreamToolCallDelta {
            index: 0,
            id: Some("call_a".into()),
            type_: None,
            function: Some(FunctionDelta {
                name: Some("grep".into()),
                arguments: Some(r#"{"pattern":"todo"}"#.into()),
            }),
        },
        StreamToolCallDelta {
            index: 1,
            id: Some("call_b".into()),
            type_: None,
            function: Some(FunctionDelta {
                name: Some("read_file".into()),
                arguments: Some(r#"{"path":"README.md"}"#.into()),
            }),
        },
    ];
    let ready = acc.ingest(&deltas);
    assert_eq!(ready.len(), 2, "both tools should be ready");
}
