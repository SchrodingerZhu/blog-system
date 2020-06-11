use askama::*;
use crate::model::{Post, Comment, Page};

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
#[template(path = "search.html")]
pub struct PostsSearch<'a> {
    pub blog_name: &'a str,
    pub posts: Vec<Post>,
    pub page_number: i64,
    pub search: &'a str,
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

#[derive(serde::Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct Tag<'a> {
    pub count: i32,
    pub tag: &'a str,
}

#[derive(Template)]
#[template(path = "tags.html")]
pub struct TagsTemplate<'a> {
    pub tags_json: String,
    pub blog_name: &'a str,
    pub tags: Vec<Tag<'a>>
}

#[derive(Template)]
#[template(path = "remove_comment.html")]
pub struct RemoveCommentTemplate<'a> {
    pub json: String,
    pub blog_name: &'a str,
}

#[derive(Template)]
#[template(path = "page.html")]
pub struct PageTemplate<'a> {
    pub blog_name: &'a str,
    pub page: &'a Page,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    pub blog_name: &'a str,
    pub pages: &'a [Page],
    pub important_pages: &'a [(String, i32)]
}
