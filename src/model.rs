use http_types::{Status, StatusCode};

use crate::Conn;
use crate::schema::{comments, pages, posts};

#[derive(diesel::Queryable, diesel::Associations, diesel::Identifiable, Debug, serde::Serialize, serde::Deserialize)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub date: chrono::NaiveDateTime,
    pub tags: Vec<String>,
    pub content: String,
}

impl Post {
    pub fn render_content(&self) -> String {
        let cmark = pulldown_cmark::Parser::new(self.content.as_str());
        let mut buffer = String::with_capacity(1024);
        pulldown_cmark::html::push_html(&mut buffer, cmark);
        buffer
    }

    pub fn get_abstract(&self) -> String {
        self.content.chars().take(256).collect()
    }
}

#[derive(Insertable, Debug, Clone, diesel::AsChangeset)]
#[table_name = "posts"]
pub struct NewPost<'a> {
    pub title: Option<&'a str>,
    pub date: Option<&'a chrono::NaiveDateTime>,
    pub tags: Option<&'a [String]>,
    pub content: Option<&'a str>,
}

#[derive(Insertable, Debug, Clone, diesel::AsChangeset)]
#[table_name = "pages"]
pub struct NewPage<'a> {
    pub title: Option<&'a str>,
    pub content: Option<&'a str>,
    pub important: Option<bool>,
}

#[derive(diesel::Queryable, diesel::Identifiable, serde::Serialize, Debug, serde::Deserialize)]
pub struct Page {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub important: bool,
}

impl Page {
    pub fn render_abstract(&self) -> String {
        self.content.chars().take(128).collect()
    }
}

#[derive(diesel::Queryable, diesel::Identifiable, diesel::Associations, serde::Serialize, Debug, serde::Deserialize)]
#[belongs_to(Post)]
pub struct Comment {
    pub id: i32,
    pub post_id: i32,
    pub nickname: String,
    pub email: String,
    pub content: String,
    pub signature: String,
    pub finger_print: String,
    pub sha3_512: Vec<u8>,
}

#[derive(Insertable)]
#[table_name = "comments"]
pub struct NewComment<'a> {
    pub post_id: i32,
    pub nickname: &'a str,
    pub email: &'a str,
    pub content: &'a str,
    pub signature: &'a str,
    pub finger_print: &'a str,
    pub sha3_512: &'a [u8],
}

impl Comment {
    pub fn render_safe_content(&self) -> String {
        let parser = pulldown_cmark::Parser::new(self.content.as_str());
        let mut buffer = String::with_capacity(1024);
        pulldown_cmark::html::push_html(&mut buffer, parser);
        ammonia::clean(buffer.as_str())
    }
}

impl Page {
    pub fn render_content(&self) -> String {
        let cmark = pulldown_cmark::Parser::new(self.content.as_str());
        let mut buffer = String::with_capacity(1024);
        pulldown_cmark::html::push_html(&mut buffer, cmark);
        buffer
    }
}

pub type PostColumns = (
    crate::schema::posts::id,
    crate::schema::posts::title,
    crate::schema::posts::date,
    crate::schema::posts::tags,
    crate::schema::posts::content
);


pub const POST_COLUMNS: PostColumns = (
    crate::schema::posts::id,
    crate::schema::posts::title,
    crate::schema::posts::date,
    crate::schema::posts::tags,
    crate::schema::posts::content
);

impl Post {
    pub fn list(connection: &Conn, search: &str, page_number: Option<i64>) -> tide::Result<Vec<Self>> {
        use diesel::RunQueryDsl;
        use diesel::QueryDsl;
        use diesel::pg::Pg;
        use crate::schema::posts::dsl::*;
        use crate::schema;
        use diesel_full_text_search::{plainto_tsquery, TsVectorExtensions};

        let mut query = schema::posts::table.into_boxed::<Pg>();

        if !search.is_empty() {
            query = query
                .filter(text_searchable.matches(plainto_tsquery(search)));
        }
        if let Some(page_number) = page_number {
            query
                .select(POST_COLUMNS)
                .limit(20)
                .offset(page_number * 20)
                .load::<Post>(connection)
                .status(StatusCode::InternalServerError)
        } else {
            query
                .select(POST_COLUMNS)
                .load::<Post>(connection)
                .status(StatusCode::InternalServerError)
        }
    }
}

diesel::joinable!(comments -> posts (post_id));