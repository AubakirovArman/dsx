use super::*;
use serde_json::json;
use tokio::io::duplex;

#[tokio::test]
async fn frame_round_trip() {
    let (mut client, mut server) = duplex(4096);
    let expected = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
    });

    let writer = tokio::spawn(async move {
        write_frame(&mut client, &expected).await.unwrap();
    });
    let actual = read_frame(&mut server).await.unwrap();
    writer.await.unwrap();

    assert_eq!(actual["method"], "tools/list");
    assert_eq!(actual["id"], 1);
}

#[tokio::test]
async fn stdio_client_lists_and_calls_tools() {
    let script = r#"
import json, sys

def read_frame():
    header = b""
    while not header.endswith(b"\r\n\r\n"):
        b = sys.stdin.buffer.read(1)
        if not b:
            return None
        header += b
    length = 0
    for line in header.decode().splitlines():
        if line.lower().startswith("content-length:"):
            length = int(line.split(":", 1)[1].strip())
    return json.loads(sys.stdin.buffer.read(length))

def write_frame(obj):
    data = json.dumps(obj).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(data)}\r\n\r\n".encode() + data)
    sys.stdout.buffer.flush()

while True:
    msg = read_frame()
    if msg is None:
        break
    method = msg.get("method")
    if method == "initialize":
        write_frame({"jsonrpc":"2.0","id":msg["id"],"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"fake"}}})
    elif method == "notifications/initialized":
        pass
    elif method == "tools/list":
        write_frame({"jsonrpc":"2.0","id":msg["id"],"result":{"tools":[{"name":"echo","description":"Echo args","inputSchema":{"type":"object"}}]}})
    elif method == "tools/call":
        write_frame({"jsonrpc":"2.0","id":msg["id"],"result":{"content":[{"type":"text","text":json.dumps(msg["params"]["arguments"])}]}})
"#;
    let args = vec!["-c".to_string(), script.to_string()];
    let mut client = McpClient::connect_stdio("python3", &args).await.unwrap();
    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");

    let result = client.call_tool("echo", json!({"value": 7})).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content[0]["type"], "text");

    client.shutdown().await.unwrap();
}

#[test]
fn load_config_reads_project_deepseek_code_mcp() {
    let tmp = std::env::temp_dir().join("dsx_test_mcp_config_project");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join(".deepseek-code")).unwrap();
    std::fs::write(
        tmp.join(".deepseek-code").join("mcp.json"),
        r#"{
            "servers": [
                { "name": "local", "command": "python3", "args": ["-c", "print(1)"] }
            ]
        }"#,
    )
    .unwrap();

    let config = load_config(&tmp).unwrap();
    let server = config
        .servers
        .iter()
        .find(|server| server.name == "local")
        .unwrap();
    assert_eq!(server.command.as_deref(), Some("python3"));
    assert!(server.enabled);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn parse_config_supports_mcp_servers_map_and_disabled_flag() {
    let config = crate::config::parse_config_str(
        r#"{
            "mcpServers": {
                "echo": {
                    "command": "python3",
                    "args": ["server.py"],
                    "disabled": true
                }
            }
        }"#,
    )
    .unwrap();

    assert_eq!(config.servers.len(), 1);
    assert_eq!(config.servers[0].name, "echo");
    assert!(!config.servers[0].enabled);
}
