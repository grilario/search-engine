use std::sync::Arc;

use askama::Template;
use askama_axum::Response;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, IntoMakeService},
    Router,
};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{
    parser::parse_document,
    storage::{Document, Storage},
};

pub fn make_routes(state: Arc<Storage>) -> IntoMakeService<Router> {
    let app = Router::new()
        .route("/", get(home))
        .route("/search", get(search))
        .route("/insert", get(insert))
        .with_state(state);

    app.into_make_service()
}

#[derive(Template)]
#[template(path = "index.html")]
struct HomeTemplate;

pub async fn home() -> impl IntoResponse {
    HomeTemplate
}

#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate {
    title: String,
    results: Vec<Document>,
}

#[derive(Deserialize)]
pub struct QuerySearch {
    #[serde(rename = "q")]
    query: Option<String>,
}

pub async fn search(
    State(storage): State<Arc<Storage>>,
    Query(QuerySearch { query }): Query<QuerySearch>,
) -> Result<Response, StatusCode> {
    let query = match query {
        Some(query) => query,
        None => return Ok(Redirect::to("/").into_response()),
    };

    let results = storage
        .search(query.clone(), 10)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(SearchTemplate {
        title: query,
        results,
    }
    .into_response())
}

#[derive(Template)]
#[template(path = "insert.html")]
struct InsertTemplate;

#[derive(Deserialize)]
pub struct QueryInsert {
    url: Option<String>,
}

pub async fn insert(
    State(storage): State<Arc<Storage>>,
    Query(query): Query<QueryInsert>,
) -> Result<impl IntoResponse, StatusCode> {
    let QueryInsert { url } = query;

    if url.is_none() {
        return Ok(InsertTemplate);
    }

    let response = make_request(url.clone().unwrap()).await.map_err(|e| {
        eprint!("{}", e);
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    let (title, description, texts) = parse_document(url.clone().unwrap(), response).await.map_err(|e| {
        eprint!("{}", e);
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    storage
        .insert(url.unwrap(), title, description, texts)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(InsertTemplate)
}

async fn make_request(link: String) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36")
        .build()?;
    let response = client.get(link).send().await?.text().await?;

    Ok(response)
}
