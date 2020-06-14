use std::str::FromStr;

use askama::Template;
use chrono::FixedOffset;
use diesel::prelude::*;
use tide::{Redirect, Request, Response, Status, StatusCode};

use crate::api::JsonRequest;
use crate::crypto::Packet;
use crate::model::{Comment, NewComment, Page, Post, POST_COLUMNS};
use crate::ServerState;
use crate::template::{PostsTemplate, Tag, TagTemplate};

static EMAIL_REGEX: &str = "^[A-Za-z0-9._%-]+@[A-Za-z0-9.-]+[.][A-Za-z]+$";

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
    use crate::schema::lower;
    let path = request.url().path().trim_start_matches("/");
    if path.contains('/') || !path.ends_with(".html") {
        return Ok(Response::new(StatusCode::NotFound));
    }
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let name = path.trim_end_matches(".html");
    let name = percent_encoding::percent_decode_str(name).decode_utf8()?
        .replace("-", " ");
    let post: Post = p::posts
        .select(POST_COLUMNS)
        .filter(lower(p::title).eq(name.to_ascii_lowercase()))
        .first::<Post>(&conn)
        .status(StatusCode::NotFound)?;
    let all_comments = Comment::belonging_to(&post)
        .load::<Comment>(&conn)?;
    let template = crate::template::PostTemplate {
        post,
        comments: all_comments,
    };
    let page = template.render()?;
    Ok(normal_page(page))
}

