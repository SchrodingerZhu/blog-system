diesel::table! {
    comments (id) {
        id -> Int4,
        post_id -> Int4,
        nickname -> Varchar,
        email -> Varchar,
        content -> Text,
        signature -> Text,
        finger_print -> Varchar,
        sha3_512 -> Bytea,
    }
}

diesel::table! {
    pages (id) {
        id -> Int4,
        title -> Varchar,
        content -> Text,
        important -> Bool,
        description -> Text,
    }
}

diesel::table! {
    posts (id) {
        id -> Int4,
        title -> Varchar,
        public_date -> Timestamp,
        update_date -> Timestamp,
        tags -> Array<Text>,
        content -> Text,
        text_searchable -> diesel_full_text_search::TsVector,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    comments,
    pages,
    posts,
);

diesel::sql_function!(fn lower(x: diesel::types::Varchar) -> diesel::types::Varchar);
diesel::sql_function!(fn translate(x: diesel::types::Varchar, y: diesel::types::Varchar, z: diesel::types::Varchar) -> diesel::types::Varchar);
diesel::sql_function!(fn
    concat(a: diesel::types::Varchar,
           b: diesel::types::Varchar,
           c: diesel::types::Varchar,
           d: diesel::types::Varchar) -> diesel::types::Varchar);