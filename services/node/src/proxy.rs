use crate::context::Context;
use chrono::prelude::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{
    body, client::HttpConnector, Body, Client as HttpClient, Error as HyperError, Method, Request,
    Response, Server,
};
use log::{info, trace, warn};
use redis::{AsyncCommands, RedisResult};
use regex::Regex;
use shared::ports::ServicePort;
use std::sync::Arc;

lazy_static! {
    static ref SESSION_RE: Regex = Regex::new(r"/session/(?P<sid>[^/]*)(?:/(?P<op>.+))?").unwrap();
}

async fn handle_driver_response(
    driver_response: Response<Body>,
    ctx: Arc<Context>,
    path: String,
    method: Method,
    external_session_id: String,
) -> Result<Response<Body>, HyperError> {
    // Split the response body apart and read it for logging and termination checks
    let response_status = driver_response.status();
    let (parts, body) = driver_response.into_parts();

    let body = match body::to_bytes(body).await {
        Ok(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string()),
        Err(_) => "".to_string(),
    };

    trace!(
        "<- {} {} => {}; body: '{}'",
        method,
        path,
        response_status,
        body.replace("\n", "")
    );

    // Evaluate if this is a termination request
    let is_session_delete_request =
        method == Method::DELETE && path == format!("/session/{}", external_session_id);
    let is_window_delete_request =
        method == Method::DELETE && path == format!("/session/{}/window", external_session_id);

    let session_closed = if is_window_delete_request {
        lazy_static! {
            static ref EMPTY_VALUE_RE: Regex = Regex::new(r#""value": ?\[\]"#).unwrap();
        }

        EMPTY_VALUE_RE.is_match(&body)
    } else {
        is_session_delete_request
    };

    // Terminate the service if requested
    if session_closed {
        warn!("Session closed by downstream");
        ctx.heart.kill();
    }

    Ok(Response::from_parts(parts, Body::from(body)))
}

async fn handle(
    mut req: Request<Body>,
    ctx: Arc<Context>,
    client: HttpClient<HttpConnector, Body>,
    internal_session_id: String,
    external_session_id: String,
) -> Result<Response<Body>, HyperError> {
    // Update the generic metrics and reset the lifetime
    let mut con = ctx.con.clone();
    let _: RedisResult<()> = con
        .hset(
            format!("session:{}:downstream", ctx.config.session_id),
            "lastSeen",
            Utc::now().to_rfc3339(),
        )
        .await;

    ctx.heart.reset_lifetime();

    // Translate the request path
    let req_path = req
        .uri()
        .path_and_query()
        .map(|x| x.as_str().to_owned())
        .unwrap_or_else(|| "".to_string());
    let path = match SESSION_RE.captures(&req_path) {
        Some(captures) => {
            let session_id = &captures["sid"];

            if session_id == external_session_id {
                match captures.name("op") {
                    Some(operation) => {
                        format!("/session/{}/{}", internal_session_id, operation.as_str())
                    }
                    None => format!("/session/{}", internal_session_id),
                }
            } else {
                req_path.to_string()
            }
        }
        None => req_path.to_string(),
    };

    // Overwrite the original path with the translated one
    let req_method = req.method().clone();
    let uri_string = format!("http://{}{}", ctx.driver_addr, path);
    let uri = uri_string.parse().unwrap();
    *req.uri_mut() = uri;

    // Split the request body apart and read it for logging
    let (req_parts, req_body) = req.into_parts();
    let req_body = match body::to_bytes(req_body).await {
        Ok(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string()),
        Err(_) => "".to_string(),
    };

    trace!(
        "-> {} {} -> {}; body: '{}'",
        req_method,
        path,
        uri_string,
        req_body.replace("\n", "")
    );

    // Rebuild the request and create a request object
    let rebuild_req = Request::from_parts(req_parts, Body::from(req_body));
    let proxy_request = client.request(rebuild_req);

    // Dispatch the request
    match proxy_request.await {
        Ok(driver_response) => {
            handle_driver_response(
                driver_response,
                ctx,
                req_path,
                req_method,
                external_session_id,
            )
            .await
        }
        Err(driver_response) => {
            warn!("Upstream error {}", driver_response);
            Err(driver_response)
        }
    }
}

pub async fn serve_proxy(ctx: Arc<Context>, internal_session_id: String) {
    let in_addr = ServicePort::Node.socket_addr();
    let out_addr = ctx.driver_addr;
    let client_main = HttpClient::new();

    info!("WebDriver proxy serving {:?} -> {:?}", in_addr, out_addr);

    let make_service = make_service_fn(move |_| {
        let ctx = ctx.clone();
        let client = client_main.clone();
        let external_session_id = ctx.config.session_id.clone();
        let internal_session_id = internal_session_id.clone();

        async move {
            Ok::<_, HyperError>(service_fn(move |req| {
                handle(
                    req,
                    ctx.clone(),
                    client.clone(),
                    internal_session_id.clone(),
                    external_session_id.clone(),
                )
            }))
        }
    });

    let server = Server::bind(&in_addr).serve(make_service);
    tokio::spawn(server);
}
