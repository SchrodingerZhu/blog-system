use std::io::Write;

use tide::{Status, StatusCode};
use std::fmt::Display;
use serde::Serialize;
use anyhow::*;
use serde_json::*;
use prettytable::*;

static KEY_REGEX: &str = r#"Primary key fingerprint: (.+)|Good signature from (".+" \[ultimate\])"#;

pub async fn simdjson_body<T, State>(req: &mut tide::Request<State>) -> tide::Result<T>
    where for<'a> T: serde::de::Deserialize<'a> {
    let mut res = req.body_bytes()
        .await?;
    Ok(simd_json::from_slice(res.as_mut_slice()).status(StatusCode::UnprocessableEntity)?)
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
            Value::String(x) => Box::new(x.to_string()),
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


