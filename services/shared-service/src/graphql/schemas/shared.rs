use async_graphql::{ComplexObject, Enum, InputObject, OutputType, SimpleObject};
use lib::utils::models::{ApiResponse, CurrencyId, UploadedFileId};
use lib::utils::serialization::convert_float_to_string;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

use crate::graphql::schemas::blog::{BlogComment, BlogPost};
use crate::graphql::schemas::user::{
    PublicSiteResources, UserPortfolio, UserProfessionalInfo, UserResources, UserResume,
    UserService, UserSkill,
};

type BlogPosts = Vec<BlogPost>;
type Messages = Vec<Message>;
type Ratecards = Vec<Ratecard>;
type ServiceRates = Vec<ServiceRate>;
type ServiceRequests = Vec<ServiceRequest>;

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
    #[graphql(skip)]
    pub supporting_docs: Vec<RecordId>,
    pub description: String,
    pub start_date: String,
    pub engagement_length: String,
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
    pub supporting_docs: Vec<UploadedFileId>,
    #[graphql(skip)]
    pub description: String,
    pub start_date: String,
    pub engagement_length: String,
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
pub enum BillingInterval {
    #[graphql(name = "Hourly")]
    Hourly,
    #[graphql(name = "Weekly")]
    Weekly,
    #[graphql(name = "Monthly")]
    Monthly,
    #[graphql(name = "Annual")]
    Annual,
    #[graphql(name = "Milestone")]
    Milestone,
}

#[derive(SimpleObject)]
#[graphql(concrete(name = "UserProfessionalInfoResponse", params(UserProfessionalInfo)))]
#[graphql(concrete(name = "UserServiceResponse", params(UserService)))]
#[graphql(concrete(name = "UserPortfolioResponse", params(UserPortfolio)))]
#[graphql(concrete(name = "UserResumeResponse", params(UserResume)))]
#[graphql(concrete(name = "UserSkillResponse", params(UserSkill)))]
#[graphql(concrete(name = "BlogPostResponse", params(BlogPost)))]
#[graphql(concrete(name = "BlogPostsResponse", params(BlogPosts)))]
#[graphql(concrete(name = "BlogCommentResponse", params(BlogComment)))]
#[graphql(concrete(name = "ReactionResponse", params(Reaction)))]
#[graphql(concrete(name = "MessageResponse", params(Message)))]
#[graphql(concrete(name = "MessagesResponse", params(Messages)))]
#[graphql(concrete(name = "RatecardResponse", params(Ratecard)))]
#[graphql(concrete(name = "RatecardsResponse", params(Ratecards)))]
#[graphql(concrete(name = "ServiceRequestResponse", params(ServiceRequest)))]
#[graphql(concrete(name = "ServiceRequestsResponse", params(ServiceRequests)))]
#[graphql(concrete(name = "ServiceRateResponse", params(ServiceRate)))]
#[graphql(concrete(name = "ServiceRatesResponse", params(ServiceRates)))]
#[graphql(concrete(name = "UserResourcesResponse", params(UserResources)))]
#[graphql(concrete(name = "PublicSiteResourcesResponse", params(PublicSiteResources)))]
#[graphql(concrete(name = "StringResponse", params(String)))]
#[graphql(concrete(name = "BoolResponse", params(bool)))]
#[graphql(concrete(name = "U32Response", params(u32)))]
pub struct GraphQLApiResponse<T: OutputType> {
    pub data: T,
    pub metadata: GraphQLApiResponseMetadata,
}

#[derive(SimpleObject)]
pub struct GraphQLApiResponseMetadata {
    pub request_id: String,
    pub new_access_token: Option<String>,
}

impl<T: Send + Sync + Clone + OutputType> From<ApiResponse<T>> for GraphQLApiResponse<T> {
    fn from(standard_res: ApiResponse<T>) -> Self {
        Self {
            data: standard_res.get_data(),
            metadata: GraphQLApiResponseMetadata {
                request_id: standard_res.get_request_id(),
                new_access_token: standard_res.get_new_access_token(),
            },
        }
    }
}
