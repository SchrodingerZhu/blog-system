use std::path::PathBuf;
use std::str::FromStr;

use anyhow::*;
use structopt::*;

use crate::api::{JsonRequest, ModelType};

#[derive(Debug, StructOpt)]
pub struct Config {
    #[structopt(short, long, help = "server address", env = "BLOG_SERVER_ADDRESS")]
    pub server_address: String,
    #[structopt(short, long, help = "server address", env = "BLOG_SERVER_PORT")]
    pub port: String,
    #[structopt(short = "k", long, help = "path to local private key", env = "BLOG_LOCAL_PRIVATE")]
    pub private_key: String,
    #[structopt(short = "r", long, help = "path to server public key", env = "BLOG_REMOTE_PUBLIC")]
    pub public_key: String,
    #[structopt(subcommand)]
    pub command: SubCommand,
}

#[derive(Debug)]
pub struct TagList(Vec<String>);

#[derive(Debug, StructOpt)]
pub enum SubCommand {
    #[structopt(name = "create-post", about = "Create a new post")]
    CreatePost {
        #[structopt(short, long, help = "Post title")]
        title: String,
        #[structopt(short, long, help = "Path to the post content file")]
        content_file: PathBuf,
        #[structopt(short = "g", long, help = "Post tags")]
        tags: TagList,
    },
    #[structopt(name = "create-page", about = "Create a new page")]
    CreatePage {
        #[structopt(short, long, help = "Page title")]
        title: String,
        #[structopt(short, long, help = "Path to the page content file")]
        content_file: PathBuf,
        #[structopt(short = "m", long, help = "Mark the page as an important one")]
        important: bool,
    },
    #[structopt(name = "update-post", about = "Update a post")]
    UpdatePost {
        #[structopt(short, long, help = "Id number of the post")]
        id: i32,
        #[structopt(short, long, help = "Path to the page content file")]
        content_file: Option<PathBuf>,
        #[structopt(short = "g", long, help = "Post tags")]
        tags: Option<TagList>,
        #[structopt(short, long, help = "Post title")]
        title: Option<String>,
    },
    #[structopt(name = "update-page", about = "Update a page")]
    UpdatePage {
        #[structopt(short, long, help = "Id number of the page")]
        id: i32,
        #[structopt(short, long, help = "Page title")]
        title: Option<String>,
        #[structopt(short, long, help = "Path to the page content file")]
        content_file: Option<PathBuf>,
        #[structopt(short = "m", long, help = "Whether the page is an important one")]
        important: Option<bool>,
    },
    #[structopt(name = "remove-post", about = "Remove a post")]
    RemovePost {
        #[structopt(short, long, help = "Id number")]
        id: i32
    },
    #[structopt(name = "remove-page", about = "Remove a page")]
    RemovePage {
        #[structopt(short, long, help = "Id number")]
        id: i32
    },
    #[structopt(name = "remove-comment", about = "Remove a comment")]
    RemoveComment {
        #[structopt(short, long, help = "Id number")]
        id: i32
    },
    #[structopt(name = "search-post", about = "Search post")]
    SearchPost {
        #[structopt(short, long, help = "Search string")]
        search: String
    },
    #[structopt(name = "list-comment", about = "List comments")]
    ListComment {
        #[structopt(short, long, help = "Post id number, set if you only want to related comments")]
        post_id: Option<i32>
    },
    #[structopt(name = "list-post", about = "List posts")]
    ListPost,
    #[structopt(name = "list-page", about = "List pages")]
    ListPage,
    #[structopt(name = "check-comment", about = "Show a specific comment")]
    CheckComment {
        #[structopt(short, long, help = "Id number")]
        id: i32,
        #[structopt(short, long, help = "Show raw content only")]
        raw: bool,
    },
    #[structopt(name = "check-post", about = "Show a specific post")]
    CheckPost {
        #[structopt(short, long, help = "Id number")]
        id: i32,
        #[structopt(short, long, help = "Show raw content only")]
        raw: bool,
    },
    #[structopt(name = "check-page", about = "Show a specific page")]
    CheckPage {
        #[structopt(short, long, help = "Id number")]
        id: i32,
        #[structopt(short, long, help = "Show raw content only")]
        raw: bool,
    },
}

