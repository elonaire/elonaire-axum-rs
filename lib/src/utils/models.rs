use async_graphql::{ComplexObject, SimpleObject};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
pub struct AuthStatus {
    pub is_auth: bool,
    pub sub: String,
    pub current_role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForeignKey {
    pub table: String,
    pub column: String,
    pub foreign_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
pub struct UserId {
    #[graphql(skip)]
    pub id: RecordId,
    pub user_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct CurrencyId {
    #[graphql(skip)]
    pub id: RecordId,
    pub currency_id: String,
}

#[ComplexObject]
impl CurrencyId {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct UploadedFileId {
    #[graphql(skip)]
    pub id: RecordId,
    pub file_id: String,
}

#[ComplexObject]
impl UploadedFileId {
    async fn id(&self) -> String {
        self.id.key().to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizationConstraint {
    pub permissions: Vec<String>,
    pub privilege: AdminPrivilege,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy, Eq)]
pub enum AdminPrivilege {
    Admin,
    SuperAdmin,
    None,
}

impl TryFrom<i32> for AdminPrivilege {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AdminPrivilege::Admin),
            1 => Ok(AdminPrivilege::SuperAdmin),
            2 => Ok(AdminPrivilege::None),
            _ => Err("Invalid status"),
        }
    }
}

impl From<AdminPrivilege> for i32 {
    fn from(status: AdminPrivilege) -> Self {
        match status {
            AdminPrivilege::Admin => 0,
            AdminPrivilege::SuperAdmin => 1,
            AdminPrivilege::None => 2,
        }
    }
}
