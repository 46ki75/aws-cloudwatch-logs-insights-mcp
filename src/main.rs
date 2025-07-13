use aws_config::BehaviorVersion;
use rmcp::{
    ServiceExt,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    serde_json, tool, tool_handler, tool_router,
    transport::stdio,
};

#[derive(Clone)]
pub struct Counter {
    client: aws_sdk_cloudwatchlogs::Client,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddParams {
    /// The end of the time range to query. The range is inclusive,
    /// so the specified end time is included in the query. Specified as epoch time,
    /// the number of seconds since `January 1, 1970, 00:00:00 UTC`.
    pub end_time: i64,

    /// The beginning of the time range to query. The range is inclusive,
    /// so the specified start time is included in the query. Specified as epoch time,
    /// the number of seconds since `January 1, 1970, 00:00:00 UTC`.
    pub start_time: Option<i64>,

    /// The maximum number of log entries to return. Defaults to 1000 if not specified.
    pub limit: Option<i32>,

    /// The keyword to search for in log messages. Only messages containing this keyword will be returned.
    pub keyword: String,
}

#[tool_router]
impl Counter {
    async fn new() -> Self {
        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;

        let client = aws_sdk_cloudwatchlogs::Client::new(&sdk_config);

        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Query CloudWatch Logs for messages containing the specified keyword within a time range"
    )]
    async fn query(
        &self,
        Parameters(AddParams {
            end_time,
            start_time,
            limit,
            keyword,
        }): Parameters<AddParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client.clone();

        let query_string = format!(
            r#"
        fields @timestamp, @message, @logStream, @log
        | sort @timestamp desc
        | filter @message like /.*{}.*/
        "#,
            keyword
        );

        let query_id = client
            .start_query()
            .log_group_name("/dev/46ki75/web/cloudwatch/log_group/lambda_http")
            .set_end_time(Some(end_time))
            .set_start_time(start_time)
            .limit(limit.unwrap_or(1000))
            .query_string(query_string)
            .send()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                ErrorData::new(
                    ErrorCode(1),
                    format!("An error occurred when calling the API: {}", e),
                    Some(e.to_string().into()),
                )
            })?
            .query_id;

        #[derive(serde::Serialize)]
        struct Record {
            pub field: ::std::option::Option<::std::string::String>,
            pub value: ::std::option::Option<::std::string::String>,
        }

        let results = client
            .get_query_results()
            .set_query_id(query_id)
            .send()
            .await
            .map_err(|e| {
                println!("{:?}", e.as_service_error());
                ErrorData::new(
                    ErrorCode(1),
                    "An error occurred when calling the API",
                    Some(e.to_string().into()),
                )
            })?
            .results
            .map(|r| {
                r.into_iter()
                    .map(|r| {
                        r.into_iter()
                            .map(|record| Record {
                                field: record.field,
                                value: record.value,
                            })
                            .collect::<Vec<Record>>()
                    })
                    .collect::<Vec<Vec<Record>>>()
            })
            .unwrap();

        let results_json = serde_json::to_string(&results).unwrap();

        Ok(CallToolResult::success(vec![Content::text(results_json)]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for Counter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = Counter::new().await.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test() {
        let c = Counter::new().await;

        let _ = c
            .query(Parameters(AddParams {
                end_time: 1752364800,         // サンプルのエポック秒
                start_time: Some(1752278400), // サンプルのエポック秒
                limit: Some(10),              // サンプルのリミット
                keyword: "error".to_string(), // サンプルのキーワード
            }))
            .await
            .unwrap();
    }
}
