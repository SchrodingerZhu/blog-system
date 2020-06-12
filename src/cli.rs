use std::path::PathBuf;
use crate::cli::client::SubCommand;

pub mod client;

#[derive(structopt::StructOpt)]
pub enum Command {
    #[structopt(name="server", about="Start the server")]
    Server {
        #[structopt(short, long, help="address to listen", default_value="0.0.0.0", env="BLOG_LISTEN_ADDRESS")]
        listen_address: String,
        #[structopt(short, long, help="server port", default_value="8080", env="BLOG_SERVER_PORT")]
        port: u16,
        #[structopt(short, long, help="path to the server private key", env="BLOG_SERVER_PRIVATE")]
        server_private_key: PathBuf,
        #[structopt(short, long, help="whether the server private key is encrypted")]
        encrypted_server_private: bool,
        #[structopt(short, long, help="path to the owner public key", env="BLOG_OWNER_PUBLIC")]
        owner_public_key: PathBuf,
        #[structopt(short="d", long, help="postgres connection url", env="BLOG_POSTGRES")]
        postgres: String,
        #[structopt(short, long, help="name of the blog", env="BLOG_NAME", default_value="Rusty Blog")]
        blog_name: String,
        #[structopt(short, long, help="root of the web server", env="BLOG_WEB_ROOT", default_value=".")]
        web_root: PathBuf,
        #[structopt(short="u", long, help="server domain", env="BLOG_SERVER_DOMAIN")]
        domain: String
    },
    #[structopt(name="client", about="Use as a client")]
    Client {
        #[structopt(short, long, help = "server address", env = "BLOG_SERVER_ADDRESS")]
        server_address: String,
        #[structopt(short, long, help = "server address", env = "BLOG_SERVER_PORT")]
        port: String,
        #[structopt(short = "k", long, help = "path to local private key", env = "BLOG_LOCAL_PRIVATE")]
        private_key: PathBuf,
        #[structopt(short, long, help="whether the private key is encrypted")]
        encrypted_private_key: bool,
        #[structopt(short = "r", long, help = "path to server public key", env = "BLOG_REMOTE_PUBLIC")]
        public_key: PathBuf,
        #[structopt(subcommand)]
        command: SubCommand,
    }
}