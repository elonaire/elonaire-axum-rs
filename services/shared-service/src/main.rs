mod database;
mod graphql;
mod middleware;

use std::{env, sync::Arc};

use async_graphql::{http::{Credentials, GraphiQLSource}, EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{Extension, response::{IntoResponse, Html}, Router, routing::get, http::HeaderValue, serve};
use graphql::resolvers::{mutation::Mutation, query::Query};
use hyper::{header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE}, HeaderMap, Method};
use surrealdb::{Surreal, engine::remote::ws::Client, Result};
use tower_http::cors::CorsLayer;

type MySchema = Schema<Query, Mutation, EmptySubscription>;

async fn graphql_handler(
    schema: Extension<MySchema>,
    db: Extension<Arc<Surreal<Client>>>,
    headers: HeaderMap,
    req: GraphQLRequest,

) -> GraphQLResponse {
    let mut request = req.0;
    request = request.data(db.clone());
    request = request.data(headers.clone());

    tracing::info!("Executing GraphQL request: {:?}", request);

    // Execute the GraphQL request
    let response = schema.execute(request).await;

    // Log the response
    tracing::debug!("GraphQL response: {:?}", response);

    // Convert GraphQL response into the Axum response type
    response.into()
}

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/").title("Shared Service").credentials(Credentials::Include).finish())
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Arc::new(database::connection::create_db_connection().await.unwrap());

    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();

    let allowed_services_cors = env::var("ALLOWED_SERVICES_CORS")
        .expect("Missing the ALLOWED_SERVICES environment variable.");

    let origins: Vec<HeaderValue> = allowed_services_cors
        .as_str()
        .split(",")
        .into_iter()
        .map(|endpoint| endpoint.parse::<HeaderValue>().unwrap())
        .collect();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let app = Router::new()
        .route("/", get(graphiql).post(graphql_handler))
        .layer(Extension(schema))
        .layer(Extension(db))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE])
                .allow_credentials(true)
                .allow_methods(vec![Method::GET, Method::POST]),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3002").await.unwrap();
    serve(listener, app).await.unwrap();

    Ok(())
}
