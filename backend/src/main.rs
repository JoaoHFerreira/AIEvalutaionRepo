use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use search_core::SearchResult;
use serde::Deserialize;
use std::net::SocketAddr;
use sqlx::{PgPool, FromRow};

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

#[derive(FromRow)]
struct DbSearchResult {
    id: i32,
    title: String,
    description: String,
}

impl From<DbSearchResult> for SearchResult {
    fn from(db: DbSearchResult) -> Self {
        Self {
            id: db.id as u32,
            title: db.title,
            description: db.description,
        }
    }
}

async fn search(
    State(pool): State<PgPool>,
    Query(params): Query<SearchQuery>
) -> Json<Vec<SearchResult>> {
    let query = params.q.unwrap_or_default();
    let ilike_query = format!("%{}%", query);

    let results = sqlx::query_as::<_, DbSearchResult>(
        "SELECT id, title, description FROM results WHERE title ILIKE $1 OR description ILIKE $1"
    )
    .bind(ilike_query)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let results = results.into_iter().map(SearchResult::from).collect();

    Json(results)
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to database");

    let app = Router::new()
        .route("/search", get(search))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
