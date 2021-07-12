use super::creation::SessionCreateResponse;
use crate::library::helpers::wait_for;
use hyper::{
    http::{Method, Request},
    Body, Client,
};
use std::io::Error as IoError;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Stdio;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;
use tokio::process::{Child, Command};

struct WebDriverState {
    internal_session_id: String,
    actual_capabilities: String,
}

/// Resolution of a graphical user interface
///
/// Composited by width and height and parsable from strings like "1920x1080"
#[derive(Debug, Clone, Copy)]
pub struct ScreenResolution(u16, u16);

impl FromStr for ScreenResolution {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((raw_width, raw_height)) = s.split_once("x") {
            if let (Ok(width), Ok(height)) = (raw_width.parse(), raw_height.parse()) {
                Ok(Self(width, height))
            } else {
                Err("unable to convert width and height components to unsigned integers")
            }
        } else {
            Err("missing 'x' separator in resolution")
        }
    }
}

/// Errors thrown during creation of a webdriver instance
#[derive(Debug, Error)]
pub enum WebDriverError {
    /// unable to spawn webdriver process
    #[error("unable to spawn webdriver process")]
    IoError(#[from] IoError),
    /// timed out while waiting for webdriver startup
    #[error("timed out while waiting for webdriver startup")]
    StartupTimeout,
    /// creation of local webdriver session failed
    #[error("creation of local webdriver session failed")]
    SessionCreationError(#[from] hyper::Error),
    /// HTTP request composition failed
    #[error("HTTP request composition failed")]
    RequestCompositionFailed(#[from] hyper::http::Error),
    /// failed to parse webdriver response
    #[error("failed to parse webdriver response: {0}")]
    ParseFailure(String, #[source] serde_json::Error),
}

/// Vendors for WebDriver executables
///
/// Used to determine the command line arguments that should be passed to the executable on startup.
#[derive(Debug, Clone, Copy)]
pub enum WebDriverVariant {
    /// [Google Chrome](http://chrome.google.com) controlled through [`chromedriver`](https://sites.google.com/chromium.org/driver/)
    Chrome,
    /// [Safari](https://www.apple.com/safari/) browser developed by Apple and controlled by the `safaridriver` executable available on macOS
    Safari,
    /// Mozilla [Firefox](https://www.mozilla.org/en-US/firefox/) controlled through [`geckodriver`](https://github.com/mozilla/geckodriver)
    Firefox,
}

impl FromStr for WebDriverVariant {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "chrome" => Ok(Self::Chrome),
            "safari" => Ok(Self::Safari),
            "firefox" => Ok(Self::Firefox),
            _ => Err("unknown webdriver variant"),
        }
    }
}

impl WebDriverVariant {
    /// Returns the port used by the driver.
    /// Since some drivers (chhrrroOOOOOOMMMMMEEEE :evil_looks:) are fundamentally broken
    /// and basic arguments like ports do not work on certain versions, we parametrise it.
    fn port(&self) -> u16 {
        match self {
            WebDriverVariant::Chrome => 4444,
            WebDriverVariant::Safari => 4444,
            WebDriverVariant::Firefox => 4444,
        }
    }

    fn extra_arguments(&self) -> &[&'static str] {
        match self {
            WebDriverVariant::Chrome => &["--port=4444"],
            WebDriverVariant::Safari => &["--diagnose", "-p", "4444"],
            WebDriverVariant::Firefox => &["-p", "4444"],
        }
    }
}

/// Builder for a webdriver instance
pub struct WebDriver<'a> {
    binary: &'a Path,
    variant: WebDriverVariant,
    resolution: ScreenResolution,
    capabilities: &'a str,
    startup_timeout: Duration,
}

impl<'a> Default for WebDriver<'a> {
    fn default() -> Self {
        Self {
            binary: Path::new("/"),
            variant: WebDriverVariant::Firefox,
            resolution: ScreenResolution(1920, 1080),
            capabilities: "{}",
            startup_timeout: Duration::from_secs(30),
        }
    }
}

impl<'a> WebDriver<'a> {
    /// Sets the location of the webdriver executable
    pub fn binary(mut self, location: &'a Path) -> Self {
        self.binary = location;
        self
    }

