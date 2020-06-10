use askama::*;
use chrono::Utc;
use crate::model::{Post, Comment};

#[derive(Template)]
#[template(path = "post.html")]
pub struct PostTemplate {
    pub post: Post,
    pub comments: Vec<Comment>
}

#[derive(Template)]
#[template(path = "posts.html")]
pub struct PostsTemplate<'a> {
    pub blog_name: &'a str,
    pub posts: Vec<Post>,
    pub page_number: i64
}

#[derive(Template)]
#[template(path = "tag.html")]
pub struct TagTemplate<'a> {
    pub blog_name: &'a str,
    pub name: &'a str,
    pub posts: Vec<Post>,
    pub page_number: i64
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub code: String,
    pub message: Option<String>
}

#[derive(Template)]
#[template(path = "tags.html")]
pub struct TagsTemplate {
    pub tags_json: String,
}
