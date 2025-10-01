use std::sync::Arc;

use async_graphql::{Context, Error, Object};
use axum::Extension;
// use gql_client::Client as GQLClient;

use hyper::{HeaderMap, StatusCode};
use lib::utils::models::{ForeignKey, UploadedFile, User};
use lib::{
    integration::foreign_key::add_foreign_key_if_not_exists,
    middleware::auth::graphql::check_auth_from_acl, utils::custom_error::ExtendedError,
};
use surrealdb::{engine::remote::ws::Client as SurrealClient, Surreal};

use crate::graphql::schemas::user::UserProfessionalInfo;
use crate::graphql::schemas::{blog, shared, user};

// const CHUNK_SIZE: u64 = 1024 * 1024 * 5; // 5MB

pub struct Mutation;

#[Object]
impl Mutation {
    /// Create new professional details
    async fn create_professional_details(
        &self,
        ctx: &Context<'_>,
        professional_details: UserProfessionalInfo,
    ) -> async_graphql::Result<UserProfessionalInfo> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error extracting Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;
        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong!",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build());
        }

        tracing::debug!("professional_details: {:?}", professional_details);
        tracing::debug!("user_id: {:?}", auth_res_from_acl.sub);

        let mut database_transaction = db
        .query(
            "
            BEGIN TRANSACTION;
            LET $user = (SELECT VALUE id FROM ONLY type::table($table) WHERE user_id = $user_id LIMIT 1);
            LET $professional_details = (RELATE $user -> professional_details -> $user CONTENT $professional_details_input RETURN AFTER)[0];
            RETURN $professional_details;
            COMMIT TRANSACTION;
            "
        )
        .bind(("professional_details_input", professional_details))
        .bind(("table", "user_id"))
        .bind(("user_id", auth_res_from_acl.sub))
        .await
        .map_err(|e| {
            tracing::debug!("DB Query Failed: {}", e);
            // Error::new("Internal Server Error.")
            ExtendedError::new(
                "Failed",
                Some(StatusCode::BAD_REQUEST.as_u16()),
            )
            .build()
        })?;

        let response: Option<UserProfessionalInfo> = database_transaction.take(0).map_err(|e| {
            tracing::debug!("Deserialization Failed: {}", e);
            tracing::debug!("database_transaction: {:?}", database_transaction);
            // Error::new("Internal Server Error.")
            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(professional_info) => Ok(professional_info),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }

        // Ok(response)
    }

    /// Create a new user service
    pub async fn create_user_service(
        &self,
        ctx: &Context<'_>,
        user_service: user::UserServiceInput,
    ) -> async_graphql::Result<user::UserService> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let added_user_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            User,
        >(db, user_fk)
        .await;

        if added_user_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Invalid Input Data",
                Some(StatusCode::BAD_REQUEST.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
            BEGIN TRANSACTION;
            LET $user = (SELECT VALUE id FROM ONLY type::table($table) WHERE user_id = $user_id LIMIT 1);

            LET $user_service = (RELATE $user->service->$user CONTENT $user_service_input RETURN AFTER)[0];
            RETURN $user_service;
            COMMIT TRANSACTION;
            ",
            )
            .bind(("user_service_input", user_service))
            .bind(("table", "user_id"))
            .bind(("user_id", auth_res_from_acl.sub))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query Error: {}", e);

                ExtendedError::new(
                    "Failed",
                    Some(StatusCode::BAD_REQUEST.as_u16()),
                )
                .build()
            })?;

        let response: Option<user::UserService> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(user_service) => Ok(user_service),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }

        // Ok(response)
    }

    /// Create a new user project/portfolio item
    pub async fn create_portfolio_item(
        &self,
        ctx: &Context<'_>,
        portfolio_item: user::UserPortfolioInput,
    ) -> async_graphql::Result<user::UserPortfolio> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Invalid Input Data",
                Some(StatusCode::BAD_REQUEST.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $user = (SELECT VALUE id FROM ONLY type::table($table) WHERE user_id = $user_id LIMIT 1);
                LET $portfolio_item = SELECT *, ->uses_skill->skill.* AS skills FROM ONLY (RELATE $user -> portfolio -> $user CONTENT $portfolio_item_input RETURN AFTER)[0] LIMIT 1;
                RETURN $portfolio_item;
                COMMIT TRANSACTION;
            ",
            )
            .bind(("portfolio_item_input", portfolio_item))
            .bind(("table", "user_id"))
            .bind(("user_id", auth_res_from_acl.sub))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query Error: {}", e);

                ExtendedError::new(
                    "Failed",
                    Some(StatusCode::BAD_REQUEST.as_u16()),
                )
                .build()
            })?;

        let response: Option<user::UserPortfolio> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(user_portfolio) => Ok(user_portfolio),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }

        // Ok(response)
    }

    /// Create a new user resume item
    pub async fn create_resume_item(
        &self,
        ctx: &Context<'_>,
        resume_item: user::UserResumeInput,
    ) -> async_graphql::Result<user::UserResume> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::debug!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
            BEGIN TRANSACTION;
            LET $user = (SELECT VALUE id FROM type::table($table) WHERE user_id = $user_id LIMIT 1);

            LET $resume_item = (RELATE $user->resume->$user CONTENT $resume_item_input RETURN AFTER)[0];
            RETURN $resume_item;
            COMMIT TRANSACTION;
            ",
            )
            .bind(("resume_item_input", resume_item))
            .bind(("table", "user_id"))
            .bind(("user_id", auth_res_from_acl.sub))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query Error: {}", e);

                ExtendedError::new(
                    "Failed",
                    Some(StatusCode::BAD_REQUEST.as_u16()),
                )
                .build()
            })?;

        let response: Option<user::UserResume> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(resume_item) => Ok(resume_item),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }

        // Ok(response)
    }

    /// Create a new user resume item achievement
    pub async fn create_resume_item_achievement(
        &self,
        ctx: &Context<'_>,
        resume_item_achievement: user::ResumeAchievement,
        resume_id: String,
    ) -> async_graphql::Result<user::ResumeAchievement> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
        .query(
            "
            BEGIN TRANSACTION;
            LET $resume = type::thing($resume_id);

            LET $resume_item_achievement = (RELATE $resume->achievement->$resume CONTENT $resume_item_achievement_input RETURN AFTER)[0];
            RETURN $resume_item_achievement;
            COMMIT TRANSACTION;
            "
        )
        .bind(("resume_item_achievement_input", resume_item_achievement))
        .bind(("table", "resume"))
        .bind(("resume_id", format!("resume:{}", resume_id)))
        .await
        .map_err(|e| {
            tracing::debug!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                Some(StatusCode::BAD_REQUEST.as_u16()),
            )
            .build()
        })?;

        let response: Option<user::ResumeAchievement> =
            database_transaction.take(0).map_err(|e| {
                tracing::error!("Deserialization Error: {:?}", e);

                ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
            })?;

        match response {
            Some(resume_achievement) => Ok(resume_achievement),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }

        // Ok(response)
    }

    /// Create a new user skill
    pub async fn create_skill(
        &self,
        ctx: &Context<'_>,
        skill: user::UserSkillInput,
    ) -> async_graphql::Result<user::UserSkill> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
            BEGIN TRANSACTION;
            LET $user = (SELECT VALUE id FROM type::table($table) WHERE user_id = $user_id LIMIT 1);

            LET $skill = (RELATE $user->skill->$user CONTENT $skill_input RETURN AFTER)[0];
            RETURN $skill;
            COMMIT TRANSACTION;
            ",
            )
            .bind(("skill_input", skill))
            .bind(("table", "user_id"))
            .bind(("user_id", auth_res_from_acl.sub))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query Error: {}", e);

                ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
            })?;

        let response: Option<user::UserSkill> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(user_skill) => Ok(user_skill),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }

        // Ok(response)
    }

    /// Create a new blog post
    pub async fn create_blog_post(
        &self,
        ctx: &Context<'_>,
        blog_post: blog::BlogPostInput,
    ) -> async_graphql::Result<blog::BlogPost> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let content_file_fk = ForeignKey {
            table: "file_id".to_string(),
            column: "file_id".to_string(),
            foreign_key: blog_post.content_file.clone(),
        };

        let added_user = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            User,
        >(db, user_fk)
        .await
        .ok_or_else(|| {
            tracing::error!("Failed to add user_id");
            ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let added_file = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UploadedFile,
        >(db, content_file_fk)
        .await
        .ok_or_else(|| {
            tracing::error!("Failed to add content_file");
            ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let mut database_transaction = db
            .query(
                "
            BEGIN TRANSACTION;
            LET $user = (SELECT VALUE id FROM type::table($table) WHERE user_id = $user_id LIMIT 1);
            LET $content_file = type::thing('file_id', $file_id);
            IF !$content_file.exists() {
                THROW 'Invalid Input';
            };

            LET $blog_post = (RELATE $user->blog_post->$user CONTENT {
                title: $blog_post_input.title,
                short_description: $blog_post_input.short_description,
                status: $blog_post_input.status,
                thumbnail: $blog_post_input.thumbnail,
                content_file: $content_file,
                other_images: $blog_post_input.other_images,
                category: $blog_post_input.category
            } RETURN AFTER)[0];
            RETURN $blog_post;
            COMMIT TRANSACTION;
            ",
            )
            .bind(("blog_post_input", blog_post))
            .bind(("table", "user_id"))
            .bind(("user_id", added_user.user_id))
            .bind(("file_id", added_file.id))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query Error: {}", e);
                Error::new("Internal Server Error")
            })?;

        let response: Option<blog::BlogPost> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(blog_post) => Ok(blog_post),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }
    }

    /// Add a comment to a blog post
    pub async fn add_comment_to_blog_post(
        &self,
        ctx: &Context<'_>,
        blog_comment: blog::BlogComment,
        blog_post_id: String,
    ) -> async_graphql::Result<blog::BlogComment> {
        // TODO: Might have to allow anonymous comments?
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
        .query(
            "
            BEGIN TRANSACTION;
            -- Get the user
            LET $user = (SELECT VALUE id FROM ONLY type::table($user_table) WHERE user_id = $user_id LIMIT 1);
            -- Get the blog post
            LET $blog_post = (SELECT VALUE id FROM ONLY type::table($blog_table) WHERE id = type::thing($blog_post_id) LIMIT 1);
            -- Create comment
            LET $blog_comment = CREATE comment CONTENT $blog_comment_input;
            LET $blog_comment_id = (SELECT VALUE id FROM $blog_comment);

            -- Relate the comment to the blog post
            RELATE $blog_post->has_comment->$blog_comment_id;
            -- Relate the comment to the user
            RELATE $user->wrote->$blog_comment_id;
            RETURN $blog_comment;
            COMMIT TRANSACTION;
            "
        )
        .bind(("blog_comment_input", blog_comment))
        .bind(("blog_table", "blog_post"))
        .bind(("blog_post_id", format!("blog_post:{}", blog_post_id)))
        .bind(("user_id", auth_res_from_acl.sub))
        .bind(("user_table", "user_id"))
        .await
        .map_err(|e| {
            tracing::debug!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                Some(StatusCode::BAD_REQUEST.as_u16()),
            )
            .build()
        })?;

        let response: Option<blog::BlogComment> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(blog_comment) => Ok(blog_comment),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }
    }

    /// Reply to a comment
    pub async fn reply_to_a_comment(
        &self,
        ctx: &Context<'_>,
        blog_comment: blog::BlogComment,
        comment_id: String,
    ) -> async_graphql::Result<blog::BlogComment> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
        .query(
            "
            BEGIN TRANSACTION;
            -- Get the user, parent comment and blog post
            LET $parent_comment = (SELECT VALUE id FROM ONLY type::table($comment_table) WHERE id = type::thing($comment_id) LIMIT 1);
            LET $user = (SELECT VALUE id FROM ONLY type::table($user_table) WHERE user_id = $user_id LIMIT 1);

            -- Create comment reply
            LET $comment_reply = CREATE comment CONTENT $blog_comment_input;
            LET $comment_reply_id = (SELECT VALUE id FROM $comment_reply);

            -- Relate the comment reply to the parent comment and the user
            RELATE $parent_comment->has_reply->$comment_reply_id;
            RELATE $user->wrote->$comment_reply_id;

            RETURN $comment_reply;
            COMMIT TRANSACTION;
            "
        )
        .bind(("blog_comment_input", blog_comment))
        .bind(("comment_table", "comment"))
        .bind(("comment_id", format!("comment:{}", comment_id)))
        .bind(("user_id", auth_res_from_acl.sub))
        .bind(("user_table", "user_id"))
        .await
        .map_err(|e| {
            tracing::debug!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                Some(StatusCode::BAD_REQUEST.as_u16()),
            )
            .build()
        })?;

        let response: Option<blog::BlogComment> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(blog_comment) => Ok(blog_comment),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }
    }

    /// React to a blog post
    pub async fn react_to_blog_post(
        &self,
        ctx: &Context<'_>,
        reaction: shared::Reaction,
        blog_post_id: String,
    ) -> async_graphql::Result<shared::Reaction> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
        .query(
            "
            BEGIN TRANSACTION;
            -- Get the user and blog post
            LET $user = (SELECT VALUE id FROM ONLY type::table($user_table) WHERE user_id = $user_id LIMIT 1);
            LET $blog_post = (SELECT VALUE id FROM ONLY type::table($blog_table) WHERE id = type::thing($blog_post_id) LIMIT 1);


            -- Relate the reaction to the user and blog_post
            LET $reaction = (RELATE $user->reaction->$blog_post CONTENT $reaction_input RETURN AFTER)[0];

            RETURN $reaction;
            COMMIT TRANSACTION;
            "
        )
        .bind(("reaction_input", reaction))
        .bind(("user_id", auth_res_from_acl.sub))
        .bind(("user_table", "user_id"))
        .bind(("blog_table", "blog_post"))
        .bind(("blog_post_id", format!("blog_post:{}", blog_post_id)))
        .await
        .map_err(|e| {
            tracing::debug!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                Some(StatusCode::BAD_REQUEST.as_u16()),
            )
            .build()
        })?;

        let response: Option<shared::Reaction> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(resume_item) => Ok(resume_item),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }
    }

    /// React to a blog comment
    pub async fn react_to_blog_comment(
        &self,
        ctx: &Context<'_>,
        reaction: shared::Reaction,
        comment_id: String,
    ) -> async_graphql::Result<shared::Reaction> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let auth_res_from_acl = check_auth_from_acl(headers).await?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: auth_res_from_acl.sub.clone(),
        };

        let id_added =
            add_foreign_key_if_not_exists::<Extension<Arc<Surreal<SurrealClient>>>, User>(
                db, user_fk,
            )
            .await;

        if id_added.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build());
        }

        let mut database_transaction = db
        .query(
            "
            BEGIN TRANSACTION;
            -- Get the user, comment
            LET $user = (SELECT VALUE id FROM ONLY type::table($user_table) WHERE user_id = $user_id LIMIT 1);
            LET $comment = (SELECT VALUE id FROM ONLY type::table($comment_table) WHERE id = type::thing($comment_id) LIMIT 1);

            -- Relate the reaction to the user
            LET $reaction = (RELATE $user->reaction->$comment CONTENT $reaction_input RETURN AFTER)[0];

            RETURN $reaction;
            COMMIT TRANSACTION;
            "
        )
        .bind(("reaction_input", reaction))
        .bind(("user_id", auth_res_from_acl.sub))
        .bind(("user_table", "user_id"))
        .bind(("comment_table", "comment"))
        .bind(("comment_id", format!("comment:{}", comment_id)))
        .await
        .map_err(|e| {
            tracing::debug!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                Some(StatusCode::BAD_REQUEST.as_u16()),
            )
            .build()
        })?;

        let response: Option<shared::Reaction> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(reaction) => Ok(reaction),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }
    }

    /// Send a message
    pub async fn send_message(
        &self,
        ctx: &Context<'_>,
        message: shared::Message,
    ) -> async_graphql::Result<shared::Message> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;

        let message: Option<shared::Message> =
            db.create("message").content(message).await.map_err(|e| {
                tracing::error!("Deserialization Error: {:?}", e);

                ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
            })?;

        match message {
            Some(message) => Ok(message),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }
    }

    /// Add a skill to a portfolio item
    pub async fn add_skill_to_portfolio_item(
        &self,
        ctx: &Context<'_>,
        skill_id: String,
        portfolio_item_id: String,
    ) -> async_graphql::Result<user::UserSkill> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;
        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let _auth_res_from_acl = check_auth_from_acl(headers).await?;

        let mut database_transaction = db
            .query(
                "
            BEGIN TRANSACTION;
            -- Get the skill and portfolio item
            LET $skill = type::thing($skill_id);
            LET $portfolio_item = type::thing($portfolio_item_id);

            -- Relate the skill to the portfolio item
            RELATE $portfolio_item->uses_skill->$skill;
            LET $skill_val = SELECT * FROM ONLY $skill LIMIT 1;

            RETURN $skill_val;
            COMMIT TRANSACTION;
            ",
            )
            .bind(("skill_id", format!("skill:{}", skill_id)))
            .bind(("skill_table", "skill"))
            .bind((
                "portfolio_item_id",
                format!("portfolio:{}", portfolio_item_id),
            ))
            .bind(("portfolio_table", "portfolio"))
            .await
            .map_err(|e| {
                tracing::debug!("DB Query Error: {}", e);

                ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
            })?;

        let response: Option<user::UserSkill> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);
            ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
        })?;

        match response {
            Some(user_skill) => Ok(user_skill),
            None => {
                Err(ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build())
            }
        }
    }

    pub async fn edit_blog_post(
        &self,
        ctx: &Context<'_>,
        blog_post: blog::BlogPostUpdate,
        blog_post_id: String,
    ) -> async_graphql::Result<blog::BlogPostUpdateResponse> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new(
                    "Server Error",
                    Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                )
                .build()
            })?;
        let headers = ctx.data::<HeaderMap>().map_err(|e| {
            tracing::error!("Error HeaderMap: {:?}", e);
            ExtendedError::new(
                "Server Error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .build()
        })?;

        let _auth_res_from_acl = check_auth_from_acl(headers).await?;

        let response: Option<blog::BlogPostUpdateResponse> = db
            .update(("blog_post", blog_post_id))
            .merge(blog_post)
            .await
            .map_err(|e| {
                tracing::error!("Deserialization Error: {:?}", e);

                ExtendedError::new("Failed", Some(StatusCode::BAD_REQUEST.as_u16())).build()
            })?;

        match response {
            Some(blog_post) => Ok(blog_post),
            None => Err(ExtendedError::new(
                "Blog post not found",
                Some(StatusCode::NOT_FOUND.as_u16()),
            )
            .build()),
        }
    }
}
