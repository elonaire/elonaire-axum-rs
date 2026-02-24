mod database;
mod graphql;
mod utils;

use std::{
    env,
    io::{Error, ErrorKind},
    sync::Arc,
    time::Duration,
};

use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{http::HeaderValue, routing::post, serve, Extension, Router};
use graphql::resolvers::{mutation::Mutation, query::Query};
use hyper::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderMap, Method,
};
use surrealdb::{engine::remote::ws::Client, Surreal};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::CorsLayer;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use uuid::Uuid;

type MySchema = Schema<Query, Mutation, EmptySubscription>;

async fn graphql_handler(
    schema: Extension<MySchema>,
    db: Extension<Arc<Surreal<Client>>>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = req.0;

    let mut headers = headers.clone();
    let request_id = Uuid::new_v4();
    headers.insert(
        "x-request-id",
        HeaderValue::from_str(&request_id.to_string()).unwrap_or(HeaderValue::from_static("")),
    );
    request = request.data(db.clone());
    request = request.data(headers.clone());

    let operation_name = request.operation_name.clone();

    // Log request info
    tracing::info!("Executing GraphQL request: {:?}", &operation_name);
    let start = std::time::Instant::now();

    // Execute the GraphQL request
    let response = schema.execute(request).await;

    let duration = start.elapsed();
    tracing::info!("{:?} request processed in {:?}", operation_name, duration);

    // Debug the response
    if response.errors.len() > 0 {
        tracing::error!("GraphQL Error: {:?}", response.errors);
    } else {
        tracing::info!("GraphQL request completed without errors");
    }

    // Convert GraphQL response into the Axum response type
    response.into()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Persist the server logs to a file on a daily basis using "tracing_subscriber"
    let file_appender = tracing_appender::rolling::daily("./logs", "shared.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let stdout = std::io::stdout
        .with_filter(|meta| {
            meta.target() != "h2::codec::framed_write" && meta.target() != "h2::codec::framed_read"
        })
        .with_max_level(tracing::Level::DEBUG); // Log to console at DEBUG level

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(stdout.and(non_blocking))
        .init();

    let connection_pool = database::connection::create_db_connection()
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to the database: {}", e);
            Error::new(
                ErrorKind::ConnectionRefused,
                "Failed to connect to the database",
            )
        })?;
    let db = Arc::new(connection_pool);

    // Import env vars
    let deployment_env = env::var("ENVIRONMENT").unwrap_or_else(|_| "prod".to_string()); // default to production because it's the most secure
    let allowed_services_cors = env::var("ALLOWED_SERVICES_CORS").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "ALLOWED_SERVICES_CORS not set")
    })?;
    let shared_service_http_port = env::var("SHARED_SERVICE_HTTP_PORT").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "SHARED_SERVICE_HTTP_PORT not set")
    })?;

    // Initialize the schema builder
    let mut schema_builder = Schema::build(Query, Mutation, EmptySubscription);
    // Disable introspection & limit query depth in production
    schema_builder = match deployment_env.as_str() {
        "prod" => schema_builder.disable_introspection().limit_depth(5),
        _ => schema_builder,
    };

    let schema = schema_builder.finish();

    let origins: Vec<HeaderValue> = allowed_services_cors
        .as_str()
        .split(",")
        .filter_map(|endpoint| endpoint.trim().parse::<HeaderValue>().ok())
        .collect();

    // Allow bursts with up to five requests per IP address
    // and replenishes one element every two seconds
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(2)
        .burst_size(5)
        .finish()
        .unwrap();

    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);
    // a separate background task to clean up
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        tracing::info!("rate limiting storage size: {}", governor_limiter.len());
        governor_limiter.retain_recent();
    });

    let app = Router::new()
        .route("/", post(graphql_handler))
        .layer(GovernorLayer::new(governor_conf))
        .layer(Extension(schema))
        .layer(Extension(db))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE])
                .allow_credentials(true)
                .allow_methods(vec![Method::GET, Method::POST]),
        );

    match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", shared_service_http_port)).await {
        Ok(http_listener) => {
            let _http_server = serve(http_listener, app)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to create HTTP server: {}", e);
                })
                .ok();
        }
        Err(e) => {
            tracing::error!("Failed to create TCP listener: {}", e);
            return Err(Error::new(
                ErrorKind::ConnectionAborted,
                "Failed to create TCP listener",
            ));
        }
    };

    Ok(())
}
