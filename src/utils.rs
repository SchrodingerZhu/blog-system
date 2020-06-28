use std::fmt::Display;
use std::io::Write;

use anyhow::*;
use prettytable::*;
use serde::Serialize;
use serde_json::*;
use tide::{Status, StatusCode};

static KEY_REGEX: &str = r#"Primary key fingerprint: (.+)|Good signature from (".+" \[ultimate\])"#;

pub async fn simdjson_body<T, State>(req: &mut tide::Request<State>) -> tide::Result<T>
    where for<'a> T: serde::de::Deserialize<'a> {
    let mut res = req.body_bytes()
        .await?;
    simd_json::from_slice(res.as_mut_slice()).status(StatusCode::UnprocessableEntity)
}

pub(crate) fn gpg_decrypt(content: &str) -> anyhow::Result<(String, String)> {
    let mut to_verify = tempfile::NamedTempFile::new()?;
    to_verify.write(content.as_bytes())?;
    to_verify.flush()?;
    let output = std::process::Command::new("gpg")
        .arg("--decrypt")
        .arg(to_verify.path())
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("invalid signature"));
    }
    let real_content: String = String::from_utf8(output.stdout)?;
    let content = String::from_utf8(output.stderr)?;
    let re = regex::Regex::new(KEY_REGEX)?;
    re.captures(content.as_str())
        .take()
        .and_then(|x| match x.get(2) {
            Some(e) => Some(e),
            None => x.get(1)
        })
        .map(|x| (real_content, x.as_str().to_string()))
        .ok_or(anyhow::anyhow!("wrong format!"))
}

pub fn to_table<T: Serialize>(s: &T) -> anyhow::Result<Table> {
    serde_json::value::to_value(s)
        .map_err(|x| x.into())
        .and_then(|x| {
            match x {
                Value::Object(e) => {
                    let mut table = Table::new();
                    for (x, y) in e {
                        table.add_row(row![bFy->x, bFb->y.to_table_item()]);
                    }
                    Ok(table)
                }
                _ => Err(
                    anyhow!("to table can only be used to struct")
                )
            }
        })
}

trait ToTableItem {
    fn to_table_item(&self) -> Box<dyn Display>;
}


impl<T: Display> ToTableItem for Option<T> {
    fn to_table_item(&self) -> Box<dyn Display> {
        match self {
            None => Box::new("N/A"),
            Some(e) => Box::new(e.to_string())
        }
    }
}

impl ToTableItem for Value {
    fn to_table_item(&self) -> Box<dyn Display> {
        match self {
            Value::String(x) => {
                let mut content = String::with_capacity(512);
                for i in x.lines().map(|x|x.trim_end()) {
                    let mut current = 0;
                    while current + 60 < i.len() {
                        content.extend(i[current..current + 60].chars());
                        if current + 60 < i.len() {
                            content.extend("\n →".chars());
                        }
                        current += 60;
                    }
                    content.extend(i[current..i.len()].chars());
                    content.push('\n');
                }
                Box::new(content)
            }
            Value::Null => Box::new("N/A"),
            Value::Bool(value) => Box::new(*value),
            Value::Number(t) => Box::new(t.to_string()),
            Value::Array(t) => {
                if t.is_empty() {
                    Box::new("")
                } else {
                    let mut table = Table::new();
                    for i in t {
                        table.add_row(row![i.to_table_item()]);
                    }
                    Box::new(table)
                }
            }
            Value::Object(t) => {
                let mut table = Table::new();
                for (x, y) in t {
                    table.add_row(row![bFr->x, bfg->y.to_table_item()]);
                }
                Box::new(table)
            }
        }
    }
}


pub fn confirm<S: AsRef<str>>(msg: S) -> anyhow::Result<()> {
    print!("Are you sure to {} [Y/n]: ", msg.as_ref());
    std::io::stdout().flush()?;
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer)?;
    if "y" == buffer.trim().to_ascii_lowercase() {
        Ok(())
    } else {
        Err(anyhow!("operation cancelled"))
    }
}

use katex::{Opts, OutputType};

pub trait RenderKaTeX {
    fn render_katex(&self) -> tide::Result<String>;
}

impl<S: AsRef<str>> RenderKaTeX for S {
    fn render_katex(&self) -> tide::Result<String> {
        let content = self.as_ref();
        let mut buffer = String::with_capacity(1024);
        let mut sub_buffer = String::new();
        let mut status = 0;
        for i in content.chars() {
            status = match (status, i) {
                (0, '$') => 1,
                (1, '$') => 2, // display
                (1, _) => 5, // simple
                (2, '$') => {
                    let res = katex::render_with_opts(sub_buffer.as_str(), Opts::builder()
                        .display_mode(false)
                        .output_type(OutputType::Html)
                        .build()
                        .unwrap())?;
                    buffer.push_str(&res);
                    sub_buffer.clear();
                    3
                },
                (3, '$') => 0,
                (5, '$') => {
                    let res = katex::render_with_opts(sub_buffer.as_str(), Opts::builder()
                        .display_mode(false)
                        .output_type(OutputType::Html)
                        .build()
                        .unwrap())?;
                    buffer.push_str(&res);
                    sub_buffer.clear();
                    0
                }
                (_, _) => status,
            };
            if i != '$' {
                if status != 2 && status != 5 {
                    buffer.push(i);
                } else {
                    sub_buffer.push(i);
                }
            }
        }
        Ok(buffer)
    }
}