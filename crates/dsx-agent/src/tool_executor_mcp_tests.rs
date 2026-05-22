use crate::tool_executor::{ToolContext, execute};
use dsx_core::types::PermissionMode;
use dsx_provider::streaming::ToolCallReady;

#[tokio::test]
async fn test_mcp_list_tools_from_project_config() {
    let tmp = std::env::temp_dir().join("dsx_test_agent_mcp_list");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join(".deepseek-code")).unwrap();

    std::fs::write(
        tmp.join(".deepseek-code").join("mcp.json"),
        r#"{
            "servers": [
                {
                    "name": "local",
                    "command": "python3",
                    "args": [
                        "-c",
                        "import json, sys\n\ndef read_frame():\n    header = b''\n    while not header.endswith(b'\\r\\n\\r\\n'):\n        b = sys.stdin.buffer.read(1)\n        if not b:\n            return None\n        header += b\n    length = 0\n    for line in header.decode().splitlines():\n        if line.lower().startswith('content-length:'):\n            length = int(line.split(':', 1)[1].strip())\n    return json.loads(sys.stdin.buffer.read(length))\n\ndef write_frame(obj):\n    data = json.dumps(obj).encode()\n    sys.stdout.buffer.write(f'Content-Length: {len(data)}\\r\\n\\r\\n'.encode() + data)\n    sys.stdout.buffer.flush()\n\nwhile True:\n    msg = read_frame()\n    if msg is None:\n        break\n    method = msg.get('method')\n    if method == 'initialize':\n        write_frame({'jsonrpc':'2.0','id':msg['id'],'result':{'protocolVersion':'2024-11-05','capabilities':{},'serverInfo':{'name':'fake'}}})\n    elif method == 'notifications/initialized':\n        pass\n    elif method == 'tools/list':\n        write_frame({'jsonrpc':'2.0','id':msg['id'],'result':{'tools':[{'name':'echo','description':'Echo args','inputSchema':{'type':'object','properties':{'value':{'type':'number'}}}}]}})\n"
                    ]
                }
            ]
        }"#,
    )
    .unwrap();

    let call = ToolCallReady {
        id: "call_1".into(),
        name: "mcp_list_tools".into(),
        arguments: r#"{"server":"local"}"#.into(),
    };
    let ctx = ToolContext {
        workspace: tmp.clone(),
        mode: PermissionMode::Ask,
        approval_tx: None,
    };

    let result = execute(&call, &ctx).await;
    assert!(result.success, "{}", result.content);
    assert!(result.content.contains("Server: local"));
    assert!(result.content.contains("echo"));
    assert!(result.content.contains("inputSchema"));

    let _ = std::fs::remove_dir_all(&tmp);
}
