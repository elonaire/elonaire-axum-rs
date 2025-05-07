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
