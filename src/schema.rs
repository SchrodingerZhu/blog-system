diesel::table! {
    comments (id) {
        id -> Int4,
        post_id -> Int4,
        nickname -> Varchar,
        email -> Varchar,
        content -> Text,
        signature -> Text,
        finger_print -> Varchar,
    }
}


diesel::table! {
    pages (id) {
        id -> Int4,
        title -> Varchar,
        content -> Text,
    }
}


diesel::table! {
    posts (id) {
        id -> Int4,
        title -> Varchar,
        date -> Timestamp,
        tags -> Array<Text>,
        content -> Text,
        signature -> Text,
    }
}

diesel::joinable!(comments->posts(post_id));

diesel::allow_tables_to_appear_in_same_query!(
    comments,
    pages,
    posts,
);