    /// Specifies the browser variant which may impact the commandline arguments passed to the driver
    pub fn variant(mut self, variant: WebDriverVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the resolution to which the browser window will be resized post-launch
    pub fn resolution(mut self, resolution: ScreenResolution) -> Self {
        self.resolution = resolution;
        self
    }

    /// Updates the maximum duration the driver may take to become ready
    pub fn startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    /// Determines the capabilities that will be used to create the session upon launch
    pub fn capabilities(mut self, capabilities: &'a str) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Spawns an instance of the webdriver executable, creates a new session, and resizes the window
    pub async fn launch(self) -> Result<WebDriverInstance, WebDriverError> {
        let process = self.spawn_process()?;

        self.startup().await?;
        let state = self.create_local_session().await?;
        self.resize_window(&state.internal_session_id).await?;

        Ok(WebDriverInstance {
            state,
            process,

            socket_addr: self.socket_addr(),
        })
    }

    fn socket_addr(&self) -> SocketAddr {
        ([127, 0, 0, 1], self.variant.port()).into()
    }

    fn spawn_process(&self) -> Result<Child, IoError> {
        let args = self.variant.extra_arguments();

        Command::new(&self.binary)
            .args(args)
            .current_dir("/")
            // TODO Re-add environment isolation
            // .env_clear()
            // .env("PATH", std::env::var("PATH").unwrap_or_default())
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .spawn()
    }

    async fn startup(&self) -> Result<(), WebDriverError> {
        let url = format!("http://{}/status", self.socket_addr());

        match wait_for(&url, self.startup_timeout).await {
            Ok(_) => {
                log::info!("Driver became responsive");
                Ok(())
            }
            Err(_) => {
                log::error!("Timeout waiting for driver startup");
                Err(WebDriverError::StartupTimeout)
            }
        }
    }

    async fn create_local_session(&self) -> Result<WebDriverState, WebDriverError> {
        let uri = format!("http://{}/session", self.socket_addr());
        let body: Body = format!("{{\"capabilities\": {} }}", self.capabilities).into();

        let client = Client::new();
        let req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("Content-Type", "application/json")
            .body(body)?;

        let res = client.request(req).await?;

        let body = hyper::body::to_bytes(res.into_body()).await?;
        let response: SessionCreateResponse = serde_json::from_slice(&body).map_err(|e| {
            WebDriverError::ParseFailure(String::from_utf8_lossy(&body).to_string(), e)
        })?;
        let capabilities = serde_json::to_string(&response.value.capabilities).map_err(|e| {
            WebDriverError::ParseFailure(String::from_utf8_lossy(&body).to_string(), e)
        })?;

        Ok(WebDriverState {
            internal_session_id: response.value.session_id,
            actual_capabilities: capabilities,
        })
    }

    async fn resize_window(&self, session_id: &str) -> Result<(), WebDriverError> {
        let url = format!(
            "http://{}/session/{}/window/rect",
            self.socket_addr(),
            session_id
        );
        let body_string = format!(
            "{{\"x\": 0, \"y\": 0, \"width\": {}, \"height\": {}}}",
            self.resolution.0, self.resolution.1
        );

        let client = Client::new();
        let req = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Body::from(body_string))?;

        client.request(req).await?;

        Ok(())
    }
}

/// Running instance of a WebDriver executable
pub struct WebDriverInstance {
    state: WebDriverState,
    process: Child,
    socket_addr: SocketAddr,
}

impl WebDriverInstance {
    /// Capabilities object returned by the driver on session creation
    pub fn capabilities(&self) -> &str {
        &self.state.actual_capabilities
    }

    /// Session ID of the active session spawned with the driver
    pub fn session_id(&self) -> &str {
        &self.state.internal_session_id
    }

    /// Address where the driver is reachable
    pub fn socket_addr(&self) -> SocketAddr {
        self.socket_addr
    }

    /// Attempts to kill the webdriver and waits for it to die in agony
    pub async fn kill(mut self) -> Result<(), IoError> {
        self.process.kill().await
    }
}
