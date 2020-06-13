use std::path::PathBuf;

use crate::cli::client::SubCommand;

pub mod client;

#[derive(structopt::StructOpt)]
pub enum Command {
    #[structopt(name = "server", about = "Start the server")]
    Server {
        #[structopt(short, long, help = "Address to listen", default_value = "0.0.0.0", env = "BLOG_LISTEN_ADDRESS")]
        listen_address: String,
        #[structopt(short, long, help = "Server port", default_value = "8080", env = "BLOG_SERVER_PORT")]
        port: u16,
        #[structopt(short, long, help = "Path to the server private key", env = "BLOG_SERVER_PRIVATE")]
        server_private_key: PathBuf,
        #[structopt(short, long, help = "Whether the server private key is encrypted")]
        encrypted_server_private: bool,
        #[structopt(short, long, help = "Path to the owner public key", env = "BLOG_OWNER_PUBLIC")]
        owner_public_key: PathBuf,
        #[structopt(short = "d", long, help = "Postgres connection url", env = "BLOG_POSTGRES")]
        postgres: String,
        #[structopt(short, long, help = "Name of the blog", env = "BLOG_NAME", default_value = "Rusty Blog")]
        blog_name: String,
        #[structopt(short, long, help = "Root of the web server", env = "BLOG_WEB_ROOT", default_value = ".")]
        web_root: PathBuf,
        #[structopt(short = "u", long, help = "Server domain", env = "BLOG_SERVER_DOMAIN")]
        domain: String,
    },
    #[structopt(name = "client", about = "Use as a client")]
    Client {
        #[structopt(short, long, help = "Server address", env = "BLOG_SERVER_ADDRESS")]
        server_address: String,
        #[structopt(short, long, help = "Server address", env = "BLOG_SERVER_PORT")]
        port: String,
        #[structopt(short = "k", long, help = "Path to local private key", env = "BLOG_LOCAL_PRIVATE")]
        private_key: PathBuf,
        #[structopt(short, long, help = "Whether the private key is encrypted")]
        encrypted_private_key: bool,
        #[structopt(short = "r", long, help = "Path to server public key", env = "BLOG_REMOTE_PUBLIC")]
        public_key: PathBuf,
        #[structopt(subcommand)]
        command: SubCommand,
    },
    #[structopt(name = "key-gen", about = "Generate a key pair")]
    KeyGen {
        #[structopt(short = "P", long, help = "Path to store private key", default_value = "./private.pem")]
        private_key_path: PathBuf,
        #[structopt(short, long, help = "Path to store public key", default_value = "./public.pem")]
        public_key_path: PathBuf,
        #[structopt(short, long, help = "Encrypt the private key with password")]
        encryption: bool,
        #[structopt(short = "l", long, help = "The length of the RSA private key", default_value = "4096")]
        key_length: usize,
    },
}