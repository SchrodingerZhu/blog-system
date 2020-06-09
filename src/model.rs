use chrono::Utc;
use crate::schema::{posts, comments, pages};

#[derive(diesel::Queryable, diesel::Associations, diesel::Identifiable, Debug)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub date: chrono::NaiveDateTime,
    pub tags: Vec<String>,
    pub content: String,
    pub signature: String
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

#[derive(Insertable)]
#[table_name="posts"]
pub struct NewPost<'a> {
    pub title: &'a str,
    pub date: &'a chrono::NaiveDateTime,
    pub tags: &'a [String],
    pub content: &'a str,
    pub signature: &'a str
}

#[derive(diesel::Queryable, diesel::Identifiable)]
pub struct Page {
    pub id: i32,
    pub title: String,
    pub content: String,
}

#[derive(diesel::Queryable, diesel::Identifiable,  diesel::Associations)]
#[belongs_to(Post)]
pub struct Comment {
    pub id: i32,
    pub post_id: i32,
    pub nickname: String,
    pub email: String,
    pub content: String,
    pub signature: String,
    pub finger_print: String
}

#[derive(Insertable)]
#[table_name="comments"]
pub struct NewComment<'a> {
    pub post_id: i32,
    pub nickname: &'a str,
    pub email: &'a str,
    pub content: &'a str,
    pub signature: &'a str,
    pub finger_print: &'a str
}

impl Comment {
    pub fn render_safe_content(&self) -> String {
        let parser = pulldown_cmark::Parser::new(self.content.as_str());
        let mut buffer = String::with_capacity(1024);
        pulldown_cmark::html::push_html(&mut buffer, parser);
        ammonia::clean(buffer.as_str())
    }
}