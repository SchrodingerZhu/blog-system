use std::io::Write;

use tide::{Status, StatusCode};

static KEY_REGEX: &str = r#"Primary key fingerprint: (.+)|Good signature from (".+" \[ultimate\])"#;

async fn simdjson_body<T, State>(req: &mut tide::Request<State>) -> tide::Result<T>
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




