use std::{collections::HashMap, sync::Mutex};
use actix_web::{
    post,
    web::{Data, Json},
};
use serde_json::{Value};
use governor::{RateLimiter, state::{NotKeyed, InMemoryState}, clock::{QuantaClock, QuantaInstant}, middleware::NoOpMiddleware};
use ttlhashmap::TtlHashMap;
use crate::config::Config;
use crate::jsonrpc::models::{JsonRpcRequest, JsonRpcResponse, JsonRpcResponseError};
use crate::jsonrpc::utils::post_jsonrpc;

#[post("/")]
async fn index(
    req: Json<JsonRpcRequest>,
    config: Data<Mutex<Config>>,
    rate_limiters: Data<Mutex<HashMap<String, RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>>>>,
    cache: Data<Mutex<TtlHashMap<String, Value>>>,
) -> Result<JsonRpcResponse, JsonRpcResponse> {
    log::debug!("| -> {:?}...", req);

    let unwrapped_config = &config.lock().unwrap();
    log::debug!("Config: {:?}", unwrapped_config);

    let unwrapped_cache = &mut cache.lock().unwrap();

    let cache_key = if req.params.is_none() { req.method.clone() } else { format!("{}{}", req.method, serde_json::to_string(&(req.params.as_ref().unwrap())).unwrap()) };
    let cache_enabled = unwrapped_config.cache.enabled;
    let mut cache_permitted: bool = false;

    if cache_enabled {
        log::debug!("Cache enabled, cache key = {:?}", cache_key);

        // Checking if cache is permitted for method

        cache_permitted = match &unwrapped_config.cache.exclude_methods {
            Some(cache_exclude_methods) => {
                match &cache_exclude_methods
                    .get(&req.method.clone())
                {
                    Some(true) => false,
                    _ => true,
                }
            }
            _ => true,
        };

        log::debug!("Cache permitted: {} for method {}", cache_permitted, req.method.clone());

        let is_in_cache = unwrapped_cache.contains_key(&cache_key);
        log::debug!("Is in cache: {is_in_cache}");

        if is_in_cache && cache_permitted {
            let cached_result = unwrapped_cache.get(&cache_key).cloned();

            let res = JsonRpcResponse {
                jsonrpc: String::from("2.0"),
                id: Some(req.id),
                result: cached_result,
                error: None,
            };
            log::debug!("| <- [cached] {:?}", res);
            return Ok(res);
        }
    }

    let client = reqwest::Client::new();
    let upstreams = &unwrapped_config.upstreams;
    for upstream in upstreams.iter() {
        log::debug!("Upstream: {:?}", upstream);

        let unwrapped_rate_limiters = rate_limiters.lock().unwrap();
        let rate_limiter = unwrapped_rate_limiters.get(&upstream.http_url.to_owned());

        if rate_limiter.is_some() {
            let failover = upstream.failover.unwrap_or(false);
            log::debug!("Rate limiter is active: {:?}, failover = {:?}", rate_limiter, failover);

            match rate_limiter.unwrap().check() {
                Ok(_) => log::debug!("Rate limit quota available"),
                Err(negative_outcome) => {
                    log::debug!("Rate limit quota exhausted: {:?}", negative_outcome);

                    if !failover {
                        log::debug!("Skipping to the next upstream because current is not failover...");
                        continue;
                    }

                    // Waiting for rate limits to unlock

                    rate_limiter.unwrap().until_ready().await;
                    log::debug!("Quota has been updated");
                } 
            }            
        } else {
            log::debug!("Rate not limited");
        }

        match post_jsonrpc(&client, &upstream.http_url.to_owned(), &req).await {
            Ok(res) => {
                if res.error.is_some() {
                    match &unwrapped_config.try_next_upstream_on_errors {
                        Some(try_next_upstream_on_errors) => {
                            match &try_next_upstream_on_errors
                                .get(&res.error.as_ref().unwrap().message)
                            {
                                Some(true) => {
                                    log::debug!("Recoverable PRC error. Trying next upstream...");
                                    continue;
                                }
                                _ => return Err(res),
                            }
                        }
                        _ => return Err(res),
                    }
                } else if res.result.is_none() {
                    log::debug!("Null result. Trying next upstream...");
                    continue;
                } else {
                    log::debug!("| <- [upstream: {}] {:?}", upstream.http_url, res);

                    if cache_enabled && cache_permitted {
                        log::debug!("Updating cache");
                        unwrapped_cache.insert(cache_key, res.result.as_ref().unwrap().clone());
                        unwrapped_cache.cleanup();
                        log::debug!("New cache size: {:?} out of {:?}", unwrapped_cache.len(), unwrapped_cache.max_nodes.unwrap_or(0));
                    }

                    return Ok(res);
                }
            }
            _ => {
                log::debug!("HTTP error. Trying next upstream...");
                continue;
            }
        }
    }

    log::debug!("No more upstreams to test");

    return Err(JsonRpcResponse {
        jsonrpc: String::from("2.0"),
        id: None,
        result: None,
        error: Some(JsonRpcResponseError {
            code: -32603,
            message: String::from("No upstream was able to process this request"),
            data: None,
        }),
    });
}