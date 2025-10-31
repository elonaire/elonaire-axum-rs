use crate::{convert_from_protobuf_bidirectionally, utils};

// should match the package name in the .proto file
pub mod acl_service {
    include!("out/acl.rs");
}

// should match the package name in the .proto file
pub mod email_service {
    include!("out/email.rs");
}

// should match the package name in the .proto file
pub mod files_service {
    include!("out/files.rs");
}

convert_from_protobuf_bidirectionally!(
    acl_service::AuthStatus,
    utils::models::AuthStatus,
    { sub, is_auth, current_role }
);

/// For easy conversion to protobuf
impl From<acl_service::ConfirmAuthenticationResponse> for utils::models::AuthStatus {
    fn from(auth_status: acl_service::ConfirmAuthenticationResponse) -> Self {
        Self {
            sub: auth_status.sub,
            is_auth: auth_status.is_auth,
            current_role: auth_status.current_role,
        }
    }
}

/// For easy conversion to protobuf
impl From<acl_service::AuthorizationConstraint> for utils::models::AuthorizationConstraint {
    fn from(authorization_constraint: acl_service::AuthorizationConstraint) -> Self {
        Self {
            roles: authorization_constraint.roles,
            privilege: Some(
                authorization_constraint
                    .privilege
                    .unwrap()
                    .try_into()
                    .unwrap(),
            ),
        }
    }
}

/// For easy conversion to protobuf
impl From<utils::models::AuthorizationConstraint> for acl_service::AuthorizationConstraint {
    fn from(authorization_constraint: utils::models::AuthorizationConstraint) -> Self {
        Self {
            roles: authorization_constraint.roles,
            privilege: Some(
                authorization_constraint
                    .privilege
                    .unwrap()
                    .try_into()
                    .unwrap(),
            ),
        }
    }
}
