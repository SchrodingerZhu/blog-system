use crate::model::{NewPage, NewPost, Post, Comment, Page};
use crate::Conn;
use std::time::SystemTime;
use chrono::Utc;
use diesel::{QueryDsl, ExpressionMethods, RunQueryDsl};
use crate::api::JsonResponse::*;
use diesel::expression::array_comparison::In;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "request_type", content = "request_body")]
pub enum JsonRequest {
    PostUpdate {
        id: i32,
        title: Option<String>,
        tags: Option<Vec<String>>,
        content: Option<String>,
    },
    PostSearch(String),
    PostCreate {
        title: String,
        content: String,
        tag: Vec<String>
    },
    PostComments(i32),
    PageUpdate {
        id: i32,
        title: Option<String>,
        content: Option<String>,
        important: Option<bool>
    },
    PageCreate {
        title: String,
        content: String,
        important: bool
    },
    ListOperation { list_type: ModelType },
    CheckOperation { id: i32, check_type: ModelType, },
    DeleteOperation { id: i32, delete_type: ModelType, },
}


#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "response_type", content = "response_body")]
pub enum JsonResponse {
    PostList(Vec<(i32, String, chrono::NaiveDateTime)>),
    PostSearchList(Vec<Post>),
    CommentList(Vec<Comment>),
    PageList(Vec<(i32, String)>),
    PostInfo(Post),
    PageInfo(Page),
    CommentInfo(Comment),
    Error(String),
    Success
}


#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "tag", content = "content")]
pub enum ModelType {
    Post,
    Page,
    Comment,
}

impl<T : ToString> From<T> for JsonResponse {
    fn from(e: T) -> Self {
        Error(e.to_string())
    }
}

impl JsonRequest {
    pub async fn handle(&self, conn: &Conn) -> JsonResponse {
        use JsonRequest::*;
        match self {
            PostUpdate { id, title, tags, content } => {
                use crate::schema::posts::dsl as p;
                let time = Utc::now().naive_local();
                let change_set = NewPost {
                    title: title.as_ref().map(|x|x.as_str()),
                    date: Some(&time),
                    tags: tags.as_ref().map(|x|x.as_slice()),
                    content: content.as_ref().map(|x| x.as_str())
                };
                diesel::update(p::posts.filter(p::id.eq(*id)))
                    .set(change_set)
                    .execute(conn)
                    .map(|_| Success)
                    .unwrap_or_else(Into::into)
            }
            PostSearch(search) => {
                Post::list(conn, search.as_str(), None)
                    .map(|x|PostSearchList(x))
                    .unwrap_or_else(Into::into)
            }
            PageCreate { title, content, important } => {
                use crate::schema::pages::dsl as p;
                diesel::insert_into(p::pages)
                    .values(NewPage {
                        title: Some(title),
                        content: Some(content),
                        important: Some(*important)
                    })
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .map(|_| Success)
                    .unwrap_or_else(Into::into)
            }
            PageUpdate { id, title, content, important } => {
                use crate::schema::pages::dsl as p;
                let time = Utc::now().naive_local();
                let change_set = NewPage {
                    title: title.as_ref().map(|x|x.as_str()),
                    content: content.as_ref().map(|x| x.as_str()),
                    important: important.clone()
                };
                diesel::update(p::pages.filter(p::id.eq(*id)))
                    .set(change_set)
                    .execute(conn)
                    .map(|_| Success)
                    .unwrap_or_else(Into::into)
            }
            PostCreate { title, content, tag } => {
                use crate::schema::posts::dsl as p;
                let time = Utc::now().naive_local();
                diesel::insert_into(p::posts)
                    .values(NewPost {
                        title: Some(title),
                        date: Some(&time),
                        tags: Some(tag),
                        content: Some(content),
                    })
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .map(|_| Success)
                    .unwrap_or_else(Into::into)
            }
            PostComments(post_id) => {
                use crate::schema::comments::dsl as c;
                c::comments.filter(c::post_id.eq(*post_id))
                    .load(conn)
                    .map(|x| CommentList(x))
                    .unwrap_or_else(Into::into)
            }
            ListOperation { list_type } => {
                match list_type {
                    ModelType::Comment => {
                        use crate::schema::comments::dsl as c;
                        c::comments.load(conn)
                            .map(|x| CommentList(x))
                            .unwrap_or_else(Into::into)
                    }
                    ModelType::Post => {
                        use crate::schema::posts::dsl as p;
                        p::posts
                            .select((p::id, p::title, p::date))
                            .load(conn)
                            .map(|x| PostList(x))
                            .unwrap_or_else(Into::into)
                    }
                    ModelType::Page => {
                        use crate::schema::pages::dsl as p;
                        p::pages
                            .select((p::id, p::title))
                            .load(conn)
                            .map(|x| PageList(x))
                            .unwrap_or_else(Into::into)
                    }
                }
            }
            _ => unimplemented!()
        }
    }
}