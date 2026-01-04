use std::{env, sync::Arc};

use async_graphql::{Context, Object};
use axum::Extension;
use hyper::{
    header::{AUTHORIZATION, COOKIE},
    HeaderMap, StatusCode,
};
use reqwest::Client;
use surrealdb::{engine::remote::ws::Client as SurrealClient, RecordId, Surreal};
use tonic::transport::Channel;

use crate::graphql::schemas::{
    blog::{self, BlogPost, BlogStatus},
    shared::{self, BillingInterval, Ratecard},
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
        serialization::convert_float_to_string,
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
                tracing::error!("DB Query error: {}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
            })?;

        let result: Vec<blog::BlogPost> = query_result.take(0).map_err(|e| {
            tracing::error!("blog_posts deserialization error: {}", e);
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
                tracing::error!("DB Query error: {}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
            })?;

        let blog_posts: Vec<blog::BlogPost> = query_results.take(0).map_err(|e| {
            tracing::error!("blog_posts deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
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
                tracing::error!("DB Query error: {}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
            })?;

        let blog_posts: Vec<blog::BlogPost> = query_results.take(0).map_err(|e| {
            tracing::error!("blog_posts deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;
        let professional_info: Vec<user::UserProfessionalInfo> =
            query_results.take(1).map_err(|e| {
                tracing::error!("professional_info deserialization error: {}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;
        let portfolio: Vec<user::UserPortfolio> = query_results.take(2).map_err(|e| {
            tracing::error!("portfolio deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;
        let resume: Vec<user::UserResume> = query_results.take(3).map_err(|e| {
            tracing::error!("resume deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;
        let skills: Vec<user::UserSkill> = query_results.take(4).map_err(|e| {
            tracing::error!("skills deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;
        let services: Vec<user::UserService> = query_results.take(5).map_err(|e| {
            tracing::error!("services deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
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
        let mut query_results = db.query("SELECT * FROM message").await.map_err(|e| {
            tracing::error!("Query Error: {:?}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let messages: Vec<shared::Message> = query_results.take(0).map_err(|e| {
            tracing::error!("messages deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        Ok(messages)
    }

    pub async fn fetch_billing_rate(
        &self,
        ctx: &Context<'_>,
        billing_interval: BillingInterval,
        service_ids: Vec<String>,
    ) -> async_graphql::Result<String> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let service_record_ids = service_ids
            .iter()
            .map(|service_id| RecordId::from_table_key("service", service_id))
            .collect::<Vec<RecordId>>();

        // fetch all messages in DB
        let mut query_results = db
            .query(
                r#"
                BEGIN TRANSACTION;
                LET $billing_rates = (SELECT service.title AS service_title, service.id AS service_id, base_rate AS hourly_rate, base_rate * hour_week AS weekly_rate, base_rate * hour_week * 4 AS monthly_rate, base_rate * hour_week * 4 * 12 AS annual_rate FROM rate WHERE service.id IN $service_record_ids GROUP BY service_id);

                LET $rates_count = array::len($billing_rates);
                LET $bundle_discount = IF $rates_count > 1 {
                    1 - (0.8 * (1 - math::pow(math::e, (-0.4 * $rates_count))))
                } ELSE {
                    1
                };

                RETURN IF $billing_interval = 'Weekly' {
                    LET $weekly_rates = $billing_rates.map(|$billing_rate| $billing_rate.weekly_rate);
                    math::sum($weekly_rates)*$bundle_discount
                } ELSE IF $billing_interval = 'Monthly' {
                    LET $monthly_rates = $billing_rates.map(|$billing_rate| $billing_rate.monthly_rate);
                    math::sum($monthly_rates)*$bundle_discount
                } ELSE IF $billing_interval = 'Annual' {
                    LET $annual_rates = $billing_rates.map(|$billing_rate| $billing_rate.annual_rate);
                    math::sum($annual_rates)*$bundle_discount
                } ELSE {
                    LET $hourly_rates = $billing_rates.map(|$billing_rate| $billing_rate.hourly_rate);
                    math::sum($hourly_rates)*$bundle_discount
                };

                COMMIT TRANSACTION;
                "#
            )
            .bind(("billing_interval", billing_interval))
            .bind(("service_record_ids", service_record_ids))
            .await
            .map_err(|e| {
                tracing::error!("Query Error: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
            })?;

        let response: Option<f64> = query_results.take(0).map_err(|e| {
            tracing::error!("billing rate deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        match response {
            Some(billing_rate) => Ok(convert_float_to_string(billing_rate)),
            None => Err(ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()),
        }
    }

    pub async fn fetch_ratecards(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Ratecard>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        // fetch all ratecards in DB
        let mut query_results = db
            .query("SELECT *, ->contains->service.* AS services FROM ratecard")
            .await
            .map_err(|e| {
                tracing::error!("Query Error: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let ratecards: Vec<Ratecard> = query_results.take(0).map_err(|e| {
            tracing::error!("ratecards deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        Ok(ratecards)
    }
}
