use std::sync::Arc;

use async_graphql::{Context, Error, Object};
use axum::Extension;
// use gql_client::Client as GQLClient;

use hyper::{HeaderMap, StatusCode};
use lib::utils::api_response::synthesize_graphql_response;
use lib::utils::grpc::{confirm_authorization, create_file_from_content};
use lib::utils::models::{
    AdminPrivilege, AllowedCreateFileExtension, AuthorizationConstraint, CreateFileInfo,
    CurrencyId, ForeignKey, UploadedFileId, UserId,
};
use lib::{
    integration::foreign_key::add_foreign_key_if_not_exists,
    middleware::auth::graphql::confirm_authentication, utils::custom_error::ExtendedError,
};
use surrealdb::RecordId;
use surrealdb::{engine::remote::ws::Client as SurrealClient, Surreal};

use crate::graphql::schemas::shared::{
    GraphQLApiResponse, Ratecard, RatecardInput, RatecardInputMetadata, ServiceRate,
    ServiceRateInput, ServiceRateInputMetadata, ServiceRequest, ServiceRequestInput,
    ServiceRequestInputMetadata,
};
use crate::graphql::schemas::user::{UserProfessionalInfo, UserProfessionalInfoInput};
use crate::graphql::schemas::{blog, shared, user};

// const CHUNK_SIZE: u64 = 1024 * 1024 * 5; // 5MB

pub struct Mutation;

