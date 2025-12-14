use async_graphql::{ComplexObject, Enum, InputObject, SimpleObject};
use lib::utils::models::{CurrencyId, UploadedFileId};
use lib::utils::serialization::convert_float_to_string;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

use crate::graphql::schemas::user::UserService;

// Reaction
#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct ReactionInput {
    pub r#type: ReactionType,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct Reaction {
    #[graphql(skip)]
    pub id: RecordId,
    pub r#type: ReactionType,
}

// enum for ReactionType
#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq)]
pub enum ReactionType {
    #[graphql(name = "Like")]
    Like,
    #[graphql(name = "Dislike")]
    Dislike,
    #[graphql(name = "Love")]
    Love,
    #[graphql(name = "Haha")]
    Haha,
    #[graphql(name = "Wow")]
    Wow,
    #[graphql(name = "Sad")]
    Sad,
    #[graphql(name = "Angry")]
    Angry,
}

#[ComplexObject]
impl Reaction {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct MessageInput {
    pub subject: Subject,
    pub body: String,
    pub sender_name: String,
    pub sender_email: String,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct Message {
    #[graphql(skip)]
    pub id: RecordId,
    pub subject: Subject,
    pub body: String,
    pub sender_name: String,
    pub sender_email: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq)]
pub enum Subject {
    #[graphql(name = "JobOffer")]
    JobOffer,
    #[graphql(name = "Consultation")]
    Consultation,
    #[graphql(name = "Feedback")]
    Feedback,
    #[graphql(name = "Complaint")]
    Complaint,
    #[graphql(name = "Enquiry")]
    Enquiry,
    #[graphql(name = "Suggestion")]
    Suggestion,
}

#[ComplexObject]
impl Message {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct ServiceRateInput {
    #[graphql(skip)]
    pub service: Option<RecordId>,
    pub base_rate: String,
    pub hour_week: Option<u8>,
    #[graphql(skip)]
    pub currency_id: Option<RecordId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct ServiceRateInputMetadata {
    pub service_id: String,
    pub currency_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct ServiceRate {
    #[graphql(skip)]
    pub id: RecordId,
    pub service: UserService,
    #[graphql(skip)]
    pub base_rate: f64,
    pub hour_week: u8,
    pub currency_id: CurrencyId,
    pub created_at: String,
    pub updated_at: String,
}

#[ComplexObject]
impl ServiceRate {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }

    // To prevent loss of precision during serialization and deserialization
    async fn base_rate(&self) -> String {
        convert_float_to_string(self.base_rate)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct RatecardInput {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct RatecardInputMetadata {
    pub service_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct Ratecard {
    #[graphql(skip)]
    pub id: RecordId,
    pub name: String,
    pub services: Vec<UserService>,
    pub created_at: String,
    pub updated_at: String,
}

#[ComplexObject]
impl Ratecard {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct ServiceRequestInput {
    pub description: String,
    #[graphql(skip)]
    pub supporting_docs: Vec<RecordId>,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, InputObject)]
pub struct ServiceRequestInputMetadata {
    pub supporting_docs_file_ids: Vec<String>,
    pub service_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct ServiceRequest {
    #[graphql(skip)]
    pub id: RecordId,
    pub description: String,
    pub supporting_docs: Vec<UploadedFileId>,
    pub start_date: String,
    pub end_date: String,
    pub created_at: String,
    pub updated_at: String,
}

#[ComplexObject]
impl ServiceRequest {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Enum, Copy, Eq, PartialEq)]
pub enum BillingPeriod {
    #[graphql(name = "Hourly")]
    Hourly,
    #[graphql(name = "Weekly")]
    Weekly,
    #[graphql(name = "Monthly")]
    Monthly,
    #[graphql(name = "Annual")]
    Annual,
}
