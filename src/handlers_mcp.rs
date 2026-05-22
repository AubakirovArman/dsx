//! CLI handlers for direct MCP inspection.

pub async fn run_mcp_list(command: &str, args: &[String]) -> anyhow::Result<()> {
    let mut client = dsx_mcp::McpClient::connect_stdio(command, args).await?;
    let tools = client.list_tools().await?;
    if tools.is_empty() {
        println!("No MCP tools exposed.");
    } else {
        println!("MCP tools:");
        for tool in &tools {
            let description = tool.description.as_deref().unwrap_or("");
            println!("  {}  {}", tool.name, description);
        }
    }
    client.shutdown().await?;
    Ok(())
}

pub async fn run_mcp_call(
    command: &str,
    args: &[String],
    tool: &str,
    arguments_json: &str,
) -> anyhow::Result<()> {
    let arguments = serde_json::from_str::<serde_json::Value>(arguments_json)?;
    let mut client = dsx_mcp::McpClient::connect_stdio(command, args).await?;
    let result = client.call_tool(tool, arguments).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    client.shutdown().await?;
    Ok(())
}
