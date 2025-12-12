use std::{env, sync::Arc};

use async_graphql::{Context, Error, Object};
use axum::Extension;
use hyper::{
    header::{AUTHORIZATION, COOKIE},
    HeaderMap, StatusCode,
};
use reqwest::Client;
use surrealdb::{engine::remote::ws::Client as SurrealClient, Surreal};
use tonic::transport::Channel;

use crate::graphql::schemas::{
    blog::{self, BlogPost, BlogStatus},
    shared::{self},
    user::{self, PublicSiteResources, UserResources},
};

use lib::{
    integration::grpc::clients::files_service::{
        files_service_client::FilesServiceClient, FetchFileNameRequest,
    },
    middleware::auth::graphql::confirm_authentication,
    utils::{
        custom_error::ExtendedError,
        grpc::{create_grpc_client, AuthMetaData},
        models::UploadedFileId,
    },
};

pub struct Query;

#[Object]
impl Query {
    /// Get all blog posts
    pub async fn fetch_blog_posts(
        &self,
        ctx: &Context<'_>,
        status: BlogStatus,
    ) -> async_graphql::Result<Vec<blog::BlogPost>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let mut query_result = db
            .query("SELECT *, ->has_comment->comment[*] AS comments FROM blog_post WHERE status = $status")
            .bind(("status", status))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query error: {}", e);
                Error::new("Internal Server Error".to_string())
            })?;

        let result: Vec<blog::BlogPost> = query_result.take(0).map_err(|e| {
            tracing::debug!("blog_posts deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        Ok(result)
    }

    /// Get user resources
    /// Combines all the resources of a logged-in user into a single graphql query
    pub async fn fetch_user_resources(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<UserResources> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let authenticated = confirm_authentication(headers).await?;
        let authenticated_ref = &authenticated;

        let mut query_results = db
            .query("SELECT *, ->has_comment->comment[*] AS comments FROM blog_post WHERE ->(user_id WHERE user_id = $external_user_id)")
            .bind(("external_user_id", authenticated_ref.sub.to_owned()))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query error: {}", e);
                Error::new("Internal Server Error".to_string())
            })?;

        let blog_posts: Vec<blog::BlogPost> = query_results.take(0).map_err(|e| {
            tracing::debug!("blog_posts deserialization error: {}", e);
            Error::new("Internal Server Error".to_string())
        })?;

        let user_resources = UserResources { blog_posts };

        Ok(user_resources)
    }

    /// Get site public resources
    /// Combines all the public resources of this site into a single graphql query
    pub async fn fetch_site_resources(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<PublicSiteResources> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let mut query_results = db
            .query("SELECT *, ->has_comment->comment[*] AS comments FROM blog_post WHERE status = 'Published'")
            .query("SELECT * FROM professional_details WHERE active = true")
            .query("SELECT *, ->uses_skill->skill[*] AS skills FROM portfolio")
            .query("SELECT *, ->achievement[*] AS achievements FROM resume")
            .query("SELECT * FROM skill")
            .query("SELECT * FROM service")
            .await
            .map_err(|e| {
                tracing::debug!("DB Query error: {}", e);
                Error::new("Internal Server Error".to_string())
            })?;

        let blog_posts: Vec<blog::BlogPost> = query_results.take(0).map_err(|e| {
            tracing::debug!("blog_posts deserialization error: {}", e);
            Error::new("Internal Server Error".to_string())
        })?;
        let professional_info: Vec<user::UserProfessionalInfo> =
            query_results.take(1).map_err(|e| {
                tracing::debug!("professional_info deserialization error: {}", e);
                Error::new("Internal Server Error".to_string())
            })?;
        let portfolio: Vec<user::UserPortfolio> = query_results.take(2).map_err(|e| {
            tracing::debug!("query_results: {:?}", query_results);
            tracing::debug!("portfolio deserialization error: {}", e);
            Error::new("Internal Server Error".to_string())
        })?;
        let resume: Vec<user::UserResume> = query_results.take(3).map_err(|e| {
            tracing::debug!("resume deserialization error: {}", e);
            Error::new("Internal Server Error".to_string())
        })?;
        let skills: Vec<user::UserSkill> = query_results.take(4).map_err(|e| {
            tracing::debug!("skills deserialization error: {}", e);
            Error::new("Internal Server Error".to_string())
        })?;
        let services: Vec<user::UserService> = query_results.take(5).map_err(|e| {
            tracing::debug!("services deserialization error: {}", e);
            Error::new("Internal Server Error".to_string())
        })?;

        let user_resources = PublicSiteResources {
            blog_posts,
            professional_info,
            portfolio,
            resume,
            skills,
            services,
        };

        Ok(user_resources)
    }

    /// Fetch Blog Content
    pub async fn fetch_single_blog_post(
        &self,
        ctx: &Context<'_>,
        blog_id_or_slug: String,
    ) -> async_graphql::Result<BlogPost> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let headers = ctx.data_opt::<HeaderMap>().ok_or_else(|| {
            ExtendedError::new("Unauthorized", StatusCode::UNAUTHORIZED.as_str()).build()
        })?;

        let mut blog_post_db_query = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $blog_id = type::thing('blog_post', $blog_id_or_slug);
                LET $blog_post = SELECT *, ->has_comment->comment[*] AS comments FROM ONLY blog_post WHERE id = $blog_id OR link = $blog_id_or_slug LIMIT 1;
                RETURN $blog_post;
                COMMIT TRANSACTION;
                "
            )
            .bind(("blog_id_or_slug", blog_id_or_slug))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query error: {}", e);
                ExtendedError::new(
                    "Server Error",
                    StatusCode::INTERNAL_SERVER_ERROR.as_str(),
                )
                .build()
            })?;

        let blog_post: Option<BlogPost> = blog_post_db_query.take(0)?;

        if blog_post.is_none() {
            return Err(
                ExtendedError::new("Blog post not found", StatusCode::NOT_FOUND.as_str()).build(),
            );
        }

        let mut blog_post = blog_post.unwrap();

        let mut file_id_db_query = db
            .query(
                "
                SELECT * FROM ONLY file_id WHERE id = $internal_file_id LIMIT 1
                ",
            )
            .bind(("internal_file_id", blog_post.content_file.clone()))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query error: {}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let uploaded_file: Option<UploadedFileId> = file_id_db_query.take(0)?;

        if uploaded_file.is_none() {
            return Err(
                ExtendedError::new("Content not found", StatusCode::NOT_FOUND.as_str()).build(),
            );
        }

        let mut request = tonic::Request::new(FetchFileNameRequest {
            file_id: uploaded_file.unwrap().file_id,
        });

        let auth_header = headers.get(AUTHORIZATION);
        let cookie_header = headers.get(COOKIE);

        let auth_metadata: AuthMetaData<FetchFileNameRequest> = AuthMetaData {
            auth_header,
            cookie_header,
            constructed_grpc_request: Some(&mut request),
        };

        let files_service_grpc = env::var("FILES_SERVICE_GRPC").map_err(|e| {
            tracing::debug!("Missing FILES_SERVICE_GRPC environment variable: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let mut files_grpc_client = create_grpc_client::<
            FetchFileNameRequest,
            FilesServiceClient<Channel>,
        >(&files_service_grpc, true, Some(auth_metadata))
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to Files service: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let res = files_grpc_client
            .fetch_file_name(request)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch file name from Files service: {}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let file_name: String = res.into_inner().file_name;

        let base_url = std::env::var("FILES_SERVICE").map_err(|e| {
            tracing::error!("FILES_SERVICE environment variable not set: {}", e);

            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;
        let url = format!("{}/view/{}", base_url, file_name);

        // Create an HTTP client
        let client = Client::new();

        // Fetch the content from the URL
        let text = client.get(&url).send().await.map_err(|e| {
            tracing::error!("Failed to connect to Files service: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;
        let content = text.text().await.map_err(|e| {
            tracing::error!("Failed to parse response from Files service: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;
        let blog_content = markdown::to_html(&content);

        blog_post.content = Some(blog_content);
        Ok(blog_post)
    }

    pub async fn fetch_messages(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<shared::Message>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let _authenticated = confirm_authentication(headers).await?;

        // fetch all messages in DB
        let mut query_results = db
            .query("SELECT * FROM message")
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        let messages: Vec<shared::Message> = query_results.take(0)?;

        Ok(messages)
    }
}
