use std::io::Write;
use std::pin::Pin;

use askama::Template;
use async_std::prelude::Future;
use diesel::prelude::*;
use tide::{Request, Response, Status, StatusCode, Redirect};

use crate::{ConnPool, ServerState};
use crate::model::{Comment, NewComment, Post, POST_COLUMNS};
use crate::template::{PostsTemplate, Tag, TagTemplate};

static EMAIL_REGEX: &str = "^[A-Za-z0-9._%-]+@[A-Za-z0-9.-]+[.][A-Za-z]+$";

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output=T> + Send + 'a>>;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "request_type", content = "request_body")]
pub enum JsonRequest {
    ListPosts
}

pub async fn serve_posts(request: Request<ServerState>) -> tide::Result<tide::Response> {
    use crate::schema::posts::dsl::*;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let page_number: i64 = {
        let pat = request.url()
            .path()
            .trim()
            .trim_start_matches("/");
        if pat.is_empty() { 0 } else { pat.parse()? }
    };
    let all_posts = posts
        .select(POST_COLUMNS)
        .limit(20)
        .offset(page_number * 20)
        .load::<Post>(&conn)
        .status(StatusCode::InternalServerError)?;

    let posts_template = PostsTemplate {
        blog_name: request.state().blog_name.as_str(),
        posts: all_posts,
        page_number,
    };
    let page = posts_template.render()?;
    let mut responce = tide::Response::new(StatusCode::Ok);
    responce.set_body(page);
    responce.set_content_type(http_types::mime::HTML);
    Ok(
        responce
    )
}

#[inline(always)]
pub fn normal_page<A: Into<tide::Body>>(page: A) -> tide::Response {
    let mut responce = tide::Response::new(StatusCode::Ok);
    responce.set_body(page.into());
    responce.set_content_type(http_types::mime::HTML);
    responce
}

pub async fn error_handle(result: tide::Result<Response>) -> tide::Result<tide::Response> {
    let mut response = result.map(|x| (None, x)).unwrap_or_else(|e| {
        let status = e.status();
        (Some(e), Response::new(status))
    });
    if !response.1.status().is_success() {
        let page = crate::template::ErrorTemplate {
            code: response.1.status().to_string(),
            message: response.0.map(|x| x.to_string()),
        };
        response.1.set_body(page.render()?);
        response.1.set_content_type(http_types::mime::HTML);
    }
    Ok(
        response.1
    )
}

