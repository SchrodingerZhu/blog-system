#![feature(map_first_last)]
#![feature(unboxed_closures)]
#![feature(core_intrinsics)]
#[macro_use]
extern crate diesel;

use std::path::Path;

use anyhow::*;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use structopt::StructOpt;
use std::sync::Arc;
use xactor::{Actor, Addr};

use crate::api::JsonResponse;
use crate::crypto::{Packet, StampKeeper};
use crate::server::*;

mod template;
mod utils;
mod crypto;
mod server;
mod model;
mod schema;
mod api;
mod cli;

type ConnPool = Pool<ConnectionManager<PgConnection>>;
type Conn = diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>>;

#[cfg(feature = "use-snmalloc")]
#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[cfg(feature = "use-mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

const PAGE_LIMIT : i64 = 5;

#[derive(Clone)]
pub struct ServerState {
    pool: ConnPool,
    blog_name: String,
    stamp_keeper: Addr<StampKeeper>,
    key_pair: Arc<KeyPair>,
    domain: String,
}

pub struct KeyPair {
    server_private: botan::Privkey,
    owner_public: botan::Pubkey,
}


unsafe impl Send for KeyPair {}

unsafe impl Sync for KeyPair {}


async fn start_server<A: AsRef<str>,
    B: AsRef<Path>>(
    address: A,
    port: u16,
    web_root: B,
    pool: ConnPool,
    stamp_keeper: Addr<StampKeeper>,
    server_private: botan::Privkey,
    owner_public: botan::Pubkey,
    blog_name: String,
    domain: String,
) -> anyhow::Result<()> {
    let mut http_server = tide::with_state(ServerState {
        pool,
        blog_name,
        stamp_keeper,
        key_pair: Arc::new(KeyPair { server_private, owner_public }),
        domain,
    });
    http_server.at("/static").serve_dir(web_root.as_ref().join("static"))?;
    http_server.at("/posts").strip_prefix().get(serve_posts);
    http_server.at("/post").strip_prefix().get(serve_post);
    http_server.at("/page").strip_prefix().get(serve_page);
    http_server.at("/tag").strip_prefix().get(serve_tag);
    http_server.at("/tags").get(serve_tags);
    http_server.at("/lucky").get(serve_lucky);
    http_server.at("/search").post(handle_search);
    http_server.at("/raw/comment").strip_prefix().get(serve_comment_raw);
    http_server.at("/raw/post").strip_prefix().get(serve_post_raw);
    http_server.at("/raw/page").strip_prefix().get(serve_page_raw);
    http_server.at("/comment/submit").post(handle_comment);
    http_server.at("/comment/remove").strip_prefix().get(remove_comment);
    http_server.at("/comment/remove").post(handle_remove_comment);
    http_server.at("/rss.xml").get(handle_rss);
    http_server.at("/atom.xml").get(handle_atom);
    http_server.at("/sitemap.xml").get(handle_sitemap);
    http_server.at("/api").post(handle_api);
    http_server.at("/").get(index);
    http_server.with(tide::utils::After(error_handle));
    http_server.with(tide_compress::CompressMiddleware::new());
    http_server.listen(format!("{}:{}", address.as_ref(), port).as_str())
        .await
        .map_err(Into::into)
}


