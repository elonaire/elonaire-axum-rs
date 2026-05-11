use async_graphql::{ComplexObject, Enum, InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};

use super::blog::BlogPost;

#[derive(Clone, Debug, Serialize, Deserialize, InputObject, SurrealValue)]
pub struct UserProfessionalInfoInput {
    pub description: String,
    pub active: bool,
    pub occupation: String,
    pub start_date: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, SurrealValue)]
#[graphql(complex)]
pub struct UserProfessionalInfo {
    #[graphql(skip)]
    pub id: RecordId,
    pub description: String,
    pub active: bool,
    pub occupation: String,
    pub start_date: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject, SurrealValue)]
pub struct UserPortfolioInput {
    pub title: String,
    pub description: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub link: String,
    pub category: UserPortfolioCategory,
    pub thumbnail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, SurrealValue)]
#[graphql(complex)]
pub struct UserPortfolio {
    #[graphql(skip)]
    pub id: RecordId,
    pub title: String,
    pub description: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub link: String,
    pub category: UserPortfolioCategory,
    pub thumbnail: String,
    pub skills: Vec<UserSkill>,
}

// enum for UserPortfolio category
#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq, SurrealValue)]
pub enum UserPortfolioCategory {
    #[graphql(name = "JavaScript")]
    JavaScript,
    #[graphql(name = "Rust")]
    Rust,
    #[graphql(name = "Database")]
    Database,
    #[graphql(name = "DevOps")]
    DevOps,
    #[graphql(name = "Cloud")]
    Cloud,
    #[graphql(name = "Mobile")]
    Mobile,
}

#[ComplexObject]
impl UserPortfolio {
    async fn id(&self) -> Option<String> {
        match &self.id.key {
            RecordIdKey::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    async fn years_of_experience(&self) -> Option<u32> {
        let start_date = self.start_date.date_naive();

        let end_date = self
            .end_date
            .as_ref()
            .map(|date| date.date_naive())
            .unwrap_or_else(|| Utc::now().date_naive());

        end_date.years_since(start_date)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject, SurrealValue)]
pub struct UserResumeInput {
    pub title: String,
    pub more_info: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub link: Option<String>,
    pub section: UserResumeSection,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, SurrealValue)]
#[graphql(complex)]
pub struct UserResume {
    #[graphql(skip)]
    pub id: RecordId,
    pub title: String,
    pub more_info: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub link: Option<String>,
    pub section: UserResumeSection,
    pub achievements: Vec<ResumeAchievement>,
}

#[ComplexObject]
impl UserResume {
    async fn id(&self) -> Option<String> {
        match &self.id.key {
            RecordIdKey::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    async fn years_of_experience(&self) -> Option<u32> {
        // TODO: factor in months, currently only years e.g. 1 year 6 months
        let start_date = self.start_date.date_naive();

        let end_date = self
            .end_date
            .as_ref()
            .map(|date| date.date_naive())
            .unwrap_or_else(|| Utc::now().date_naive());

        end_date.years_since(start_date)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq, SurrealValue)]
pub enum UserResumeSection {
    #[graphql(name = "Education")]
    Education,
    #[graphql(name = "Experience")]
    Experience,
    #[graphql(name = "Achievements")]
    Achievements,
    #[graphql(name = "Projects")]
    Projects,
    #[graphql(name = "Certifications")]
    Certifications,
    #[graphql(name = "Volunteer")]
    Volunteer,
    #[graphql(name = "Publications")]
    Publications,
    #[graphql(name = "Languages")]
    Languages,
    #[graphql(name = "Interests")]
    Interests,
    #[graphql(name = "References")]
    References,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, SurrealValue)]
#[graphql(complex)]
pub struct ResumeAchievement {
    #[graphql(skip)]
    pub id: RecordId,
    pub description: String,
}

#[ComplexObject]
impl ResumeAchievement {
    async fn id(&self) -> Option<String> {
        match &self.id.key {
            RecordIdKey::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

// UserSkill
#[derive(Clone, Debug, Serialize, Deserialize, InputObject, SurrealValue)]
pub struct UserSkillInput {
    pub thumbnail: String,
    pub name: String,
    pub description: String,
    pub level: Option<UserSkillLevel>,
    pub r#type: UserSkillType,
    pub start_date: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, SurrealValue)]
#[graphql(complex)]
pub struct UserSkill {
    #[graphql(skip)]
    pub id: RecordId,
    pub thumbnail: String,
    pub name: String,
    pub description: String,
    pub level: Option<UserSkillLevel>,
    pub r#type: UserSkillType,
    pub start_date: DateTime<Utc>,
}

#[ComplexObject]
impl UserSkill {
    async fn id(&self) -> Option<String> {
        match &self.id.key {
            RecordIdKey::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    async fn years_of_experience(&self) -> Option<u32> {
        // TODO: factor in months, currently only years e.g. 1 year 6 months
        let start_date = self.start_date.date_naive();
        let today = Utc::now().date_naive();

        today.years_since(start_date)
    }
}

// UserSkillType enum
#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq, SurrealValue)]
pub enum UserSkillType {
    #[graphql(name = "Technical")]
    Technical,
    #[graphql(name = "Soft")]
    Soft,
}

// UserSkillLevel enum
#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq, SurrealValue)]
pub enum UserSkillLevel {
    #[graphql(name = "Beginner")]
    Beginner,
    #[graphql(name = "Intermediate")]
    Intermediate,
    #[graphql(name = "Advanced")]
    Advanced,
    #[graphql(name = "Expert")]
    Expert,
}

// UserService
#[derive(Clone, Debug, Serialize, Deserialize, InputObject, SurrealValue)]
pub struct UserServiceInput {
    pub title: String,
    pub description: String,
    pub thumbnail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, SurrealValue)]
#[graphql(complex)]
pub struct UserService {
    #[graphql(skip)]
    pub id: RecordId,
    pub title: String,
    pub description: String,
    pub thumbnail: String,
}

#[ComplexObject]
impl UserService {
    async fn id(&self) -> Option<String> {
        match &self.id.key {
            RecordIdKey::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, SurrealValue)]
pub struct UserResources {
    pub blog_posts: Vec<BlogPost>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject, SurrealValue)]
pub struct PublicSiteResources {
    pub blog_posts: Vec<BlogPost>,
    pub professional_info: Vec<UserProfessionalInfo>,
    pub portfolio: Vec<UserPortfolio>,
    pub resume: Vec<UserResume>,
    pub skills: Vec<UserSkill>,
    pub services: Vec<UserService>,
}

#[ComplexObject]
impl UserProfessionalInfo {
    async fn id(&self) -> Option<String> {
        match &self.id.key {
            RecordIdKey::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    async fn years_of_experience(&self) -> Option<u32> {
        // TODO: factor in months, currently only years e.g. 1 year 6 months
        let start_date = self.start_date.date_naive();
        let today = Utc::now().date_naive();

        today.years_since(start_date)
    }
}
