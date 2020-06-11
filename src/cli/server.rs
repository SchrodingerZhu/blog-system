use structopt::*;
use std::path::PathBuf;

#[derive(Debug, StructOpt)]
pub struct Config {
    #[structopt(short, long, help="address to listen", default_value="0.0.0.0", env="BLOG_LISTEN_ADDRESS")]
    pub listen_address: String,
    #[structopt(short, long, help="server port", default_value="8080", env="BLOG_SERVER_PORT")]
    pub port: u16,
    #[structopt(short, long, help="path to the server private key", env="BLOG_SERVER_PRIVATE")]
    pub server_private_key: PathBuf,
    #[structopt(short, long, help="whether the server private key is encrypted")]
    pub encrypted_server_private: bool,
    #[structopt(short, long, help="path to the owner public key", env="BLOG_OWNER_PUBLIC")]
    pub owner_public_key: PathBuf,
    #[structopt(short="d", long, help="postgres connection url", env="BLOG_POSTGRES")]
    pub postgres: String,
    #[structopt(short, long, help="name of the blog", env="BLOG_NAME", default_value="Rusty Blog")]
    pub blog_name: String,
    #[structopt(short, long, help="root of the web server", env="BLOG_WEB_ROOT", default_value=".")]
    pub web_root: PathBuf
}