impl FromStr for TagList {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut list: Vec<String> = s.split(",")
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        list.sort();
        list.dedup_by(|x, y| x == y);
        Ok(TagList(list))
    }
}

impl SubCommand {
    pub fn into_json_request(self) -> anyhow::Result<JsonRequest> {
        Ok(match self {
            SubCommand::CreatePost { title, content_file, tags } => {
                JsonRequest::PostCreate {
                    title,
                    content: std::fs::read_to_string(content_file.as_path())?,
                    tag: tags.0,
                }
            }
            SubCommand::CreatePage { title, content_file, important } => {
                JsonRequest::PageCreate {
                    title,
                    content: std::fs::read_to_string(content_file.as_path())?,
                    important,
                }
            }
            SubCommand::UpdatePost { id, content_file, tags, title } => {
                let content = if content_file.is_none() { None } else {
                    Some(std::fs::read_to_string(content_file.unwrap().as_path())?)
                };
                JsonRequest::PostUpdate {
                    id,
                    title,
                    tags: tags.map(|x| x.0),
                    content,
                }
            }
            SubCommand::UpdatePage { id, title, content_file, important } => {
                let content = if content_file.is_none() { None } else {
                    Some(std::fs::read_to_string(content_file.unwrap().as_path())?)
                };
                JsonRequest::PageUpdate {
                    id,
                    title,
                    content,
                    important,
                }
            }
            SubCommand::RemovePost { id } => {
                confirm(format!("remove post {}", id))?;
                JsonRequest::DeleteOperation { id, delete_type: ModelType::Post }
            }
            SubCommand::RemovePage { id } => {
                confirm(format!("remove page {}", id))?;
                JsonRequest::DeleteOperation { id, delete_type: ModelType::Page }
            }
            SubCommand::RemoveComment { id } => {
                confirm(format!("remove page {}", id))?;
                JsonRequest::DeleteOperation { id, delete_type: ModelType::Comment }
            }
            SubCommand::SearchPost { search } => {
                JsonRequest::PostSearch(search)
            }
            SubCommand::ListComment { post_id } => {
                match post_id {
                    Some(id) => JsonRequest::PostComments(id),
                    None => JsonRequest::ListOperation {
                        list_type: ModelType::Comment
                    }
                }
            }
            SubCommand::ListPost => {
                JsonRequest::ListOperation {
                    list_type: ModelType::Post
                }
            }
            SubCommand::ListPage => {
                JsonRequest::ListOperation {
                    list_type: ModelType::Page
                }
            }
            SubCommand::CheckComment { id, .. } => {
                JsonRequest::CheckOperation {
                    id,
                    check_type: ModelType::Comment,
                }
            }
            SubCommand::CheckPost { id, .. } => {
                JsonRequest::CheckOperation {
                    id,
                    check_type: ModelType::Post,
                }
            }
            SubCommand::CheckPage { id, .. } => {
                JsonRequest::CheckOperation {
                    id,
                    check_type: ModelType::Page,
                }
            }
        })
    }

    pub fn is_raw_content(&self) -> bool {
        match self {
            SubCommand::CheckPost { raw, .. } | SubCommand::CheckComment { raw, .. } | SubCommand::CheckPage { raw, .. }
            => *raw,
            _ => false
        }
    }
}

fn confirm<S: AsRef<str>>(msg: S) -> anyhow::Result<()> {
    println!("Are you sure to {} [Y/n]: ", msg.as_ref());
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer)?;
    if "y" == buffer.trim().to_ascii_lowercase() {
        Ok(())
    } else {
        Err(anyhow!("operation cancelled"))
    }
}