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
    #[structopt(name = "create-post", about = "create a new post")]
    CreatePost {
        #[structopt(short, long, help = "post title")]
        title: String,
        #[structopt(short, long, help = "path to the post content file")]
        content_file: PathBuf,
        #[structopt(short = "g", long, help = "post tags")]
        tags: TagList,
    },
    #[structopt(name = "create-page", about = "create a new page")]
    CreatePage {
        #[structopt(short, long, help = "page title")]
        title: String,
        #[structopt(short, long, help = "path to the page content file")]
        content_file: PathBuf,
        #[structopt(short = "m", long, help = "mark the page as an important one")]
        important: bool,
    },
    #[structopt(name = "update-post", about = "update a post")]
    UpdatePost {
        #[structopt(short, long, help = "id number of the post")]
        id: i32,
        #[structopt(short, long, help = "path to the page content file")]
        content_file: Option<PathBuf>,
        #[structopt(short = "g", long, help = "post tags")]
        tags: Option<TagList>,
        #[structopt(short, long, help = "post title")]
        title: Option<String>,
    },
    #[structopt(name = "update-page", about = "update a page")]
    UpdatePage {
        #[structopt(short, long, help = "id number of the page")]
        id: i32,
        #[structopt(short, long, help = "page title")]
        title: Option<String>,
        #[structopt(short, long, help = "path to the page content file")]
        content_file: Option<PathBuf>,
        #[structopt(short = "m", long, help = "whether the page is an important one")]
        important: Option<bool>,
    },
    #[structopt(name = "remove-post", about = "remove a post")]
    RemovePost {
        #[structopt(short, long, help = "id number")]
        id: i32
    },
    #[structopt(name = "remove-page", about = "remove a page")]
    RemovePage {
        #[structopt(short, long, help = "id number")]
        id: i32
    },
    #[structopt(name = "remove-comment", about = "remove a comment")]
    RemoveComment {
        #[structopt(short, long, help = "id number")]
        id: i32
    },
    #[structopt(name = "search-post", about = "search post")]
    SearchPost {
        #[structopt(short, long, help = "search string")]
        search: String
    },
    #[structopt(name = "list-comment", about = "list comments")]
    ListComment {
        #[structopt(short, long, help = "post id number, set if you only want to related comments")]
        post_id: Option<i32>
    },
    #[structopt(name = "list-post", about = "list posts")]
    ListPost,
    #[structopt(name = "list-page", about = "list pages")]
    ListPage,
    #[structopt(name = "check-comment", about = "show a specific comment")]
    CheckComment {
        #[structopt(short, long, help = "id number")]
        id: i32
    },
    #[structopt(name = "check-post", about = "show a specific post")]
    CheckPost {
        #[structopt(short, long, help = "id number")]
        id: i32
    },
    #[structopt(name = "check-page", about = "show a specific page")]
    CheckPage {
        #[structopt(short, long, help = "id number")]
        id: i32
    },
}

impl FromStr for TagList {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut list: Vec<String> = s.split(",")
            .filter(|x| !x.is_empty())
            .map(|x| x.to_string())
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
                    important
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
            SubCommand::CheckComment { id } => {
                JsonRequest::CheckOperation {
                    id,
                    check_type: ModelType::Comment
                }
            }
            SubCommand::CheckPost { id } => {
                JsonRequest::CheckOperation {
                    id,
                    check_type: ModelType::Post
                }
            }
            SubCommand::CheckPage { id } => {
                JsonRequest::CheckOperation {
                    id,
                    check_type: ModelType::Page
                }
            }
        })
    }
}

fn confirm<S : AsRef<str>>(msg: S) -> anyhow::Result<()> {
    println!("Are you sure to {} [Y/n]: ", msg.as_ref());
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer)?;
    if "y" == buffer.trim().to_ascii_lowercase() {
        Ok(())
    } else {
        Err(anyhow!("operation cancelled"))
    }
}