use std::{fmt::Display};
use actix_web::{
    body::BoxBody,
    http::{header::ContentType},
    HttpRequest, HttpResponse, Responder, ResponseError,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponseError {
   pub code: i16,
   pub message: String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcResponseError>,
}

impl Responder for JsonRpcResponse {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let res_body = serde_json::to_string(&self).unwrap() + "\n";

        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(res_body)
    }
}

impl ResponseError for JsonRpcResponse {
   fn status_code(&self) -> StatusCode {
       StatusCode::OK
   }

   fn error_response(&self) -> HttpResponse<BoxBody> {
       let body = serde_json::to_string(&self).unwrap() + "\n";
       let res = HttpResponse::new(self.status_code());
       res.set_body(BoxBody::new(body))
   }
}

impl Display for JsonRpcResponse {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       write!(f, "{:?}", self)
   }
}