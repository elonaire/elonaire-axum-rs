use std::sync::Arc;

use async_graphql::{Context, Error, Object};
use axum::Extension;
use hyper::HeaderMap;
use surrealdb::{engine::remote::ws::Client as SurrealClient, Surreal};

use crate::graphql::schemas::{
    blog,
    shared::{self},
    user::{self, UserResources},
};

use lib::middleware::auth::graphql::check_auth_from_acl;

pub struct Query;

#[Object]
impl Query {
    /// Get all blog posts
    pub async fn get_blog_posts(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<blog::BlogPost>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .unwrap();

        let result = db
            .select("blog_post")
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        Ok(result)
    }

    /// Get user resources
    /// Combines all the resources of a user into a single graphql query
    pub async fn get_user_resources(
        &self,
        ctx: &Context<'_>,
        user_id: String,
    ) -> async_graphql::Result<UserResources> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .unwrap();

        let mut user_query_result = db
            .query("SELECT * FROM user_id WHERE user_id = $user_id LIMIT 1")
            .bind(("user_id", user_id))
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        let user: Option<user::User> = user_query_result.take(0)?;

        match user {
            Some(user) => {
                let mut query_results = db
                    .query("SELECT *, ->has_comment->comment[*] AS comments FROM blog_post WHERE ->(user_id WHERE user_id = $external_user_id)")
                    .query("SELECT * FROM professional_details WHERE ->(user_id WHERE user_id = $external_user_id) AND active = true")
                    .query("SELECT *, ->uses_skill->skill[*] AS skills FROM portfolio WHERE ->(user_id WHERE user_id = $external_user_id)")
                    .query("SELECT *, ->achievement[*] AS achievements FROM resume WHERE ->(user_id WHERE user_id = $external_user_id)")
                    .query("SELECT * FROM skill WHERE ->(user_id WHERE user_id = $external_user_id)")
                    .query("SELECT * FROM service WHERE ->(user_id WHERE user_id = $external_user_id)")
                    .bind((
                        "internal_user_id",
                        format!(
                            "user_id:{}",
                            user.id.as_ref().map(|t| &t.id).expect("id").to_raw()
                        ),
                    ))
                    .bind(("external_user_id", user.user_id))
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

                let user_resources = UserResources {
                    blog_posts,
                    professional_info,
                    portfolio,
                    resume,
                    skills,
                    services,
                };

                Ok(user_resources)
            }
            None => Err(Error::new("User not found!")),
        }
    }

    pub async fn get_messages(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<shared::Message>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .unwrap();

        let headers = ctx.data::<HeaderMap>().unwrap();

        let _auth_res_from_acl = check_auth_from_acl(headers).await?;

        // fetch all messages in DB
        let mut query_results = db
            .query("SELECT * FROM message")
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        let messages: Vec<shared::Message> = query_results.take(0)?;

        Ok(messages)
    }
}
