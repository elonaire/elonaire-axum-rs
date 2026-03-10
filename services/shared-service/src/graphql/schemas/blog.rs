use async_graphql::{ComplexObject, Enum, InputObject, SimpleObject};

use lib::utils::models::{UploadedFileId, UserId};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

use crate::graphql::schemas::shared::Reaction;

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct BlogPostInput {
    pub title: String,
    pub short_description: String,
    pub status: Option<BlogStatus>,
    pub thumbnail: String,
    pub content: String,
    #[graphql(skip)]
    pub content_file: Option<RecordId>,
    #[graphql(skip)]
    pub content_text_only: Option<String>,
    pub category: BlogCategory,
    pub is_featured: Option<bool>,
    pub is_premium: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct BlogPost {
    #[graphql(skip)]
    pub id: RecordId,
    pub title: String,
    pub short_description: String,
    pub status: Option<BlogStatus>,
    pub thumbnail: String,
    #[graphql(skip)]
    pub content_file: UploadedFileId,
    pub content_text_only: Option<String>,
    #[graphql(skip)]
    pub author: UserId,
    pub content: Option<String>,
    pub category: BlogCategory,
    pub link: String,
    pub published_date: Option<String>,
    pub is_featured: Option<bool>,
    pub is_premium: Option<bool>,
    pub comments: Option<Vec<BlogComment>>,
    pub read_time: Option<u32>,
    pub reaction_count: Option<u32>,
    pub current_user_reaction: Option<Reaction>,
    pub bookmarks_count: Option<u32>,
    pub shares_count: Option<u32>,
    pub current_user_bookmarked: Option<bool>,
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
    pub content: Option<String>,
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
    #[graphql(name = "Science")]
    Science,
    #[graphql(name = "Health")]
    Health,
    #[graphql(name = "EmbeddedSystems")]
    EmbeddedSystems,
    #[graphql(name = "IoT")]
    IoT,
    #[graphql(name = "UpCloseAndCandid")]
    UpCloseAndCandid,
    #[graphql(name = "Commentary")]
    Commentary,
    #[graphql(name = "CyberSecurity")]
    CyberSecurity,
    #[graphql(name = "Programming")]
    Programming,
}

// BlogComment
#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct BlogCommentInput {
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct BlogComment {
    #[graphql(skip)]
    pub id: RecordId,
    pub content: String,
    pub reply_count: u32,
    #[graphql(skip)]
    pub author: UserId,
    pub created_at: String,
    pub updated_at: String,
    pub current_user_reaction: Option<Reaction>,
    pub reaction_count: Option<u32>,
}

#[ComplexObject]
impl BlogPost {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }

    async fn content_file(&self) -> String {
        self.content_file.file_id.to_owned()
    }

    async fn author(&self) -> String {
        self.author.user_id.to_owned()
    }
}

#[ComplexObject]
impl BlogComment {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }

    async fn author(&self) -> String {
        self.author.user_id.to_owned()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct FetchBlogPostsQueryFilters {
    pub status: Option<BlogStatus>,
    pub is_featured: Option<bool>,
    // pub is_premium: Option<bool>,
    pub sort_configs: Option<SortConfigs>,
    pub search_term: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq)]
pub enum BlogPostsFilterSortBy {
    DateOfCreation,
    Reads,
}

#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq)]
pub enum SortOrder {
    #[graphql(name = "Asc")]
    Asc,
    #[graphql(name = "Desc")]
    Desc,
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct SortConfigs {
    pub sort_by: BlogPostsFilterSortBy,
    pub sort_order: SortOrder,
}