pub async fn serve_post(request: Request<ServerState>) -> tide::Result<Response> {
    use crate::schema::posts::dsl as p;
    use crate::schema::comments::dsl as c;
    let path = request.url().path().trim_start_matches("/");
    if path.contains('/') || !path.ends_with(".html") {
        return Ok(Response::new(StatusCode::NotFound));
    }
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let name = path.trim_end_matches(".html");
    let post: Post = p::posts
        .select(POST_COLUMNS)
        .filter(p::title.eq(name))
        .first::<Post>(&conn)?;
    let all_comments = Comment::belonging_to(&post)
        .load::<Comment>(&conn)?;
    let template = crate::template::PostTemplate {
        post,
        comments: all_comments,
    };
    let page = template.render()?;
    Ok(normal_page(page))
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CommentForm {
    post_id: i32,
    comment_nickname: String,
    comment_email: String,
    comment_content: String,
}


pub async fn handle_comment(mut request: Request<ServerState>) -> tide::Result<Response> {
    use crate::schema::posts::dsl as p;
    use crate::schema::comments::dsl as c;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let form: CommentForm = request.body_form()
        .await?;
    if form.comment_nickname.is_empty() ||
        !regex::Regex::new(EMAIL_REGEX)?.is_match(form.comment_email.as_str()) {
        return Err(tide::Error::from_str(StatusCode::BadRequest, "invalid field"));
    }
    match diesel::dsl::select(diesel::dsl::exists(p::posts.filter(p::id.eq(form.post_id))))
        .get_result(&conn) {
        Ok(true) => (),
        _ => {
            return Err(tide::Error::from_str(StatusCode::BadRequest, "invalid post id"));
        }
    }
    let (real_content, finger_print) = crate::utils::gpg_decrypt(form.comment_content.as_str())
        .map_err(|x| tide::Error::from_str(StatusCode::BadRequest, "verification error"))?;

    let hash = easy_hasher::easy_hasher::sha3_512(&form.comment_content)
        .to_vec();
    let comment = NewComment {
        post_id: form.post_id,
        nickname: form.comment_nickname.as_str(),
        email: form.comment_email.as_str(),
        content: real_content.as_str(),
        signature: form.comment_content.as_str(),
        finger_print: finger_print.as_str(),
        sha3_512: &hash,
    };
    diesel::insert_into(c::comments)
        .values(&comment)
        .execute(&conn)?;
    let title : String = p::posts.select(p::title).filter(p::id.eq(form.post_id))
        .first(&conn)?;
    Ok(Redirect::new(format!("/post/{}.html", title)).into())
}

pub async fn serve_tag(request: Request<ServerState>) -> tide::Result<tide::Response> {
    use crate::schema::posts::dsl::*;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let url = request.url().path().trim_start_matches("/");
    let url_split = url.split("/").collect::<Vec<&str>>();
    if url_split.len() != 1 && url_split.len() != 2 {
        return Err(tide::Error::from_str(StatusCode::BadRequest, "unable to parse request url"));
    }
    let page_number = if url_split.len() == 2 {
        url_split[1].parse()?
    } else { 0 };

    let all_posts = posts
        .select(POST_COLUMNS)
        .filter(tags.contains(vec![url_split[0]]))
        .limit(20)
        .offset(page_number * 20)
        .load::<Post>(&conn)
        .status(StatusCode::InternalServerError)?;

    let tag_template = TagTemplate {
        blog_name: request.state().blog_name.as_str(),
        name: url_split[0],
        posts: all_posts,
        page_number,
    };
    let page = tag_template.render()?;
    Ok(
        normal_page(page)
    )
}

pub async fn serve_comment_signature(request: Request<ServerState>) -> tide::Result<String> {
    use crate::schema::comments::dsl::*;
    let cid: i32 = request.url().path().trim_start_matches("/").parse()
        .status(StatusCode::BadRequest)?;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let target: String = comments.select(signature)
        .filter(id.eq(cid))
        .first(&conn)
        .status(StatusCode::InternalServerError)?;
    Ok(target)
}

pub async fn serve_post_signature(request: Request<ServerState>) -> tide::Result<String> {
    use crate::schema::posts::dsl::*;
    let cid: i32 = request.url().path().trim_start_matches("/").parse()
        .status(StatusCode::BadRequest)?;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let target: String = posts.select(signature)
        .filter(id.eq(cid))
        .first(&conn)
        .status(StatusCode::InternalServerError)?;

    Ok(target)
}


pub async fn serve_tags(request: Request<ServerState>) -> tide::Result<Response> {
    use crate::schema::posts::dsl::*;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let all_tags: Vec<String> = posts.select(tags)
        .load::<Vec<String>>(&conn)
        .status(StatusCode::InternalServerError)?
        .into_iter()
        .flatten()
        .collect();
    let map = all_tags.iter()
        .fold(hashbrown::HashMap::new(), |mut acc, next| {
            *acc.entry(next).or_insert(0) += 1;
            acc
        });
    let mut tag_vector = Vec::new();
    for (tag, count) in &map {
        tag_vector.push(Tag { tag, count: *count });
    }
    tag_vector.sort_by(|x, y| y.cmp(x));
    let template = crate::template::TagsTemplate {
        tags_json: simd_json::to_string(&tag_vector)?,
        tags: tag_vector,
        blog_name: request.state().blog_name.as_str(),
    };
    Ok(
        normal_page(template.render()?)
    )
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct SearchForm {
    search: String,
    page_number: i64,
}

pub async fn handle_search(mut request: Request<ServerState>) -> tide::Result<Response> {
    let form: SearchForm = request.body_form().await?;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let all_posts = crate::model::Post::list(&conn,
                                             form.search.as_str(),
                                             form.page_number)?;
    let template = crate::template::PostsSearch {
        blog_name: request.state().blog_name.as_str(),
        posts: all_posts,
        page_number: form.page_number,
        search: form.search.as_str(),
    };

    Ok(
        normal_page(template.render()?)
    )
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RemoveComment {
    id: i32,
    time_stamp: std::time::SystemTime,
}

pub async fn remove_comment(request: Request<ServerState>) -> tide::Result<Response> {
    use crate::schema::comments::dsl::*;
    let cid: i32 = request.url().path().trim_start_matches("/").parse()
        .status(StatusCode::BadRequest)?;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    match diesel::dsl::select(diesel::dsl::exists(comments.filter(id.eq(cid))))
        .get_result(&conn) {
        Ok(true) => (),
        _ => {
            return Err(tide::Error::from_str(StatusCode::BadRequest, "invalid post id"));
        }
    }
    let remove_comment = RemoveComment {
        id: cid,
        time_stamp: std::time::SystemTime::now(),
    };

    let json = simd_json::to_string(&remove_comment)?;

    let template = crate::template::RemoveCommentTemplate {
        json,
        blog_name: request.state().blog_name.as_str(),
    };

    Ok(
        normal_page(template.render()?)
    )
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RemoveCommentForm {
    signed_content: String
}

pub async fn handle_remove_comment(mut request: Request<ServerState>) -> tide::Result<Response> {

    use crate::schema::comments::dsl::*;
    use crate::schema::posts::dsl as p;
    let body: RemoveCommentForm = request.body_form().await?;
    let (mut raw_json, fp) = crate::utils::gpg_decrypt(body.signed_content.as_str())
        .map_err(|_| tide::Error::from_str(StatusCode::BadRequest, "verification error"))?;
    let remove: RemoveComment = simd_json::from_str(raw_json.as_mut_str())?;
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(remove.time_stamp)?;
    if duration.as_secs() > 60 * 5 {
        return Err(tide::Error::from_str(StatusCode::RequestTimeout, "request timeout"));
    }
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let (cfp, cpost_id) : (String, i32) = comments
        .select((finger_print, post_id))
        .filter(id.eq(remove.id)).first(&conn)?;
    if cfp != fp {
        Err(tide::Error::from_str(StatusCode::Unauthorized, "fingerprint does not match"))
    } else {
        diesel::delete(comments.filter(id.eq(remove.id))).execute(&conn)?;
        let ctitle : String = p::posts.select(p::title).filter(p::id.eq(cpost_id))
            .first(&conn)?;
        Ok(Redirect::new(format!("/post/{}.html", ctitle)).into())
    }
}
