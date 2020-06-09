use crate::{ConnPool, ServerState};
use tide::{StatusCode, Status, Request, Response};
use diesel::prelude::*;
use crate::model::{Post, Comment, NewComment};
use crate::template::PostsTemplate;
use askama::Template;
use async_std::prelude::Future;
use std::pin::Pin;
use crate::schema::comments::dsl::comments;
use std::io::Write;

static EMAIL_REGEX : &str = "^[A-Za-z0-9._%-]+@[A-Za-z0-9.-]+[.][A-Za-z]+$";

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag="request_type", content="request_body")]
pub enum JsonRequest {
    ListPosts
}

pub struct PostsEndpoint {
    pool: ConnPool,
    blog_name: String
}

pub async fn serve_posts(request: Request<ServerState>) -> tide::Result<tide::Response> {
    use crate::schema::posts::dsl::*;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let all_posts = posts
        .load::<Post>(&conn)
        .status(StatusCode::InternalServerError)?;

    let posts_template = PostsTemplate {
        blog_name: request.state().blog_name.as_str(),
        posts: all_posts
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
pub fn normal_page<A : Into<tide::Body>>(page: A) -> tide::Response {
    let mut responce = tide::Response::new(StatusCode::Ok);
    responce.set_body(page.into());
    responce.set_content_type(http_types::mime::HTML);
    responce
}

pub async fn not_found(result: tide::Result<Response>) -> tide::Result<tide::Response> {
    let mut responce = result.unwrap_or_else(|e| Response::new(e.status()));
    if let StatusCode::NotFound =  responce.status() {
        let page = crate::template::NotFound;
        responce.set_body(page.render()?);
        responce.set_content_type(http_types::mime::HTML);
    }
    Ok(
        responce
    )
}

pub async fn serve_post(request: Request<ServerState>) -> tide::Result<Response> {
    use crate::schema::posts::dsl as p;
    use crate::schema::comments::dsl as c;
    let path = request.url().path().trim_start_matches("/");
    if path.contains('/') || !path.ends_with(".html") {
        return Ok(Response::new(StatusCode::NotFound))
    }
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let name = path.trim_end_matches(".html");
    let mut post : Vec<Post> = p::posts.filter(p::title.eq(name))
        .distinct().load::<Post>(&conn)?;
    if post.is_empty() {
        Ok(Response::new(StatusCode::NotFound))
    } else {
        let post = post.pop().unwrap();
        let all_comments = Comment::belonging_to(&post)
            .load::<Comment>(&conn)?;
        let template = crate::template::PostTemplate {
            post,
            comments: all_comments
        };
        let page = template.render()?;
        Ok(normal_page(page))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CommentForm {
    post_id: i32,
    comment_nickname: String,
    comment_email: String,
    comment_content: String,
}


pub async fn handle_comment(mut request: Request<ServerState>) -> tide::Result<String> {
    use crate::schema::posts::dsl as p;
    use crate::schema::comments::dsl as c;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let form : CommentForm = request.body_form()
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
    let mut to_verify = tempfile::NamedTempFile::new()?;
    to_verify.write(form.comment_content.as_bytes())?;
    to_verify.flush()?;
    let output = std::process::Command::new("gpg")
        .arg("--verify")
        .arg(to_verify.path())
        .output()?;
    if !output.status.success() {
        return Err(tide::Error::from_str(StatusCode::BadRequest, "invalid signature"));
    }
    let mut content_buffer = Vec::new();
    let mut lines = form.comment_content.lines();
    lines.next();
    lines.next();
    lines.next();
    for i in lines  {
        if i.starts_with("-----BEGIN PGP SIGNATURE-----") {
            break;
        }
        content_buffer.push(i);
    }
    let real_content : String = content_buffer.join("\n");
    let content = String::from_utf8(output.stderr)?;
    let mut lines = content.lines();
    lines.next();
    if let Some(content) = lines.next() {
        let mut words = content.split_whitespace();
        words.next();
        words.next();
        words.next();
        if let Some(finger_print) = words.next() {
            let comment = NewComment {
                post_id: form.post_id,
                nickname: form.comment_nickname.as_str(),
                email: form.comment_email.as_str(),
                content: real_content.as_str(),
                signature: form.comment_content.as_str(),
                finger_print
            };
            diesel::insert_into(c::comments)
                .values(&comment)
                .execute(&conn)?;
            return Ok("success".to_string())
        }
    }
    Err(tide::Error::from_str(StatusCode::BadRequest, "unable to parse request"))
}