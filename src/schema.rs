// @generated automatically by Diesel CLI.

diesel::table! {
    messages (id) {
        id -> Int4,
        #[max_length = 128]
        username -> Varchar,
        message -> Text,
        timestamp -> Int8,
    }
}
