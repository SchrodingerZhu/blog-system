#![feature(map_first_last)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(test)]
#[macro_use]
extern crate diesel;

use ammonia::clean;
use diesel::{PgConnection, RunQueryDsl};
use diesel::r2d2::{ConnectionManager, Pool};
use postgres::NoTls;
use pulldown_cmark::{html::push_html, Options, Parser};
use serde::*;
use snmalloc_rs::SnMalloc;
use tide::{new, Request, Status, StatusCode, Response, Endpoint};

use crate::model::NewPost;
use crate::template::PostTemplate;
use std::path::Path;
use serde::export::PhantomData;
use async_std::prelude::Future;
use crate::server::{serve_posts, serve_post, handle_comment, serve_tag};

mod template;
mod utils;
mod crypto;
mod server;
mod database;
mod model;
mod schema;
type ConnPool = Pool<ConnectionManager<PgConnection>>;
type Conn = diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>>;
#[global_allocator]
static ALLOC: SnMalloc = SnMalloc;

#[derive(Clone)]
pub struct ServerState {
    pool: ConnPool,
    blog_name: String
}

async fn start_server<A: AsRef<str>,
    B: AsRef<Path>>(
    address: A, port: u16, web_root: B,
    pool: ConnPool) -> anyhow::Result<()> {
    let mut http_server = tide::with_state(ServerState {
        pool,
        blog_name: "test blog".to_string()
    });
    http_server.at("/static").serve_dir(web_root.as_ref().join("static"))?;
    http_server.at("/posts").strip_prefix().get(serve_posts);
    http_server.at("/post").strip_prefix().get(serve_post);
    http_server.at("/tag").strip_prefix().get(serve_tag);
    http_server.at("/comment").post(handle_comment);
    http_server.middleware(tide::After(crate::server::not_found));
    http_server.listen(format!("{}:{}", address.as_ref(), port).as_str())
        .await
        .map_err(Into::into)
}


#[async_std::main]
async fn main() -> anyhow::Result<()> {
    tide::log::start();
    let manager = diesel::r2d2::ConnectionManager::<diesel::pg::PgConnection>
    ::new("postgres://schrodinger@localhost/testdb");
    let pool = diesel::r2d2::Pool::new(manager)?;
    start_server("0.0.0.0", 8080, ".", pool).await?;
    Ok(())
}