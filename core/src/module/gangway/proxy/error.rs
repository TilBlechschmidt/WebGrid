use crate::{
    domain::webdriver::{WebdriverError, WebdriverErrorCode},
    library::communication::BlackboxError,
};
use hyper::{
    http::{Response, StatusCode},
    Body,
};

pub fn new_error_response(code: WebdriverErrorCode, error: BlackboxError) -> Response<Body> {
    let webdriver_error: WebdriverError = (code, error).into();
    let serialized = serde_json::to_string(&webdriver_error)
        .unwrap_or_else(|_| "failed to serialize error".into());

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(serialized))
        .unwrap()
}
