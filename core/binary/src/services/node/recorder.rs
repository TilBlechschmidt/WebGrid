use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use thiserror::Error;

pub struct VideoQualityPreset {
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
    pub crf: u8,
    /// Upper bitrate bound in bytes
    ///
    /// The average bitrate is determined by the constant rate factor and content
    /// however if the bitrate were to exceed this specified maximum bitrate limit, the codec will increase the CRF temporarily.
    ///
    /// For more details, consult the ffmpeg H.264 documentation (section "Constrained encoding"):
    ///
    /// https://trac.ffmpeg.org/wiki/Encode/H.264
    pub max_bitrate: usize,
}

impl Default for VideoQualityPreset {
    fn default() -> Self {
        Self {
            crf: 46,
            max_bitrate: 450_000,
        }
    }
}

impl VideoQualityPreset {
    pub fn new(crf: u8, max_bitrate: usize) -> Self {
        Self { crf, max_bitrate }
    }

    fn buffer_size(&self) -> usize {
        self.max_bitrate * 2
    }
}

#[derive(Debug, Error)]
pub enum RecordingError {
    #[error("invalid input path")]
    InputPathInvalid,
    #[error("failed to access recorder stdin")]
    StdinInaccessible,
    #[error("unable to start recorder")]
    RecorderUnavailable(#[from] std::io::Error),
}

pub struct VideoRecorder {
    child: Child,
}

impl VideoRecorder {
    pub fn record(
        input: String,
        input_framerate: usize,
        quality: VideoQualityPreset,
        output: PathBuf,
        log: PathBuf,
    ) -> Result<Self, RecordingError> {
        let output_str = output.to_str().ok_or(RecordingError::InputPathInvalid)?;
        let raw_args =
            VideoRecorder::generate_arguments(&input, input_framerate, quality, output_str);
        let args: Vec<&str> = raw_args.split(' ').collect();

        let mut log_file = File::create(log)?;
        log_file.write_all(&raw_args.as_bytes())?;

        let child = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stderr(log_file)
            .spawn()
            .expect("Failed to launch ffmpeg!");

        Ok(VideoRecorder { child })
    }

    pub fn stop_capture(mut self) -> Result<ExitStatus, RecordingError> {
        {
            let stdin = self
                .child
                .stdin
                .as_mut()
                .ok_or(RecordingError::StdinInaccessible)?;
            stdin.write_all(b"q")?;
        }

        Ok(self.child.wait()?)
    }

    fn generate_arguments(
        input: &str,
        input_framerate: usize,
        quality: VideoQualityPreset,
        output: &str,
    ) -> String {
        format!(r#"
-y {input} -vf scale=w=1280:h=720:force_original_aspect_ratio=decrease
-c:v libx264 -preset ultrafast -crf {crf} -maxrate {maxrate} -bufsize {bufsize} -pix_fmt yuv420p -tune stillimage -x264-params keyint={keyint}:scenecut=0:keyint_min={keyint} -g {framerate}
-f hls -hls_playlist_type event -hls_time 6
-hls_segment_type fmp4 -hls_flags single_file+independent_segments
{output}
        "#,
            input = input,
            keyint = input_framerate * 2,
            framerate = input_framerate,
            output = output,
            crf = quality.crf,
            maxrate = quality.max_bitrate,
            bufsize = quality.buffer_size()
        ).trim().replace("\n", " ")
    }
}
