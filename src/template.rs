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
    pub posts: Vec<Post>
}

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFound;
