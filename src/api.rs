use async_diesel::*;
use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl};

use crate::api::JsonResponse::*;
use crate::ConnPool;
use crate::model::{Comment, NewPageRaw, Post, Page, POST_COLUMNS, NewPostRaw};

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
        tag: Vec<String>,
    },
    PostComments(i32),
    PageUpdate {
        id: i32,
        title: Option<String>,
        content: Option<String>,
        important: Option<bool>,
        description: Option<String>,
    },
    PageCreate {
        title: String,
        content: String,
        description: String,
        important: bool,
    },
    ListOperation { list_type: ModelType },
    CheckOperation { id: i32, check_type: ModelType },
    DeleteOperation { id: i32, delete_type: ModelType },
}


#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "response_type", content = "response_body")]
pub enum JsonResponse {
    PostList(Vec<(i32, String, chrono::NaiveDateTime, chrono::NaiveDateTime)>),
    PostSearchList(Vec<Post>),
    CommentList(Vec<Comment>),
    PageList(Vec<(i32, String)>),
    PostInfo(Post),
    PageInfo(Page),
    CommentInfo(Comment),
    Error(String),
    Success(usize),
}

impl JsonResponse {
    pub fn get_raw_content(&self) -> anyhow::Result<&str> {
        match self {
            PostInfo(Post { content, .. }) | CommentInfo(Comment { content, .. }) | PageInfo(Page { content, .. })
            => Ok(content.as_str()),
            _ => Err(anyhow::anyhow!("cannot get raw content from server reply"))
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "tag", content = "content")]
pub enum ModelType {
    Post,
    Page,
    Comment,
}

impl<T: ToString> From<T> for JsonResponse {
    fn from(e: T) -> Self {
        Error(e.to_string())
    }
}

impl JsonRequest {
    pub async fn handle(self, conn: &ConnPool) -> JsonResponse {
        use JsonRequest::*;
        let conn = &conn;
        match self {
            PostUpdate { id, title, tags, content } => {
                use crate::schema::posts::dsl as p;
                let time = Utc::now().naive_local();
                let change_set = NewPostRaw {
                    title,
                    update_date: Some(time),
                    public_date: None,
                    tags,
                    content,
                };
                diesel::update(p::posts.filter(p::id.eq(id)))
                    .set(change_set)
                    .execute_async(conn)
                    .await
                    .map(|s| Success(s))
                    .unwrap_or_else(Into::into)
            }
            PostSearch(search) => {
                conn.get().map(|x|
                    Post::list(&x, search.as_str(), None)
                        .map(|x| PostSearchList(x))
                        .unwrap_or_else(Into::into)
                ).unwrap_or_else(Into::into)
            }
            PageCreate { title, content, important, description } => {
                use crate::schema::pages::dsl as p;
                diesel::insert_into(p::pages)
                    .values(NewPageRaw {
                        title: Some(title),
                        content: Some(content),
                        important: Some(important),
                        description: Some(description),
                    })
                    .execute_async(conn)
                    .await
                    .map(|s| Success(s))
                    .unwrap_or_else(Into::into)
            }
            PageUpdate { id, title, content, important, description } => {
                use crate::schema::pages::dsl as p;
                let change_set = NewPageRaw {
                    title,
                    content,
                    important,
                    description,
                };
                diesel::update(p::pages.filter(p::id.eq(id)))
                    .set(change_set)
                    .execute_async(conn)
                    .await
                    .map(|s| Success(s))
                    .unwrap_or_else(Into::into)
            }
            PostCreate { title, content, tag } => {
                use crate::schema::posts::dsl as p;
                let time = Utc::now().naive_local();
                let mut tag: Vec<String> = tag.iter()
                    .map(|x| x.trim().to_ascii_lowercase()).collect();
                tag.sort();
                tag.dedup_by(|x, y| x == y);
                diesel::insert_into(p::posts)
                    .values(NewPostRaw {
                        title: Some(title),
                        public_date: Some(time),
                        update_date: Some(time),
                        tags: Some(tag),
                        content: Some(content),
                    })
                    .execute_async(conn)
                    .await
                    .map(|s| Success(s))
                    .unwrap_or_else(Into::into)
            }
            PostComments(post_id) => {
                use crate::schema::comments::dsl as c;
                c::comments.filter(c::post_id.eq(post_id))
                    .load_async(conn)
                    .await
                    .map(|x| CommentList(x))
                    .unwrap_or_else(Into::into)
            }
            ListOperation { list_type } => {
                match list_type {
                    ModelType::Comment => {
                        use crate::schema::comments::dsl as c;
                        c::comments.load_async(conn)
                            .await
                            .map(|x| CommentList(x))
                            .unwrap_or_else(Into::into)
                    }
                    ModelType::Post => {
                        use crate::schema::posts::dsl as p;
                        p::posts
                            .select((p::id, p::title, p::public_date, p::update_date))
                            .load_async(conn)
                            .await
                            .map(|x| PostList(x))
                            .unwrap_or_else(Into::into)
                    }
                    ModelType::Page => {
                        use crate::schema::pages::dsl as p;
                        p::pages
                            .select((p::id, p::title))
                            .load_async(conn)
                            .await
                            .map(|x| PageList(x))
                            .unwrap_or_else(Into::into)
                    }
                }
            }
            CheckOperation { id, check_type } => {
                match check_type {
                    ModelType::Comment => {
                        use crate::schema::comments::dsl as c;
                        c::comments
                            .filter(c::id.eq(id))
                            .first_async(conn)
                            .await
                            .map(|x| CommentInfo(x))
                            .unwrap_or_else(Into::into)
                    }
                    ModelType::Post => {
                        use crate::schema::posts::dsl as p;
                        p::posts
                            .select(POST_COLUMNS)
                            .filter(p::id.eq(id))
                            .first_async(conn)
                            .await
                            .map(|x| PostInfo(x))
                            .unwrap_or_else(Into::into)
                    }
                    ModelType::Page => {
                        use crate::schema::pages::dsl as p;
                        p::pages
                            .filter(p::id.eq(id))
                            .first_async(conn)
                            .await
                            .map(|x| PageInfo(x))
                            .unwrap_or_else(Into::into)
                    }
                }
            }
            DeleteOperation { id, delete_type } => {
                match delete_type {
                    ModelType::Comment => {
                        use crate::schema::comments::dsl as c;
                        diesel::delete(c::comments
                            .filter(c::id.eq(id)))
                            .execute_async(conn)
                            .await
                            .map(|s| Success(s))
                            .unwrap_or_else(Into::into)
                    }
                    ModelType::Post => {
                        use crate::schema::posts::dsl as p;
                        diesel::delete(p::posts
                            .filter(p::id.eq(id)))
                            .execute_async(conn)
                            .await
                            .map(|s| Success(s))
                            .unwrap_or_else(Into::into)
                    }
                    ModelType::Page => {
                        use crate::schema::pages::dsl as p;
                        diesel::delete(p::pages
                            .filter(p::id.eq(id)))
                            .execute_async(conn)
                            .await
                            .map(|s| Success(s))
                            .unwrap_or_else(Into::into)
                    }
                }
            }
        }
    }
}
