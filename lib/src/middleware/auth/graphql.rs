use crate::{
    integration::grpc::clients::acl_service::{acl_client::AclClient, Empty},
    utils::{
        auth::AuthStatus,
        grpc::{create_grpc_client, AuthMetaData},
    },
};

use hyper::{
    header::{AUTHORIZATION, COOKIE},
    HeaderMap,
};
use std::io::{Error, ErrorKind};
use tonic::transport::Channel;

/// False middleware for checking authentication from ACL service for GraphQL requests.
/// I used this anti-pattern because the middleware in async-graphql just doesn't work. The headers are not properly parsed.
pub async fn check_auth_from_acl(headers: &HeaderMap) -> Result<AuthStatus, Error> {
    let auth_header = headers.get(AUTHORIZATION);
    let cookie_header = headers.get(COOKIE);

    let mut request = tonic::Request::new(Empty {});

    let auth_metadata: AuthMetaData<Empty> = AuthMetaData {
        auth_header,
        cookie_header,
        constructed_grpc_request: Some(&mut request),
    };

    let mut acl_grpc_client = create_grpc_client::<Empty, AclClient<Channel>>(
        "http://[::1]:50051",
        true,
        Some(auth_metadata),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to connect to ACL service: {}", e);
        Error::new(ErrorKind::Other, "Failed to connect to ACL service")
    })?;

    let response = acl_grpc_client.confirm_authentication(request).await;

    match response {
        Ok(response) => {
            let current_user = response.into_inner().sub;
            Ok(AuthStatus {
                sub: current_user,
                is_auth: true,
            })
        }
        Err(_e) => return Err(Error::new(ErrorKind::PermissionDenied, "Unauthorized")),
    }
}
