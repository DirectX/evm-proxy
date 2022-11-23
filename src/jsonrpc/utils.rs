use actix_web::{
    web::Json,
};
use crate::jsonrpc::models::{JsonRpcRequest, JsonRpcResponse};

pub async fn post_jsonrpc(
    client: &reqwest::Client,
    url: &str,
    req: &Json<JsonRpcRequest>,
) -> Result<JsonRpcResponse, reqwest::Error> {
    let res = match client.post(url).json(&req).send().await {
        Ok(res) => res,
        Err(err) => return Err(err),
    };

    let json = match res.json::<JsonRpcResponse>().await {
        Ok(json) => json,
        Err(err) => return Err(err),
    };

    Ok(json)
}