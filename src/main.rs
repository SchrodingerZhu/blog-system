#![feature(map_first_last)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(test)]
#[macro_use]
extern crate diesel;

use std::path::Path;

use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use snmalloc_rs::SnMalloc;

use crate::server::*;
use xactor::Addr;
use crate::crypto::StampKeeper;
use async_std::sync::Arc;

mod template;
mod utils;
mod crypto;
mod server;
mod model;
mod schema;
mod api;

type ConnPool = Pool<ConnectionManager<PgConnection>>;
type Conn = diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>>;

#[global_allocator]
static ALLOC: SnMalloc = SnMalloc;

#[derive(Clone)]
pub struct ServerState {
    pool: ConnPool,
    blog_name: String,
    stamp_keeper: Addr<StampKeeper>,
    server_private: Arc<botan::Privkey>,
    owner_public: Arc<botan::Pubkey>
}

unsafe impl Send for ServerState {}
unsafe impl Sync for ServerState {}

async fn start_server<A: AsRef<str>,
    B: AsRef<Path>>(
        address: A,
        port: u16,
        web_root: B,
        pool: ConnPool,
        stamp_keeper: Addr<StampKeeper>,
        server_private: botan::Privkey,
        owner_public: botan::Pubkey
    ) -> anyhow::Result<()> {
    let mut http_server = tide::with_state(ServerState {
        pool,
        blog_name: "test blog".to_string(),
        stamp_keeper,
        server_private: Arc::new(server_private),
        owner_public: Arc::new(owner_public)
    });
    http_server.at("/static").serve_dir(web_root.as_ref().join("static"))?;
    http_server.at("/posts").strip_prefix().get(serve_posts);
    http_server.at("/post").strip_prefix().get(serve_post);
    http_server.at("/page").strip_prefix().get(serve_page);
    http_server.at("/tag").strip_prefix().get(serve_tag);
    http_server.at("/tags").get(serve_tags);
    http_server.at("/search").post(handle_search);
    http_server.at("/raw/comment").strip_prefix().get(serve_comment_raw);
    http_server.at("/raw/post").strip_prefix().get(serve_post_raw);
    http_server.at("/raw/page").strip_prefix().get(serve_page_raw);
    http_server.at("/comment/submit").post(handle_comment);
    http_server.at("/comment/remove").strip_prefix().get(remove_comment);
    http_server.at("/comment/remove").post(handle_remove_comment);
    http_server.at("/").get(index);
    http_server.middleware(tide::After(error_handle));
    http_server.listen(format!("{}:{}", address.as_ref(), port).as_str())
        .await
        .map_err(Into::into)
}


#[async_std::main]
async fn main() -> anyhow::Result<()> {
    tide::log::start();
    let manager =
        diesel::r2d2::ConnectionManager::<diesel::pg::PgConnection>
        ::new("postgres://schrodinger@localhost/testdb");
    let pool = diesel::r2d2::Pool::new(manager)?;
    //start_server("0.0.0.0", 8080, ".", pool).await?;
    Ok(())
}