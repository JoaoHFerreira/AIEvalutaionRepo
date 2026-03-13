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
    let query = params.q.unwrap_or_default();

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

    let results = search_core::filter_results(mock_data, &query);

    Json(results)
}

fn app() -> Router {
    Router::new().route("/search", get(search))
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app()).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_search_endpoint() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/search?q=rust")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let results: Vec<SearchResult> = serde_json::from_slice(&body).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming");
    }

    #[tokio::test]
    async fn test_search_no_query() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/search")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let results: Vec<SearchResult> = serde_json::from_slice(&body).unwrap();
        assert_eq!(results.len(), 3);
    }
}
