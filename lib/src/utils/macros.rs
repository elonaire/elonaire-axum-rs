#[macro_export]
macro_rules! convert_from_protobuf_bidirectionally {
    ($type_a:path, $type_b:path, { $($field:ident),+ $(,)? }) => {
        impl From<$type_a> for $type_b {
            fn from(src: $type_a) -> Self {
                Self {
                    $( $field: src.$field, )+
                }
            }
        }

        impl From<$type_b> for $type_a {
            fn from(src: $type_b) -> Self {
                Self {
                    $( $field: src.$field, )+
                }
            }
        }
    };
}
