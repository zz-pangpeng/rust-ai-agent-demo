use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::transport::stdio;
use rmcp::{ErrorData, ServiceExt, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
struct SelectExpense {
    category: String,
}

struct ExpenseServer {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl ExpenseServer {
    fn new() -> Self {
        let client = reqwest::Client::new();
        let token = "Default_token".to_string();
        let base_url = "http://localhost:3000".to_string();
        ExpenseServer {
            client,
            base_url,
            token,
        }
    }

    async fn response(
        result: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<CallToolResult, ErrorData> {
        match result {
            Ok(response) => {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("read response content fail: {e}"));
                if status.is_success() {
                    Ok(CallToolResult::success(vec![ContentBlock::text(body)]))
                } else {
                    Err(ErrorData::internal_error(format!("{status}: {body}"), None))
                }
            }
            Err(e) => Err(ErrorData::internal_error(
                format!("request failed: {e}"),
                None,
            )),
        }
    }
}

#[tool_router(server_handler)]
impl ExpenseServer {
    #[tool(description = "select expense by category")]
    async fn select_expense(
        &self,
        Parameters(params): Parameters<SelectExpense>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self
            .client
            .get(format!("{}/select/{}", self.base_url, params.category).as_str())
            .header("Authorization", self.token.clone())
            .send()
            .await;
        Self::response(result).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let server = ExpenseServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
