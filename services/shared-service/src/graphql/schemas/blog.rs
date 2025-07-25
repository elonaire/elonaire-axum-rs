use std::env;

use async_graphql::{ComplexObject, Enum, Error, InputObject, SimpleObject};
use hyper::{
    header::{AUTHORIZATION, COOKIE},
    HeaderMap, StatusCode,
};
use lib::{
    integration::grpc::clients::{
        acl_service::{acl_client::AclClient, Empty},
        files_service::{files_service_client::FilesServiceClient, FileId},
    },
    utils::{
        custom_error::ExtendedError,
        grpc::{create_grpc_client, AuthMetaData},
    },
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};
use tonic::transport::Channel;

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
#[graphql(input_name = "BlogPostInput")]
pub struct BlogPostInput {
    #[graphql(skip)]
    pub id: Option<Thing>,
    pub title: String,
    pub short_description: String,
    pub status: Option<BlogStatus>,
    pub thumbnail: String,
    pub content_file: String,
    pub other_images: Vec<String>,
    pub category: BlogCategory,
    #[graphql(skip)]
    pub link: String,
    pub published_date: Option<String>,
    pub is_featured: Option<bool>,
    pub is_premium: Option<bool>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct BlogPost {
    #[graphql(skip)]
    pub id: Option<Thing>,
    pub title: String,
    pub short_description: String,
    pub status: Option<BlogStatus>,
    pub thumbnail: String,
    pub content_file: String,
    pub other_images: Vec<String>,
    pub category: BlogCategory,
    pub link: String,
    pub published_date: Option<String>,
    pub is_featured: Option<bool>,
    pub is_premium: Option<bool>,
    pub comments: Vec<BlogComment>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq)]
pub enum BlogStatus {
    #[graphql(name = "Draft")]
    Draft,
    #[graphql(name = "Published")]
    Published,
    #[graphql(name = "Archived")]
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, InputObject)]
#[graphql(input_name = "BlogPostUpdateInput")]
pub struct BlogPostUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlogStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<BlogCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_featured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_premium: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct BlogPostUpdateResponse {
    #[graphql(skip)]
    pub id: Option<Thing>,
    pub title: String,
    pub short_description: String,
    pub status: Option<BlogStatus>,
    pub thumbnail: String,
    pub content_file: String,
    pub other_images: Vec<String>,
    pub category: BlogCategory,
    pub link: String,
    pub published_date: Option<String>,
    pub is_featured: Option<bool>,
    pub is_premium: Option<bool>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[ComplexObject]
impl BlogPostUpdateResponse {
    async fn id(&self) -> String {
        self.id.as_ref().map(|t| &t.id).expect("id").to_raw()
    }
}

// enum for BlogCategory: "WebDevelopment", "MobileDevelopment", "AI", "Technology", "Lifestyle"
#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq)]
pub enum BlogCategory {
    #[graphql(name = "WebDevelopment")]
    WebDevelopment,
    #[graphql(name = "MobileDevelopment")]
    MobileDevelopment,
    #[graphql(name = "ArtificialIntelligence")]
    ArtificialIntelligence,
    #[graphql(name = "Technology")]
    Technology,
    #[graphql(name = "Lifestyle")]
    Lifestyle,
}

impl BlogCategory {
    pub fn to_string(&self) -> String {
        match self {
            BlogCategory::WebDevelopment => "WebDevelopment".to_string(),
            BlogCategory::MobileDevelopment => "MobileDevelopment".to_string(),
            BlogCategory::ArtificialIntelligence => "ArtificialIntelligence".to_string(),
            BlogCategory::Technology => "Technology".to_string(),
            BlogCategory::Lifestyle => "Lifestyle".to_string(),
        }
    }
}

// BlogComment
#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, InputObject)]
#[graphql(input_name = "BlogCommentInput")]
#[graphql(complex)]
pub struct BlogComment {
    #[graphql(skip)]
    pub id: Option<Thing>,
    pub content: String,
    #[graphql(skip)]
    pub created_at: Datetime,
}

#[ComplexObject]
impl BlogPost {
    async fn id(&self) -> String {
        self.id.as_ref().map(|t| &t.id).expect("id").to_raw()
    }

    // generate Blog static content from the corresponding markdown file in the content_file field
    async fn content(&self) -> Option<String> {
        // Internal sign in logic using gRPC
        let request = tonic::Request::new(Empty {});

        let acl_service_grpc = env::var("OAUTH_SERVICE_GRPC");

        if let Err(e) = &acl_service_grpc {
            tracing::error!(
                "Missing the OAUTH_SERVICE_GRPC environment variable.: {}",
                e
            );

            return None;
        }

        let acl_service_grpc = acl_service_grpc.unwrap();

        if let Ok(mut acl_grpc_client) =
            create_grpc_client::<Empty, AclClient<Channel>>(&acl_service_grpc, false, None)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to connect to ACL service: {}", e);
                    Error::new("Failed to connect to ACL service".to_string())
                })
        {
            if let Ok(auth_res) = acl_grpc_client.sign_in_as_service(request).await {
                let mut header_map = HeaderMap::new();
                let internal_jwt = auth_res.into_inner().token;
                header_map.insert(
                    AUTHORIZATION,
                    format!("Bearer {}", &internal_jwt).as_str().parse().ok()?,
                );
                header_map.insert(
                    COOKIE,
                    format!("oauth_client=;t={}", &internal_jwt)
                        .as_str()
                        .parse()
                        .ok()?,
                );

                let auth_header = header_map.get(AUTHORIZATION);
                let cookie_header = header_map.get(COOKIE);

                let mut request = tonic::Request::new(FileId {
                    file_id: self.content_file.clone(),
                });

                let auth_metadata: AuthMetaData<FileId> = AuthMetaData {
                    auth_header,
                    cookie_header,
                    constructed_grpc_request: Some(&mut request),
                };

                let files_service_grpc = env::var("FILES_SERVICE_GRPC");

                if let Err(e) = &files_service_grpc {
                    tracing::error!(
                        "Missing the FILES_SERVICE_GRPC environment variable.: {}",
                        e
                    );

                    return None;
                }

                let files_service_grpc = files_service_grpc.unwrap();

                if let Ok(mut files_grpc_client) = create_grpc_client::<
                    FileId,
                    FilesServiceClient<Channel>,
                >(
                    &files_service_grpc, true, Some(auth_metadata)
                )
                .await
                .map_err(|e| {
                    tracing::error!("Failed to connect to Files service: {}", e);
                    // Error::new("Failed to connect to Files service".to_string())
                    ExtendedError::new(
                        "Failed to connect to Files service",
                        Some(StatusCode::BAD_REQUEST.as_u16()),
                    )
                    .build()
                }) {
                    if let Ok(res) = files_grpc_client.get_file_name(request).await {
                        tracing::debug!("files_grpc_client res: {:?}", res);
                        let file_name: String = res.into_inner().file_name;

                        let base_url = std::env::var("FILES_SERVICE")
                            .map_err(|e| {
                                tracing::error!("FILES_SERVICE not set: {}", e);
                            })
                            .ok()?;
                        let url = format!("{}/view/{}", base_url, file_name);

                        // Create an HTTP client
                        let client = Client::new();

                        // Fetch the content from the URL
                        if let Ok(text) = client.get(&url).send().await {
                            if let Ok(content) = text.text().await {
                                let html_content = markdown::to_html(&content);
                                Some(html_content)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[ComplexObject]
impl BlogComment {
    async fn id(&self) -> String {
        self.id.as_ref().map(|t| &t.id).expect("id").to_raw()
    }
}
