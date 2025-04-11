use std::sync::Arc;

use async_graphql::{Context, Error, Object};
use axum::Extension;
use hyper::HeaderMap;
use surrealdb::{engine::remote::ws::Client as SurrealClient, Surreal};

use crate::graphql::schemas::{
    blog,
    shared::{self, SurrealRelationQueryResponse},
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

    /// Get a single blog post by link(Unique field which is also used as the file name for the markdown content, and the URL slug for the post)
    pub async fn get_single_blog_post(
        &self,
        ctx: &Context<'_>,
        link: String,
    ) -> async_graphql::Result<blog::BlogPost> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .unwrap();

        let mut result = db
            .query("SELECT * FROM blog_post WHERE link = $link LIMIT 1")
            .bind(("link", link))
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        let post: Option<blog::BlogPost> = result.take(0)?;

        match post {
            Some(post) => Ok(post),
            None => Err(Error::new("Post not found!")),
        }
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
                    .query("SELECT ->blog_post[*] AS blog_posts FROM ONLY type::thing($internal_user_id)")
                    .query("SELECT ->professional_details[*] AS professional_details FROM ONLY type::thing($internal_user_id)")
                    .query("SELECT *, ->uses_skill->skill[*] AS skills FROM portfolio WHERE ->(user_id WHERE user_id = $external_user_id)")
                    .query("SELECT *, ->achievement[*] AS achievements FROM resume WHERE ->(user_id WHERE user_id = $external_user_id)")
                    .query("SELECT ->skill[*] AS skills FROM ONLY type::thing($internal_user_id)")
                    .query("SELECT ->service[*] AS services FROM ONLY type::thing($internal_user_id)")
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

                let blog_posts: Option<SurrealRelationQueryResponse<blog::BlogPost>> =
                    query_results.take(0).map_err(|e| {
                        tracing::debug!("blog_posts deserialization error: {}", e);
                        Error::new("Internal Server Error".to_string())
                    })?;
                let professional_info: Option<
                    SurrealRelationQueryResponse<user::UserProfessionalInfo>,
                > = query_results.take(1).map_err(|e| {
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
                let skills: Option<SurrealRelationQueryResponse<user::UserSkill>> =
                    query_results.take(4).map_err(|e| {
                        tracing::debug!("skills deserialization error: {}", e);
                        Error::new("Internal Server Error".to_string())
                    })?;
                let services: Option<SurrealRelationQueryResponse<user::UserService>> =
                    query_results.take(5).map_err(|e| {
                        tracing::debug!("services deserialization error: {}", e);
                        Error::new("Internal Server Error".to_string())
                    })?;

                let user_resources = UserResources {
                    blog_posts: blog_posts
                        .unwrap()
                        .get("blog_posts")
                        .unwrap()
                        .into_iter()
                        .map(|blog| blog.to_owned())
                        .collect(),
                    professional_info: professional_info
                        .unwrap()
                        .get("professional_details")
                        .unwrap()
                        .into_iter()
                        .map(|info| info.to_owned())
                        .collect(),
                    portfolio,
                    resume,
                    skills: skills
                        .unwrap()
                        .get("skills")
                        .unwrap()
                        .into_iter()
                        .map(|skill| skill.to_owned())
                        .collect(),
                    services: services
                        .unwrap()
                        .get("services")
                        .unwrap()
                        .into_iter()
                        .map(|service| service.to_owned())
                        .collect(),
                };

                Ok(user_resources)
            }
            None => Err(Error::new("User not found!")),
        }
    }

    /// Get resume achievements by user_id and resume_id
    /// This query is used to get the achievements of a resume
    pub async fn get_resume_achievements(
        &self,
        ctx: &Context<'_>,
        resume_id: String,
    ) -> async_graphql::Result<Vec<user::ResumeAchievement>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .unwrap();

        let mut query_results = db
            .query("SELECT ->has_achievement->achievement.* AS achievements FROM type::thing($resume_id)")
            .bind(("resume_id", format!("resume:{}", resume_id)))
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        let achievements: Option<SurrealRelationQueryResponse<user::ResumeAchievement>> =
            query_results.take(0)?;

        Ok(achievements
            .unwrap()
            .get("achievements")
            .unwrap()
            .into_iter()
            .map(|achievement| achievement.to_owned())
            .collect())
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

    pub async fn get_skills(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<user::UserSkill>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .unwrap();

        let mut query_results = db
            .query("SELECT * FROM skill")
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        let skills: Vec<user::UserSkill> = query_results.take(0)?;

        Ok(skills)
    }
}
