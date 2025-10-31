use async_graphql::SimpleObject;
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
pub struct User {
    #[graphql(skip)]
    pub id: RecordId,
    pub user_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
pub struct UploadedFile {
    #[graphql(skip)]
    pub id: RecordId,
    pub file_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizationConstraint {
    pub roles: Vec<String>,
    pub privilege: Option<AdminPrivilege>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdminPrivilege {
    Admin,
    SuperAdmin,
}

impl TryFrom<i32> for AdminPrivilege {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AdminPrivilege::Admin),
            1 => Ok(AdminPrivilege::SuperAdmin),
            _ => Err("Invalid status"),
        }
    }
}

impl From<AdminPrivilege> for i32 {
    fn from(status: AdminPrivilege) -> Self {
        match status {
            AdminPrivilege::Admin => 0,
            AdminPrivilege::SuperAdmin => 1,
        }
    }
}
