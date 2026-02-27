use std::{
    fs::File,
    io::{Error, ErrorKind, Read},
};

use dotenvy::dotenv;
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};

pub async fn create_db_connection() -> Result<Surreal<Client>, Error> {
    dotenv().ok();
    println!("Creating Surreal database connection...");
    let db_host = std::env::var("DATABASE_HOST_SHARED").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "DATABASE_HOST_SHARED not set")
    })?;
    let db_port = std::env::var("DATABASE_PORT_SHARED").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "DATABASE_PORT_SHARED not set")
    })?;
    let db_user = std::env::var("DATABASE_USER").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "DATABASE_USER not set")
    })?;
    let db_password = std::env::var("DATABASE_PASSWORD").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "DATABASE_PASSWORD not set")
    })?;
    let db_name = std::env::var("DATABASE_NAME_SHARED").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "DATABASE_NAME_SHARED not set")
    })?;
    let db_namespace = std::env::var("DATABASE_NAMESPACE").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "DATABASE_NAMESPACE not set")
    })?;

    let db_url = format!("{}:{}", db_host, db_port);
    tracing::info!("DB URL: {}", db_url);
    let db = Surreal::new::<Ws>(db_url).await.map_err(|e| {
        tracing::error!("Connection error: {:?}", e);
        Error::new(ErrorKind::Other, "Failed to connect to database")
    })?;

    db.signin(Root {
        username: &db_user,
        password: &db_password,
    })
    .await
    .map_err(|e| {
        tracing::error!("Sign In error: {:?}", e);
        Error::new(ErrorKind::Other, "Failed to connect to database")
    })?;

    // Select a specific namespace and database
    db.use_ns(&db_namespace)
        .use_db(&db_name)
        .await
        .map_err(|e| {
            tracing::error!("Namespace error: {:?}", e);
            Error::new(ErrorKind::Other, "Failed to connect to database")
        })?;

    // Perform migrations
    // println!("{:?}", env::current_dir());
    let file_name = std::env::var("DATABASE_SCHEMA_FILE_PATH").map_err(|e| {
        tracing::error!("Config Error: {}", e);
        Error::new(ErrorKind::Other, "DATABASE_SCHEMA_FILE_PATH not set")
    })?;

    let schema = read_file_to_string(&file_name)?;
    db.query(&schema).await.map_err(|e| {
        tracing::error!("Query Error: {}", e);
        Error::new(ErrorKind::Other, e)
    })?;

    Ok(db)
}

fn read_file_to_string(filename: &str) -> Result<String, Error> {
    let mut file = File::open(filename).map_err(|e| {
        tracing::error!("File Error: {}", e);
        Error::new(ErrorKind::Other, "Server Error")
    })?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| {
        tracing::error!("File Read Error: {}", e);
        Error::new(ErrorKind::Other, "Server Error")
    })?;
    Ok(contents)
}
