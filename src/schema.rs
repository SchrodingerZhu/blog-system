table! {
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

table! {
    pages (id) {
        id -> Int4,
        title -> Varchar,
        content -> Text,
    }
}

table! {
    posts (id) {
        id -> Int4,
        title -> Varchar,
        date -> Timestamp,
        tags -> Array<Text>,
        content -> Text,
        signature -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    comments,
    pages,
    posts,
);
