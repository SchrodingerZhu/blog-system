use tide::{StatusCode, Status};

async fn simdjson_body<T, State>(req: &mut tide::Request<State>) -> tide::Result<T>
    where for<'a> T : serde::de::Deserialize<'a> {
    let mut res = req.body_bytes()
        .await?;
    Ok(simd_json::from_slice(res.as_mut_slice()).status(StatusCode::UnprocessableEntity)?)
}