#[async_std::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()
        .map(|x| log::info!("dot env initialized with {:?}", x))
        .unwrap_or_else(|x| log::error!("dot env not initialized {}", x));
    let config: crate::cli::Command =
        crate::cli::Command::from_args();
    match config {
        crate::cli::Command::Server {
            listen_address,
            port,
            server_private_key,
            encrypted_server_private,
            owner_public_key,
            postgres,
            blog_name,
            web_root,
            domain
        } => {
            tide::log::start();
            let manager =
                diesel::r2d2::ConnectionManager::<diesel::pg::PgConnection>
                ::new(postgres);
            let pool =
                diesel::r2d2::Pool::new(manager)?;
            let private_key_file =
                std::fs::read_to_string(server_private_key.as_path())?;
            let private_key = if encrypted_server_private {
                let password = rpassword::prompt_password_stdout("please input password: ")?;
                botan::Privkey::load_encrypted_pem(private_key_file.as_str(), password.as_str())
                    .map_err(|x| anyhow!("{:?}", x))?
            } else {
                botan::Privkey::load_pem(private_key_file.as_str())
                    .map_err(|x| anyhow!("{:?}", x))?
            };
            let public_key_file =
                std::fs::read_to_string(owner_public_key.as_path())?;
            let public_key =
                botan::Pubkey::load_pem(public_key_file.as_str())
                    .map_err(|x| anyhow!("{:?}", x))?;
            let stamp_keeper =
                StampKeeper::start_default().
                    await?;
            start_server(listen_address,
                         port,
                         web_root,
                         pool,
                         stamp_keeper,
                         private_key,
                         public_key, blog_name, domain).await
        }
        crate::cli::Command::Client {
            server_address,
            port,
            private_key,
            encrypted_private_key,
            public_key,
            command
        } => {
            pretty_env_logger::try_init_timed_custom_env("BLOG_CLIENT_LOG")?;
            let is_raw = command.is_raw_content();
            let request = command.into_json_request()?;
            let private_key_file =
                std::fs::read_to_string(private_key.as_path())?;
            let private_key = if encrypted_private_key {
                let password = rpassword::prompt_password_stdout("please input password: ")?;
                botan::Privkey::load_encrypted_pem(private_key_file.as_str(), password.as_str())
                    .map_err(|x| anyhow!("{:?}", x))?
            } else {
                botan::Privkey::load_pem(private_key_file.as_str())
                    .map_err(|x| anyhow!("{:?}", x))?
            };
            let public_key_file =
                std::fs::read_to_string(public_key.as_path())?;
            let public_key =
                botan::Pubkey::load_pem(public_key_file.as_str())
                    .map_err(|x| anyhow!("{:?}", x))?;
            let key_pair = KeyPair {
                server_private: private_key,
                owner_public: public_key,
            };
            let packet = Packet::from_json_request(request, &key_pair).await?;
            let response = surf::Client::new()
                .post(format!("{}:{}/api", server_address, port))
                .body(simd_json::to_string(&packet)?)
                .await
                .map_err(|x| anyhow!("{}", x))?
                .body_string()
                .await
                .map_err(|x| anyhow!("{}", x))
                .and_then(|mut x| simd_json::from_str::<Packet>(x.as_mut_str())
                    .map_err(|x| x.into()))?;
            let real_response: JsonResponse = response
                .to_json_request(&key_pair, None)
                .await?;
            if !is_raw {
                crate::utils::to_table(&real_response)?.printstd();
            } else {
                let content = real_response.get_raw_content()?;
                println!("{}", content);
            }
            Ok(())
        }
        crate::cli::Command::KeyGen {
            private_key_path,
            public_key_path,
            encryption,
            key_length
        } => {
            utils::confirm(format!("generate the key pair ({:?},{:?}, encryption: {})",
                                   private_key_path, public_key_path, encryption))?;
            let random = botan::RandomNumberGenerator::new_system()
                .map_err(|x| anyhow!("{:?}", x))?;
            let private_key = botan::Privkey::create(
                "RSA",
                key_length.to_string().as_str(),
                &random,
            ).map_err(|x| anyhow!("{:?}", x))?;
            let private_key_pem = if encryption {
                let password = rpassword::prompt_password_stdout("please input password: ")?;
                private_key.pem_encode_encrypted(password.as_str(), &random)
                    .map_err(|x| anyhow!("{:?}", x))?
            } else {
                private_key.pem_encode()
                    .map_err(|x| anyhow!("{:?}", x))?
            };
            let public_key_pem = private_key.pubkey()
                .and_then(|x| x.pem_encode())
                .map_err(|x| anyhow!("{:?}", x))?;
            std::fs::write(private_key_path, private_key_pem)?;
            std::fs::write(public_key_path, public_key_pem)?;
            Ok(())
        }
    }
}