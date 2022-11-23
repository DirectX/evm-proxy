use std::{collections::HashMap, sync::Mutex, time::Duration, num::NonZeroU32};
use actix_web::{
    web::Data,
    App, HttpServer,
};
use serde_json::{Value};
use governor::{Quota, RateLimiter, state::{NotKeyed, InMemoryState}, clock::{QuantaClock, QuantaInstant}, middleware::NoOpMiddleware};
use ttlhashmap::{TtlHashMap, AutoClean};
use crate::config::Config;
use crate::jsonrpc::handlers::index;

pub async fn run_server(config: Config) -> std::io::Result<()> {
    let host = config.server.host.to_owned();
    let port = config.server.port;
    let mut rate_limiters: HashMap<String, RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>> = HashMap::new();

    for upstream in &config.upstreams {
        log::debug!("Upstream: {:?}", upstream);
        
        match &upstream.rate_limit {
            Some(rate_limit) => {
                
                // Parsing rate limits

                let parts = rate_limit.split("/").map(|s| s.trim()).collect::<Vec<&str>>();
                if parts.len() == 2 {
                    let count: u32 = parts[0].replace("K", "000").parse().unwrap();
                    let mut duration_parts = parts[1].split(" ");
                    let duration_value: u64 = duration_parts.next().unwrap().parse().unwrap();
                    let duration_unit = duration_parts.next().unwrap_or("s");

                    let duration = match duration_unit {
                        "h" => Duration::from_secs(duration_value * 3600),
                        "m" => Duration::from_secs(duration_value * 60),
                        _ => Duration::from_secs(duration_value),
                    };

                    let quota = Quota::with_period(duration)
                        .unwrap()
                        .allow_burst(NonZeroU32::new(count).unwrap());
                    let rate_limiter = RateLimiter::direct(quota);
                    log::debug!("↳ {:?}", rate_limiter);

                    rate_limiters.insert(upstream.http_url.clone(), rate_limiter);
                }
            }
            None => log::debug!("↳ Unlimited"),
        }
    }

    let app_config = Data::new(Mutex::new(config));

    log::info!("RateLimiters: {:?}", rate_limiters);
    let app_rate_limiters = Data::new(Mutex::new(rate_limiters));

    let mut cache: TtlHashMap<String, Value> = TtlHashMap::new(Duration::from_secs(3600 * 24 * 365));
    cache.max_nodes = Some(1000000);
    cache.autoclean = AutoClean::Never;
    let app_cache = Data::new(Mutex::new(cache));

    HttpServer::new(move || {
        App::new()
            .app_data(app_config.clone())
            .app_data(app_rate_limiters.clone())
            .app_data(app_cache.clone())
            .service(index)
    })
    .bind((host, port))?
    .run()
    .await
}