#[Object]
impl Mutation {
    /// Create new professional details
    async fn create_professional_details(
        &self,
        ctx: &Context<'_>,
        professional_details: UserProfessionalInfoInput,
    ) -> async_graphql::Result<GraphQLApiResponse<UserProfessionalInfo>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error extracting Surreal Client: {:?}", e);
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
            permissions: vec!["write:professional_details".into()],
            privilege: AdminPrivilege::SuperAdmin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
            (CREATE professional_details CONTENT $professional_details_input RETURN AFTER)
            ",
            )
            .bind(("professional_details_input", professional_details))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Failed: {}", e);
                // Error::new("Internal Server Error.")
                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        let response: Option<UserProfessionalInfo> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Failed: {}", e);
            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(professional_info) => {
                let api_response =
                    synthesize_graphql_response(ctx, &professional_info, Some(authenticated_ref))
                        .ok_or_else(|| {
                        tracing::error!("Failed to synthesize response!");
                        ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
                    })?;
                Ok(api_response.into())
            }
            None => Err(ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()),
        }
    }

    /// Create a new user service
    pub async fn create_user_service(
        &self,
        ctx: &Context<'_>,
        user_service: user::UserServiceInput,
    ) -> async_graphql::Result<GraphQLApiResponse<user::UserService>> {
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
            permissions: vec!["write:service".into()],
            privilege: AdminPrivilege::SuperAdmin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_user_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_user_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(
                ExtendedError::new("Invalid Input Data", StatusCode::BAD_REQUEST.as_str()).build(),
            );
        }

        let mut database_transaction = db
            .query(
                "
                (CREATE service CONTENT $user_service_input RETURN AFTER)
            ",
            )
            .bind(("user_service_input", user_service))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        let response: Option<user::UserService> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(user_service) => {
                let api_response =
                    synthesize_graphql_response(ctx, &user_service, Some(authenticated_ref))
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

    /// Create a new user project/portfolio item
    pub async fn create_portfolio_item(
        &self,
        ctx: &Context<'_>,
        portfolio_item: user::UserPortfolioInput,
        skills: Vec<String>,
    ) -> async_graphql::Result<GraphQLApiResponse<user::UserPortfolio>> {
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
            permissions: vec!["write:portfolio".into()],
            privilege: AdminPrivilege::SuperAdmin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(
                ExtendedError::new("Invalid Input Data", StatusCode::BAD_REQUEST.as_str()).build(),
            );
        }

        let mut database_transaction = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $portfolio_item = (SELECT VALUE id FROM ONLY (CREATE portfolio CONTENT $portfolio_item_input RETURN AFTER) LIMIT 1);

                FOR $skill IN $skills {
                    LET $skill = type::thing('skill', $skill);

                    RELATE $portfolio_item -> uses_skill -> $skill;
                };
                LET $portfolio_item = (SELECT *, ->uses_skill->skill[*] AS skills FROM ONLY $portfolio_item LIMIT 1);
                RETURN $portfolio_item;
                COMMIT TRANSACTION;
            ",
            )
            .bind(("portfolio_item_input", portfolio_item))
            .bind(("skills", skills))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new(
                    "Failed",
                    StatusCode::BAD_REQUEST.as_str(),
                )
                .build()
            })?;

        let response: Option<user::UserPortfolio> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(user_portfolio) => {
                let api_response =
                    synthesize_graphql_response(ctx, &user_portfolio, Some(authenticated_ref))
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

    /// Create a new user resume item
    pub async fn create_resume_item(
        &self,
        ctx: &Context<'_>,
        resume_item: user::UserResumeInput,
        achievements: Vec<String>,
    ) -> async_graphql::Result<GraphQLApiResponse<user::UserResume>> {
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
            permissions: vec!["write:resume_item".into()],
            privilege: AdminPrivilege::SuperAdmin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
            BEGIN TRANSACTION;
            LET $resume = (SELECT VALUE id FROM (CREATE resume CONTENT $resume_item_input RETURN AFTER));

            FOR $achievement IN $achievements {
                RELATE $resume->achievement->$resume CONTENT {
                    description: $achievement,
                };
            };
            LET $resume = SELECT *, ->achievement.* AS achievements FROM ONLY $resume LIMIT 1;
            RETURN $resume;
            COMMIT TRANSACTION;
            ",
            )
            .bind(("resume_item_input", resume_item))
            .bind(("achievements", achievements))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new(
                    "Failed",
                    StatusCode::BAD_REQUEST.as_str(),
                )
                .build()
            })?;

        let response: Option<user::UserResume> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(resume_item) => {
                let api_response =
                    synthesize_graphql_response(ctx, &resume_item, Some(authenticated_ref))
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

    /// Create a new user skill
    pub async fn create_skill(
        &self,
        ctx: &Context<'_>,
        skill: user::UserSkillInput,
    ) -> async_graphql::Result<GraphQLApiResponse<user::UserSkill>> {
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
            permissions: vec!["write:skill".into()],
            privilege: AdminPrivilege::SuperAdmin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
                (CREATE skill CONTENT $skill_input RETURN AFTER)
            ",
            )
            .bind(("skill_input", skill))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        let response: Option<user::UserSkill> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(user_skill) => {
                let api_response =
                    synthesize_graphql_response(ctx, &user_skill, Some(authenticated_ref))
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

    /// Create a new blog post
    pub async fn create_blog_post(
        &self,
        ctx: &Context<'_>,
        mut blog_post: blog::BlogPostInput,
    ) -> async_graphql::Result<GraphQLApiResponse<blog::BlogPost>> {
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
            permissions: vec!["write:blog_post".into()],
            privilege: AdminPrivilege::Admin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let file_info = CreateFileInfo {
            file_name: blog_post.title.clone(),
            content: blog_post.content.clone(),
            extension: AllowedCreateFileExtension::Markdown,
            is_free: !blog_post.is_premium.unwrap_or(false),
        };

        let saved_file = create_file_from_content(authenticated_ref, headers, &file_info)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create file: {}", e);
                ExtendedError::new(
                    "Something went wrong",
                    StatusCode::INTERNAL_SERVER_ERROR.as_str(),
                )
                .build()
            })?;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let content_file_fk = ForeignKey {
            table: "file_id".to_string(),
            column: "file_id".to_string(),
            foreign_key: saved_file,
        };

        let added_user = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await
        .ok_or_else(|| {
            tracing::error!("Failed to add user_id");
            ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build()
        })?;

        let added_file = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UploadedFileId,
        >(db, content_file_fk)
        .await
        .ok_or_else(|| {
            tracing::error!("Failed to add content_file");
            ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build()
        })?;

        blog_post.content_file = Some(RecordId::from(added_file.id));

        let mut database_transaction = db
            .query(
                "
            BEGIN TRANSACTION;
            LET $user = (SELECT VALUE id FROM type::table($table) WHERE user_id = $user_id LIMIT 1);

            LET $blog_post = (CREATE blog_post CONTENT $blog_post_input RETURN AFTER)[0];
            LET $blog_post_id = (SELECT VALUE id FROM ONLY $blog_post);
            RELATE $user->wrote->$blog_post_id;

            LET $full_blog_post = (SELECT *, (<-wrote<-user_id)[0].* AS author, (SELECT *, (<-wrote<-user_id)[0][*] AS author, array::len(->has_reply) AS reply_count FROM ->has_comment->comment) AS comments FROM ONLY $blog_post_id FETCH content_file);
            RETURN $full_blog_post;
            COMMIT TRANSACTION;
            ",
            )
            .bind(("blog_post_input", blog_post))
            .bind(("table", "user_id"))
            .bind(("user_id", added_user.user_id))
            // .bind(("file_id", added_file.id))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);
                Error::new("Internal Server Error")
            })?;

        let response: Option<blog::BlogPost> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(blog_post) => {
                let api_response =
                    synthesize_graphql_response(ctx, &blog_post, Some(authenticated_ref))
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

    /// Add a comment to a blog post
    pub async fn add_comment_to_blog_post(
        &self,
        ctx: &Context<'_>,
        blog_comment: blog::BlogCommentInput,
        blog_post_id: String,
    ) -> async_graphql::Result<GraphQLApiResponse<blog::BlogComment>> {
        // TODO: Might have to allow anonymous comments?
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let authenticated = confirm_authentication(ctx).await?;
        let authenticated_ref = &authenticated;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
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
            RETURN (SELECT *, (<-wrote<-user_id)[0][*] AS author, array::len(->has_reply) AS reply_count, array::len(<-reaction) AS reaction_count, (<-reaction<-(user_id WHERE user_id = $user_id))[0][*] AS current_user_reaction FROM $blog_comment);
            COMMIT TRANSACTION;
            "
        )
        .bind(("blog_comment_input", blog_comment))
        .bind(("blog_table", "blog_post"))
        .bind(("blog_post_id", format!("blog_post:{}", blog_post_id)))
        .bind(("user_id", authenticated_ref.sub.to_owned()))
        .bind(("user_table", "user_id"))
        .await
        .map_err(|e| {
            tracing::error!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                StatusCode::BAD_REQUEST.as_str(),
            )
            .build()
        })?;

        let response: Option<blog::BlogComment> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(blog_comment) => {
                let api_response =
                    synthesize_graphql_response(ctx, &blog_comment, Some(authenticated_ref))
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

    /// Reply to a comment
    pub async fn reply_to_a_comment(
        &self,
        ctx: &Context<'_>,
        blog_comment: blog::BlogCommentInput,
        comment_id: String,
    ) -> async_graphql::Result<GraphQLApiResponse<blog::BlogComment>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let authenticated = confirm_authentication(ctx).await?;
        let authenticated_ref = &authenticated;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
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
        .bind(("user_id", authenticated_ref.sub.to_owned()))
        .bind(("user_table", "user_id"))
        .await
        .map_err(|e| {
            tracing::error!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                StatusCode::BAD_REQUEST.as_str(),
            )
            .build()
        })?;

        let response: Option<blog::BlogComment> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(blog_comment) => {
                let api_response =
                    synthesize_graphql_response(ctx, &blog_comment, Some(authenticated_ref))
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

    /// React to a blog post
    pub async fn react_to_blog_post(
        &self,
        ctx: &Context<'_>,
        reaction: shared::ReactionInput,
        blog_post_id: String,
    ) -> async_graphql::Result<GraphQLApiResponse<shared::Reaction>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let authenticated = confirm_authentication(ctx).await?;
        let authenticated_ref = &authenticated;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let mut database_transaction = db
        .query(
            "
            BEGIN TRANSACTION;
            LET $user = (SELECT VALUE id FROM ONLY type::table($user_table) WHERE user_id = $user_id LIMIT 1);
            LET $blog_post = type::thing('blog_post', $blog_post_id);
            IF !$blog_post.exists() {
                THROW 'Invalid Input';
            };
            LET $existing_reaction = (SELECT VALUE id FROM ONLY reaction WHERE ->(blog_post WHERE id = $blog_post) AND <-(user_id WHERE id = $user) LIMIT 1);
            LET $reaction = IF $existing_reaction != NONE
           	{ (UPDATE $existing_reaction MERGE $reaction_input)[0] }
                        ELSE
           	{ (RELATE $user -> reaction -> $blog_post CONTENT $reaction_input RETURN AFTER)[0] }
            ;
            RETURN $reaction;
            COMMIT TRANSACTION;
            "
        )
        .bind(("reaction_input", reaction))
        .bind(("user_id", authenticated_ref.sub.to_owned()))
        .bind(("user_table", "user_id"))
        .bind(("blog_table", "blog_post"))
        .bind(("blog_post_id", blog_post_id))
        .await
        .map_err(|e| {
            tracing::error!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                StatusCode::BAD_REQUEST.as_str(),
            )
            .build()
        })?;

        let response: Option<shared::Reaction> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(reaction) => {
                let api_response =
                    synthesize_graphql_response(ctx, &reaction, Some(authenticated_ref))
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

    /// React to a blog comment
    pub async fn react_to_blog_comment(
        &self,
        ctx: &Context<'_>,
        reaction: shared::ReactionInput,
        comment_id: String,
    ) -> async_graphql::Result<GraphQLApiResponse<shared::Reaction>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let authenticated = confirm_authentication(ctx).await?;
        let authenticated_ref = &authenticated;

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let mut database_transaction = db
        .query(
            "
            BEGIN TRANSACTION;
            LET $user = (SELECT VALUE id FROM ONLY type::table($user_table) WHERE user_id = $user_id LIMIT 1);
            LET $comment = (SELECT VALUE id FROM ONLY type::table($comment_table) WHERE id = type::thing('comment', $comment_id) LIMIT 1);
            LET $existing_reaction = (SELECT VALUE id FROM ONLY reaction WHERE ->(comment WHERE id = $comment) AND <-(user_id WHERE id = $user) LIMIT 1);
            LET $reaction = IF $existing_reaction != NONE
           	{ (UPDATE $existing_reaction MERGE $reaction_input)[0] }
                        ELSE
           	{ (RELATE $user -> reaction -> $comment CONTENT $reaction_input RETURN AFTER)[0] }
            ;

            RETURN $reaction;
            COMMIT TRANSACTION;
            "
        )
        .bind(("reaction_input", reaction))
        .bind(("user_id", authenticated_ref.sub.to_owned()))
        .bind(("user_table", "user_id"))
        .bind(("comment_table", "comment"))
        .bind(("comment_id", comment_id))
        .await
        .map_err(|e| {
            tracing::error!("DB Query Error: {}", e);

            ExtendedError::new(
                "Failed",
                StatusCode::BAD_REQUEST.as_str(),
            )
            .build()
        })?;

        let response: Option<shared::Reaction> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(reaction) => {
                let api_response =
                    synthesize_graphql_response(ctx, &reaction, Some(authenticated_ref))
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

    /// Send a message
    pub async fn send_message(
        &self,
        ctx: &Context<'_>,
        message: shared::MessageInput,
    ) -> async_graphql::Result<GraphQLApiResponse<shared::Message>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let message: Option<shared::Message> =
            db.create("message").content(message).await.map_err(|e| {
                tracing::error!("Deserialization Error: {:?}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        match message {
            Some(message) => {
                let api_response =
                    synthesize_graphql_response(ctx, &message, None).ok_or_else(|| {
                        tracing::error!("Failed to synthesize response!");
                        ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
                    })?;
                Ok(api_response.into())
            }
            None => Err(ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()),
        }
    }

    pub async fn edit_blog_post(
        &self,
        ctx: &Context<'_>,
        blog_post: blog::BlogPostUpdate,
        blog_post_id: String,
    ) -> async_graphql::Result<GraphQLApiResponse<blog::BlogPost>> {
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
            permissions: vec!["write:blog_post".into()],
            privilege: AdminPrivilege::Admin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let response: Option<blog::BlogPost> = db
            .update(("blog_post", blog_post_id))
            .merge(blog_post)
            .await
            .map_err(|e| {
                tracing::error!("Deserialization Error: {:?}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        match response {
            Some(blog_post) => {
                let api_response =
                    synthesize_graphql_response(ctx, &blog_post, Some(authenticated_ref))
                        .ok_or_else(|| {
                            tracing::error!("Failed to synthesize response!");
                            ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str())
                                .build()
                        })?;
                Ok(api_response.into())
            }
            None => Err(
                ExtendedError::new("Blog post not found", StatusCode::NOT_FOUND.as_str()).build(),
            ),
        }
    }

    /// Create a new ratecard
    pub async fn create_ratecard(
        &self,
        ctx: &Context<'_>,
        ratecard_input: RatecardInput,
        ratecard_input_metadata: RatecardInputMetadata,
    ) -> async_graphql::Result<GraphQLApiResponse<Ratecard>> {
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
            permissions: vec!["write:ratecard".into()],
            privilege: AdminPrivilege::SuperAdmin,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $created_ratecard = CREATE ratecard CONTENT $ratecard_input;
                LET $created_ratecard_id = (SELECT VALUE id FROM $created_ratecard);

                FOR $service IN $ratecard_input_metadata.service_ids {
                    LET $service_record = type::thing('service', $service);
                   	RELATE $created_ratecard_id -> contains -> $service_record;
                };

                LET $full_ratecard = (SELECT *, ->contains->service.* AS services FROM ONLY $created_ratecard_id LIMIT 1);
                RETURN $full_ratecard;
                COMMIT TRANSACTION;
            ",
            )
            .bind(("ratecard_input", ratecard_input))
            .bind(("ratecard_input_metadata", ratecard_input_metadata))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        let response: Option<Ratecard> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(ratecard) => {
                let api_response =
                    synthesize_graphql_response(ctx, &ratecard, Some(authenticated_ref))
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

    /// Create a new service_request
    pub async fn create_service_request(
        &self,
        ctx: &Context<'_>,
        mut service_request_input: ServiceRequestInput,
        service_request_input_metadata: ServiceRequestInputMetadata,
    ) -> async_graphql::Result<GraphQLApiResponse<ServiceRequest>> {
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
            permissions: vec!["write:service_request".into()],
            privilege: AdminPrivilege::None,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let user_fk = ForeignKey {
            table: "user_id".to_string(),
            column: "user_id".to_string(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        for file_id in &service_request_input_metadata.supporting_docs_file_ids {
            let file_fk = ForeignKey {
                table: "file_id".to_string(),
                column: "file_id".to_string(),
                foreign_key: file_id.to_owned(),
            };

            let added_file = add_foreign_key_if_not_exists::<
                Extension<Arc<Surreal<SurrealClient>>>,
                UploadedFileId,
            >(db, file_fk)
            .await;

            if added_file.is_none() {
                tracing::error!("Failed to add file_id");
                return Err(ExtendedError::new(
                    "Something went wrong",
                    StatusCode::INTERNAL_SERVER_ERROR.as_str(),
                )
                .build());
            }

            service_request_input
                .supporting_docs
                .push(added_file.unwrap().id);
        }

        let mut database_transaction = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $created_service_request = CREATE service_request CONTENT $service_request_input;
                LET $created_service_request_id = (SELECT VALUE id FROM ONLY $created_service_request LIMIT 1);

                FOR $service IN $service_request_input_metadata.service_ids {
                    LET $service_record = type::thing('service', $service);
                   	RELATE $created_service_request_id -> contains -> $service_record;
                };
                RETURN (SELECT * FROM ONLY $created_service_request_id FETCH supporting_docs);
                COMMIT TRANSACTION;
            ",
            )
            .bind(("service_request_input", service_request_input))
            .bind((
                "service_request_input_metadata",
                service_request_input_metadata,
            ))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        let response: Option<ServiceRequest> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(service_request) => {
                let api_response =
                    synthesize_graphql_response(ctx, &service_request, Some(authenticated_ref))
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

    /// Create a new service rate
    pub async fn create_service_rate(
        &self,
        ctx: &Context<'_>,
        mut service_rate_input: ServiceRateInput,
        service_rate_input_metadata: ServiceRateInputMetadata,
    ) -> async_graphql::Result<GraphQLApiResponse<ServiceRate>> {
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
            permissions: vec!["write:service_rate".into()],
            privilege: AdminPrivilege::None,
        };
        let authorized =
            confirm_authorization(authenticated_ref, &authorization_constraint, headers).await?;

        if !authorized {
            return Err(ExtendedError::new("Forbidden", StatusCode::FORBIDDEN.as_str()).build());
        }

        let user_fk = ForeignKey {
            table: "user_id".into(),
            column: "user_id".into(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let currency_fk = ForeignKey {
            table: "currency_id".into(),
            column: "currency_id".into(),
            foreign_key: service_rate_input_metadata.currency_id.clone(),
        };

        let currency_added = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            CurrencyId,
        >(db, currency_fk)
        .await;

        if currency_added.is_none() {
            tracing::error!("Failed to add currency_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        service_rate_input.service = Some(RecordId::from_table_key(
            "service",
            &service_rate_input_metadata.service_id,
        ));
        service_rate_input.currency_id = Some(currency_added.unwrap().id);

        let mut database_transaction = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $created_service_rate = CREATE rate CONTENT $service_rate_input;

                LET $rate_id = SELECT VALUE id FROM ONLY $created_service_rate LIMIT 1;
                LET $expanded_rate = SELECT * FROM ONLY $rate_id FETCH service, currency_id;

                RETURN $expanded_rate;
                COMMIT TRANSACTION;
            ",
            )
            .bind(("service_rate_input", service_rate_input))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        let response: Option<ServiceRate> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(service_rate) => {
                let api_response =
                    synthesize_graphql_response(ctx, &service_rate, Some(authenticated_ref))
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

    /// Bookmark blog post
    pub async fn bookmark_blog_post(
        &self,
        ctx: &Context<'_>,
        blog_post_id: String,
    ) -> async_graphql::Result<GraphQLApiResponse<bool>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        let authenticated = confirm_authentication(ctx).await?;
        let authenticated_ref = &authenticated;

        let user_fk = ForeignKey {
            table: "user_id".into(),
            column: "user_id".into(),
            foreign_key: authenticated_ref.sub.to_owned(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $user = type::thing('user_id', $user_id);
                LET $blog_post = type::thing('blog_post', $blog_post_id);

                LET $existing_bookmark = (SELECT VALUE id FROM ONLY bookmark WHERE <-(user_id WHERE id = $user) AND ->(blog_post WHERE id = $blog_post) LIMIT 1);

                LET $bookmarked = IF $existing_bookmark == NONE {
                    RELATE $user -> bookmark -> $blog_post;
                    true
                } ELSE {
                    DELETE $existing_bookmark;
                    false
                };
                RETURN $bookmarked;
                COMMIT TRANSACTION;
            ",
            )
            .bind(("user_id", added_id.unwrap().id.key().to_string()))
            .bind(("blog_post_id", blog_post_id))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        let response: Option<bool> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(bookmarked) => {
                let api_response =
                    synthesize_graphql_response(ctx, &bookmarked, Some(authenticated_ref))
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

    /// Update blog post share count
    pub async fn update_blog_post_share_count(
        &self,
        ctx: &Context<'_>,
        blog_post_id: String,
    ) -> async_graphql::Result<GraphQLApiResponse<u32>> {
        let db = ctx
            .data::<Extension<Arc<Surreal<SurrealClient>>>>()
            .map_err(|e| {
                tracing::error!("Error Surreal Client: {:?}", e);
                ExtendedError::new("Server Error", StatusCode::INTERNAL_SERVER_ERROR.as_str())
                    .build()
            })?;

        // if the user is authenticated, get the user id
        let user_id = match confirm_authentication(ctx).await {
            Ok(auth_status) => auth_status.sub,
            Err(_e) => "anonymous".into(),
        };

        let user_fk = ForeignKey {
            table: "user_id".into(),
            column: "user_id".into(),
            foreign_key: user_id.clone(),
        };

        let added_id = add_foreign_key_if_not_exists::<
            Extension<Arc<Surreal<SurrealClient>>>,
            UserId,
        >(db, user_fk)
        .await;

        if added_id.is_none() {
            tracing::error!("Failed to add user_id");
            return Err(ExtendedError::new(
                "Something went wrong",
                StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            )
            .build());
        }

        let mut database_transaction = db
            .query(
                "
                BEGIN TRANSACTION;
                LET $user = type::thing('user_id', $user_id);
                LET $blog_post = type::thing('blog_post', $blog_post_id);
                RELATE $user -> share -> $blog_post;
                LET $total = (SELECT count() AS total FROM share WHERE ->(blog_post WHERE id = $blog_post) GROUP ALL)[0]['total'];
                RETURN $total;
                COMMIT TRANSACTION;
            ",
            )
            .bind(("user_id", added_id.unwrap().id.key().to_string()))
            .bind(("blog_post_id", blog_post_id))
            .await
            .map_err(|e| {
                tracing::error!("DB Query Error: {}", e);

                ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
            })?;

        let response: Option<u32> = database_transaction.take(0).map_err(|e| {
            tracing::error!("Deserialization Error: {:?}", e);

            ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()
        })?;

        match response {
            Some(updated_share_count) => {
                let api_response = synthesize_graphql_response(ctx, &updated_share_count, None)
                    .ok_or_else(|| {
                        tracing::error!("Failed to synthesize response!");
                        ExtendedError::new("Bad Request", StatusCode::BAD_REQUEST.as_str()).build()
                    })?;
                Ok(api_response.into())
            }
            None => Err(ExtendedError::new("Failed", StatusCode::BAD_REQUEST.as_str()).build()),
        }
    }
}