pub async fn serve_page(request: Request<ServerState>) -> tide::Result<Response> {
    use crate::schema::pages::dsl as p;
    use crate::schema::lower;
    let path = request.url().path().trim_start_matches("/");
    if path.contains('/') || !path.ends_with(".html") {
        return Ok(Response::new(StatusCode::NotFound));
    }
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let name = path.trim_end_matches(".html");
    let name = percent_encoding::percent_decode_str(name).decode_utf8()?
        .replace("-", " ");
    let page = p::pages
        .filter(lower(p::title).eq(name.to_ascii_lowercase()))
        .first::<Page>(&conn)
        .status(StatusCode::NotFound)?;
    let template = crate::template::PageTemplate {
        blog_name: request.state().blog_name.as_str(),
        page: &page,
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
        .map_err(|e| tide::Error::from_str(StatusCode::BadRequest, e))?;

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
    let title: String = p::posts.select(p::title).filter(p::id.eq(form.post_id))
        .first(&conn)
        .status(StatusCode::NotFound)?;
    Ok(Redirect::new(format!("/post/{}.html", title.replace(" ", "-")
        .to_ascii_lowercase())).into())
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

    let old_tag = percent_encoding::percent_decode_str(url_split[0])
        .decode_utf8()?;
    let real_tag = old_tag.replace("-", " ");
    let all_posts = posts
        .select(POST_COLUMNS)
        .filter(tags.contains(vec![real_tag.as_str()]))
        .limit(20)
        .offset(page_number * 20)
        .load::<Post>(&conn)
        .status(StatusCode::InternalServerError)?;

    let tag_template = TagTemplate {
        blog_name: request.state().blog_name.as_str(),
        name: real_tag.as_ref(),
        posts: all_posts,
        page_number,
        translated_name: old_tag.as_ref()
    };
    let page = tag_template.render()?;
    Ok(
        normal_page(page)
    )
}

pub async fn serve_comment_raw(request: Request<ServerState>) -> tide::Result<String> {
    use crate::schema::comments::dsl::*;
    let cid: i32 = request.url().path().trim_start_matches("/").parse()
        .status(StatusCode::BadRequest)?;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let target: String = comments.select(signature)
        .filter(id.eq(cid))
        .first(&conn)
        .status(StatusCode::NotFound)?;
    Ok(target)
}

pub async fn serve_post_raw(request: Request<ServerState>) -> tide::Result<String> {
    use crate::schema::posts::dsl::*;
    let cid: i32 = request.url().path().trim_start_matches("/").parse()
        .status(StatusCode::BadRequest)?;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let target: String = posts.select(content)
        .filter(id.eq(cid))
        .first(&conn)
        .status(StatusCode::NotFound)?;

    Ok(target)
}

pub async fn serve_page_raw(request: Request<ServerState>) -> tide::Result<String> {
    use crate::schema::pages::dsl::*;
    let cid: i32 = request.url().path().trim_start_matches("/").parse()
        .status(StatusCode::BadRequest)?;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let target: String = pages.select(content)
        .filter(id.eq(cid))
        .first(&conn)
        .status(StatusCode::NotFound)?;

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
    let map = all_tags.into_iter()
        .fold(hashbrown::HashMap::new(), |mut acc, next| {
            *acc.entry(next.to_ascii_lowercase()).or_insert(0) += 1;
            acc
        });
    let mut tag_vector = Vec::new();
    for (tag, count) in map.into_iter() {
        tag_vector.push(Tag { tag, count });
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
                                             Some(form.page_number))?;
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
    let (cfp, cpost_id): (String, i32) = comments
        .select((finger_print, post_id))
        .filter(id.eq(remove.id)).first(&conn)?;
    if cfp != fp {
        Err(tide::Error::from_str(StatusCode::Unauthorized, "fingerprint does not match"))
    } else {
        diesel::delete(comments.filter(id.eq(remove.id))).execute(&conn)?;
        let ctitle: String = p::posts.select(p::title).filter(p::id.eq(cpost_id))
            .first(&conn)?;
        Ok(Redirect::new(format!("/post/{}.html", ctitle.replace(" ", "-")
            .to_ascii_lowercase())).into())
    }
}

pub async fn index(request: Request<ServerState>) -> tide::Result<Response> {
    use crate::schema::pages::dsl::*;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let all_pages = pages.load::<Page>(&conn)?;
    let important_pages = pages.select((title, id))
        .filter(important)
        .load::<(String, i32)>(&conn)?
        .into_iter()
        .map(|(x, y)| {
            let z = x.to_ascii_lowercase().replace(" ", "-");
            (x, y, z)
        })
        .collect::<Vec<_>>();
    let index = crate::template::IndexTemplate {
        blog_name: request.state().blog_name.as_str(),
        pages: &all_pages,
        important_pages: &important_pages,
    };
    Ok(
        normal_page(index.render()?)
    )
}

pub async fn handle_api(mut request: Request<ServerState>) -> tide::Result<Response> {
    let packet: Packet = crate::utils::simdjson_body(&mut request).await?;
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    let mut addr = request.state().stamp_keeper.clone();
    let key_pair = &request.state().key_pair;
    let request: JsonRequest = packet.to_json_request(key_pair,
                                                      Some(&mut addr),
    ).await.map_err(|_| tide::Error::from_str(StatusCode::BadRequest, "failed to decode request"))?;
    let mut response = Response::new(StatusCode::Ok);
    let response_content = request.handle(conn)
        .await;
    let response_packet = Packet::from_json_request_tide(response_content,
                                                         key_pair,
    ).await?;
    let response_json = simd_json::to_string(&response_packet)?;
    response.set_content_type(http_types::mime::JSON);
    response.set_body(response_json);
    Ok(response)
}

pub async fn handle_rss(request: Request<ServerState>) -> tide::Result<Response> {
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    use crate::schema::posts::dsl::*;
    let all_posts: Vec<Post> = posts
        .select(POST_COLUMNS)
        .load::<Post>(&conn)
        .status(StatusCode::InternalServerError)?;
    let items: Vec<rss::Item> = all_posts.into_iter()
        .map(|x| rss::ItemBuilder::default()
            .title(Some(x.title.clone()))
            .link(Some(format!("{}/post/{}.html", request.state().domain,
                               x.title.to_ascii_lowercase().replace(" ", "-"))))
            .pub_date(Some(x.date.to_string()))
            .content(Some({
                let parser = pulldown_cmark::Parser::new(x.content.as_str());
                let mut rendered = String::with_capacity(512);
                pulldown_cmark::html::push_html(&mut rendered, parser);
                rendered
            }))
            .build())
        .filter_map(|x| x.ok())
        .collect();
    let channel: rss::Channel = rss::ChannelBuilder::default()
        .title(request.state().blog_name.as_str())
        .link(request.state().domain.as_str())
        .description("blog")
        .items(items)
        .build()
        .map_err(|_| tide::Error::from_str(StatusCode::Ok, "RSS build failed"))?;
    let mime = http_types::mime::Mime::from_str("application/rss+xml")?;
    let mut response = Response::new(StatusCode::Ok);
    response.set_content_type(mime);
    response.set_body(channel.to_string());
    Ok(
        response
    )
}


pub async fn handle_atom(request: Request<ServerState>) -> tide::Result<Response> {
    let conn = request
        .state().pool.get().status(StatusCode::InternalServerError)?;
    use crate::schema::posts::dsl::*;
    let all_posts: Vec<Post> = posts
        .select(POST_COLUMNS)
        .load::<Post>(&conn)
        .status(StatusCode::InternalServerError)?;
    let entries: Vec<atom_syndication::Entry> = all_posts.into_iter()
        .map(|x| atom_syndication::ContentBuilder::default()
            .value(Some({
                let parser = pulldown_cmark::Parser::new(x.content.as_str());
                let mut rendered = String::with_capacity(512);
                pulldown_cmark::html::push_html(&mut rendered, parser);
                rendered
            }))
            .content_type(Some("text/html".to_string()))
            .src(Some(format!("{}/raw/post/{}", request.state().domain, x.id)))
            .build()
            .and_then(|the_content| atom_syndication::EntryBuilder::default()
                .title(x.title.as_str())
                .updated({
                    chrono::DateTime::<FixedOffset>::from_utc(x.date.clone(),
                                                              chrono::FixedOffset::east(0))
                })
                .links(vec![{
                    let mut link = atom_syndication::Link::default();
                    link.set_href(format!("{}/post/{}.html", request.state().domain, x.title
                        .to_ascii_lowercase().replace(" ", "-")));
                    link.set_title(x.title.clone());
                    link
                }])
                .content(the_content)
                .build()))
        .filter_map(|x| x.ok())
        .collect();
    let channel: atom_syndication::Feed = atom_syndication::FeedBuilder::default()
        .title(request.state().blog_name.as_str())
        .links(vec![{
            let mut link = atom_syndication::Link::default();
            link.set_href(request.state().domain.clone());
            link.set_title(request.state().blog_name.clone());
            link
        }])
        .entries(entries)
        .icon(Some(format!("{}/static/img/ico.png", request.state().domain)))
        .build()
        .map_err(|_| tide::Error::from_str(StatusCode::Ok, "ATOM build failed"))?;
    let mime = http_types::mime::Mime::from_str("application/atom+xml")?;
    let mut response = Response::new(StatusCode::Ok);
    response.set_content_type(mime);
    response.set_body(channel.to_string());
    Ok(
        response
    )
}