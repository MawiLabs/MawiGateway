use mawi_protos::mawi::{LeaderServiceClient, ChatRequest, Message};
use poem_grpc::Request;

use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = poem_grpc::ClientConfig::builder()
        .uri("http://127.0.0.1:8030")
        .build()
        .unwrap();
    let mut client = LeaderServiceClient::new(client_config);

    let request = Request::new(ChatRequest {
        request_id: "test-1".to_string(),
        model_name: "gpt-4o".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Hello from the Specialized Collective!".to_string(),
        }],
        max_cost_limit: 0.5,
    });

    let mut stream = client.chat(request).await?.into_inner();

    while let Some(resp_result) = stream.next().await {
        let resp = resp_result?;
        if !resp.content_chunk.is_empty() {
             print!("{}", resp.content_chunk);
        }
        if let Some(usage) = resp.usage {
            println!("\n[Usage] Cost: ${:.4}", usage.total_cost);
        }
    }

    Ok(())
}
