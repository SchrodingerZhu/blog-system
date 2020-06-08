use ammonia::clean;
use pulldown_cmark::{Parser, Options, html::push_html};
use serde::*;
use tide::{Request, Status, StatusCode};

mod template;
mod utils;
mod crypto;
mod server;


#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let mut app = tide::new();
    app.at("/").get(|_| async { Ok("Hello, world!") });
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}