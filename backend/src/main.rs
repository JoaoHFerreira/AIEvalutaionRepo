use axum::{
    extract::Query,
    routing::get,
    Json, Router,
};
use search_core::SearchResult;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

async fn search(Query(params): Query<SearchQuery>) -> Json<Vec<SearchResult>> {
    let query = params.q.unwrap_or_default().to_lowercase();

    let mock_data = vec![
        SearchResult {
            id: 1,
            title: "Rust Programming".to_string(),
            description: "A language empowering everyone to build reliable and efficient software.".to_string(),
        },
        SearchResult {
            id: 2,
            title: "WASM in Action".to_string(),
            description: "WebAssembly is a binary instruction format for a stack-based virtual machine.".to_string(),
        },
        SearchResult {
            id: 3,
            title: "Docker Compose".to_string(),
            description: "Define and run multi-container applications with Docker.".to_string(),
        },
    ];

    let results: Vec<SearchResult> = mock_data
        .into_iter()
        .filter(|item| {
            item.title.to_lowercase().contains(&query) ||
            item.description.to_lowercase().contains(&query)
        })
        .collect();

    Json(results)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/search", get(search));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
