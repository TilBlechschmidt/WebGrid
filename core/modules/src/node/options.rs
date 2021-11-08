use crate::options::{RedisOptions, StorageOptions};
use domain::webdriver::{ScreenResolution, WebDriverVariant};
use library::helpers::parse_seconds;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;
use uuid::Uuid;

/// Options for the manager module
#[derive(Debug, StructOpt)]
pub struct Options {
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub redis: RedisOptions,

    /// Unique identifier of the linked session object
    #[structopt(env)]
    pub id: Uuid,

    /// Idle timeout (in seconds) which is in effect until the first client request is received.
    /// This allows the session to terminate early if the client no longer has any interest
    /// in the session or it itself ran into a local timeout (e.g. due to prolonged queueing).
    /// After the first request from a client has been received, the regular idle-timeout is
    /// taking effect.
    #[structopt(long, env, default_value = "30", parse(try_from_str = parse_seconds))]
    pub initial_timeout: Duration,

    /// If no WebDriver client request is received within the specified period, the node will
    /// terminate. Each incoming request resets the countdown.
    #[structopt(long, env, default_value = "120", parse(try_from_str = parse_seconds))]
    pub idle_timeout: Duration,

    /// Options relating to the WebDriver
    #[structopt(flatten)]
    pub webdriver: WebDriverOptions,

    /// Options about screen recording
    #[structopt(flatten)]
    pub recording: ScreenRecordingOptions,

    /// Hostname or IP address where this instance can be reached by proxy services
    #[structopt(short, long, env)]
    pub host: String,

    /// Maximum duration (in seconds) for the server to bind to a port and advertise its ready-state.
    /// If the server is for whatever reason unable to claim the port in this time, startup will fail.
    #[structopt(long, env, default_value = "120", parse(try_from_str = parse_seconds))]
    pub bind_timeout: Duration,

    /// Options regarding storage
    #[structopt(flatten)]
    pub storage: StorageOptions,
}

/// WebDriver related options
#[derive(Debug, StructOpt)]
pub struct WebDriverOptions {
    /// Location of the WebDriver executable
    #[structopt(env = "DRIVER")]
    pub binary: PathBuf,

    /// Variant of the WebDriver
    #[structopt(long, env = "DRIVER_VARIANT")]
    pub variant: WebDriverVariant,

    /// Screen resolution for new sessions
    #[structopt(long, env, default_value = "1920x1080")]
    pub resolution: ScreenResolution,

    /// Maximum duration (in seconds) the webdriver may take until it reports a ready state
    #[structopt(long, env, default_value = "120", parse(try_from_str = parse_seconds))]
    pub startup_timeout: Duration,

    /// Capabilities object which will be used to create a session with the driver (formatted as JSON)
    #[structopt(env)]
    pub capabilities: String,
}

// Screen recording related options
#[derive(Debug, StructOpt)]
pub struct ScreenRecordingOptions {
    /// Constant Rate Factor
    ///
    /// The range of the CRF scale is 0–51, where 0 is lossless, 23 is the default, and 51 is worst quality possible.
    /// A lower value generally leads to higher quality, and a subjectively sane range is 17–28.
    /// Consider 17 or 18 to be visually lossless or nearly so; it should look the same or nearly the same as the input but it isn't technically lossless.
    /// The range is exponential, so increasing the CRF value +6 results in roughly half the bitrate / file size, while -6 leads to roughly twice the bitrate.
    /// Choose the highest CRF value that still provides an acceptable quality. If the output looks good, then try a higher value. If it looks bad, choose a lower value.
    ///
    /// For more details, consult the ffmpeg H.264 documentation (section "Constant Rate Factor"):
    ///
    /// https://trac.ffmpeg.org/wiki/Encode/H.264
    #[structopt(long, env, default_value = "46")]
    crf: u8,

    /// Upper bitrate bound in bytes
    ///
    /// The average bitrate is determined by the constant rate factor and content
    /// however if the bitrate were to exceed this specified maximum bitrate limit, the codec will increase the CRF temporarily.
    ///
    /// For more details, consult the ffmpeg H.264 documentation (section "Constrained encoding"):
    ///
    /// https://trac.ffmpeg.org/wiki/Encode/H.264
    #[structopt(long, env, default_value = "450000")]
    max_bitrate: usize,

    /// ffmpeg input parameter specification
    #[structopt(
        name = "recording_input",
        long,
        env,
        default_value = "-rtbufsize 1500M -probesize 100M -video_size 1920x1080 -f x11grab -draw_mouse 0 -i :42"
    )]
    pub input: String,

    /// Framerate with which the screen should be recorded
    ///
    /// Changes both the input framerate (if possible) and the codec target framerate.
    /// Higher framerates yield smoother playback while significantly increasing data usage.
    /// It is recommended to stick to as low of a framerate as possible to allow more bitrate to go into each frame.
    #[structopt(long, env, default_value = "5")]
    pub framerate: usize,

    /// HLS segment duration in seconds
    ///
    /// For each segment of the given duration, a separate video file will be created. Reducing the duration allows for "faster"
    /// as in "closer to live" streaming but increases the overhead and CPU load. It is not recommended to change this value.
    #[structopt(long, env, default_value = "6")]
    pub segment_duration: usize,
}

impl ScreenRecordingOptions {
    pub fn generate_arguments(&self) -> String {
        let output = format!(
            "-method PUT http://127.0.0.1:{}/screen.m3u8",
            crate::constants::PORT_STORAGE
        );

        format!(r#"
        -y -framerate {framerate} {input} -vf scale=w=1280:h=720:force_original_aspect_ratio=decrease
        -c:v libx264 -preset ultrafast -crf {crf} -maxrate {maxrate} -bufsize {bufsize} -pix_fmt yuv420p -tune stillimage -x264-params keyint={keyint}:scenecut=0:keyint_min={keyint} -g {framerate}
        -f hls -hls_playlist_type event -hls_time {segment_duration}
        -hls_segment_type fmp4 -hls_flags program_date_time
        {output}
                "#,
                    input = self.input,
                    keyint = self.framerate * 2,
                    framerate = self.framerate,
                    output = output,
                    crf = self.crf,
                    maxrate = self.max_bitrate,
                    bufsize = self.max_bitrate * 2,
                    segment_duration = self.segment_duration
                ).trim().replace("\n", " ")
    }
}
