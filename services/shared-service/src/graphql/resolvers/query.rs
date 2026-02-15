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

use crate::{
    graphql::schemas::{
        blog::{self, BlogPost, BlogStatus, FetchBlogPostsQueryFilters},
        shared::{
            self, BillingInterval, GraphQLApiResponse, Ratecard, ServiceRate, ServiceRequest,
        },
        user::{self, PublicSiteResources, UserResources},
    },
    utils::{read_time::calculate_read_time_minutes, syntax_highlighter::SyntaxHighlighter},
};

use lib::{
    integration::grpc::clients::files_service::{
        files_service_client::FilesServiceClient, FetchFileNameRequest,
    },
    middleware::auth::graphql::confirm_authentication,
    utils::{
        api_response::synthesize_graphql_response,
        custom_error::ExtendedError,
        grpc::{confirm_authorization, create_grpc_client, AuthMetaData},
        models::{AdminPrivilege, AuthorizationConstraint, UploadedFileId},
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
        filters: Option<FetchBlogPostsQueryFilters>,
    ) -> async_graphql::Result<GraphQLApiResponse<Vec<blog::BlogPost>>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        tracing::debug!("passed filters: {:?}", filters);

        let mut query_result = db
            .query("
                <set> array::flatten([
                   	(SELECT *, (<-wrote<-user_id)[0].* AS author, (SELECT *, (<-wrote<-user_id)[0][*] AS author, array::len(->has_reply) AS reply_count FROM ->has_comment->comment) AS comments FROM blog_post WHERE ($filters.status != NONE AND $filters.is_featured = NONE AND status = $filters.status) OR ($filters.status != NONE AND $filters.is_featured != NONE AND is_featured = $filters.is_featured AND status = $filters.status) OR ($filters.status = NONE AND $filters.is_featured != NONE AND is_featured = $filters.is_featured) FETCH content_file)
                ]);
                ")
            .bind(("filters", filters))
            .await
            .map_err(|e| {
                tracing::error!("DB Query error: {}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
            })?;

        let mut result: Vec<blog::BlogPost> = query_result.take(0).map_err(|e| {
            tracing::error!("blog_posts deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let highlighter = SyntaxHighlighter::new();

        for blog_post in &mut result {
            if let Some(content) = &blog_post.content {
                let read_time_minutes = calculate_read_time_minutes(&content);
                blog_post.read_time = Some(read_time_minutes);
                let highlighted = highlighter.highlight_html(&content);
                blog_post.content = Some(highlighted);
            };
        }

        let api_response = synthesize_graphql_response(ctx, &result, None).ok_or_else(|| {
            tracing::error!("Failed to synthesize response!");
            ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
        })?;
        Ok(api_response.into())
    }

    /// Get user resources
    /// Combines all the resources of a logged-in user into a single graphql query
    pub async fn fetch_user_resources(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<GraphQLApiResponse<UserResources>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let authenticated = confirm_authentication(ctx).await?;
        let authenticated_ref = &authenticated;

        let mut query_results = db
            .query("SELECT *, (SELECT *, (<-wrote<-user_id)[0][*] AS author, array::len(->has_reply) AS reply_count FROM ->has_comment->comment) AS comments FROM blog_post WHERE <-wrote<-(user_id WHERE user_id = $external_user_id) FETCH content_file")
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

        let api_response =
            synthesize_graphql_response(ctx, &user_resources, Some(authenticated_ref)).ok_or_else(
                || {
                    tracing::error!("Failed to synthesize response!");
                    ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
                },
            )?;
        Ok(api_response.into())
    }

    /// Get site public resources
    /// Combines all the public resources of this site into a single graphql query
    pub async fn fetch_site_resources(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<GraphQLApiResponse<PublicSiteResources>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let mut query_results = db
            .query("SELECT *, (<-wrote<-user_id)[0].* AS author, (SELECT *, (<-wrote<-user_id)[0][*] AS author, array::len(->has_reply) AS reply_count FROM ->has_comment->comment) AS comments FROM blog_post WHERE status = 'Published' FETCH content_file")
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

        let api_response =
            synthesize_graphql_response(ctx, &user_resources, None).ok_or_else(|| {
                tracing::error!("Failed to synthesize response!");
                ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
            })?;
        Ok(api_response.into())
    }

    /// Fetch Blog Content
    pub async fn fetch_single_blog_post(
        &self,
        ctx: &Context<'_>,
        blog_id_or_slug: String,
    ) -> async_graphql::Result<GraphQLApiResponse<BlogPost>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let mut blog_post_db_query = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $blog_id = type::thing('blog_post', $blog_id_or_slug);
                LET $blog_post = SELECT *, (<-wrote<-user_id)[0].* AS author, (SELECT *, (<-wrote<-user_id)[0][*] AS author, array::len(->has_reply) AS reply_count FROM ->has_comment->comment) AS comments FROM ONLY blog_post WHERE id = $blog_id_or_slug OR link = $blog_id_or_slug LIMIT 1 FETCH content_file;
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

        match blog_post {
            Some(mut blog_post) => {
                let highlighter = SyntaxHighlighter::new();

                if let Some(content) = blog_post.content {
                    let read_time_minutes = calculate_read_time_minutes(&content);
                    blog_post.read_time = Some(read_time_minutes);

                    let highlighted = highlighter.highlight_html(&content);
                    blog_post.content = Some(highlighted);
                }

                let api_response =
                    synthesize_graphql_response(ctx, &blog_post, None).ok_or_else(|| {
                        tracing::error!("Failed to synthesize response!");
                        ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
                    })?;
                Ok(api_response.into())
            }
            None => Err(
                ExtendedError::new("Blog post not found", StatusCode::NOT_FOUND.as_str()).build(),
            ),
        }
    }

    pub async fn fetch_messages(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<GraphQLApiResponse<Vec<shared::Message>>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let authenticated = confirm_authentication(ctx).await?;
        let authenticated_ref = &authenticated;

        // fetch all messages in DB
        let mut query_results = db.query("SELECT * FROM message").await.map_err(|e| {
            tracing::error!("Query Error: {:?}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let messages: Vec<shared::Message> = query_results.take(0).map_err(|e| {
            tracing::error!("messages deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let api_response = synthesize_graphql_response(ctx, &messages, Some(authenticated_ref))
            .ok_or_else(|| {
                tracing::error!("Failed to synthesize response!");
                ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
            })?;
        Ok(api_response.into())
    }

    pub async fn fetch_billing_rate(
        &self,
        ctx: &Context<'_>,
        billing_interval: BillingInterval,
        service_ids: Vec<String>,
    ) -> async_graphql::Result<GraphQLApiResponse<String>> {
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
            Some(billing_rate) => {
                let api_response =
                    synthesize_graphql_response(ctx, &convert_float_to_string(billing_rate), None)
                        .ok_or_else(|| {
                            tracing::error!("Failed to synthesize response!");
                            ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str())
                                .build()
                        })?;
                Ok(api_response.into())
            }
            None => Err(ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()),
        }
    }

    pub async fn fetch_ratecards(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<GraphQLApiResponse<Vec<Ratecard>>> {
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

        let api_response = synthesize_graphql_response(ctx, &ratecards, None).ok_or_else(|| {
            tracing::error!("Failed to synthesize response!");
            ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
        })?;
        Ok(api_response.into())
    }

    pub async fn fetch_service_rates(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<GraphQLApiResponse<Vec<ServiceRate>>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        // fetch all service rates in DB
        let mut query_results = db
            .query("SELECT * FROM rate FETCH service, currency_id")
            .await
            .map_err(|e| {
                tracing::error!("Query Error: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let service_rates: Vec<ServiceRate> = query_results.take(0).map_err(|e| {
            tracing::error!("rate deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let api_response =
            synthesize_graphql_response(ctx, &service_rates, None).ok_or_else(|| {
                tracing::error!("Failed to synthesize response!");
                ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
            })?;
        Ok(api_response.into())
    }

    pub async fn fetch_service_requests(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<GraphQLApiResponse<Vec<ServiceRequest>>> {
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

        let authenticated = confirm_authentication(ctx).await?;
        let authenticated_ref = &authenticated;

        let authorization_constraint = AuthorizationConstraint {
            permissions: vec!["read:service_request".into()],
            privilege: AdminPrivilege::Admin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        // fetch all service rates in DB
        let mut query_results = db
            .query("SELECT * FROM service_request FETCH supporting_docs")
            .await
            .map_err(|e| {
                tracing::error!("Query Error: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let service_requests: Vec<ServiceRequest> = query_results.take(0).map_err(|e| {
            tracing::error!("rate deserialization error: {}", e);
            ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str()).build()
        })?;

        let api_response =
            synthesize_graphql_response(ctx, &service_requests, Some(authenticated_ref))
                .ok_or_else(|| {
                    tracing::error!("Failed to synthesize response!");
                    ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
                })?;
        Ok(api_response.into())
    }
}
