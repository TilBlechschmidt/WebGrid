use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use thiserror::Error;

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
        output: PathBuf,
        log: PathBuf,
    ) -> Result<Self, RecordingError> {
        let output_str = output.to_str().ok_or(RecordingError::InputPathInvalid)?;
        let raw_args = VideoRecorder::generate_arguments(&input, input_framerate, output_str);
        let args: Vec<&str> = raw_args.split(" ").collect();

        let mut log_file = File::create(log)?;
        log_file.write(&raw_args.as_bytes())?;

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
            stdin.write_all("q".as_bytes())?;
        }

        Ok(self.child.wait()?)
    }

    fn generate_arguments(input: &str, input_framerate: usize, output: &str) -> String {
        let crf = 46;
        let bitrate = "250k";
        let maxrate = "450k";
        let bufsize = "500k";

        format!(r#"
-y {input}
-c:v libx264 -preset ultrafast -crf {crf} -b:v {bitrate} -maxrate {maxrate} -bufsize {bufsize} -profile:v baseline -pix_fmt yuv420p -tune stillimage -x264-params keyint={keyint}:scenecut=0:keyint_min={keyint} -g {framerate}
-f hls -hls_playlist_type event -hls_time 6
-hls_segment_type fmp4 -hls_flags single_file+independent_segments
{output}
        "#,
            input = input,
            keyint = input_framerate * 2,
            framerate = input_framerate,
            output = output,
            crf = crf,
            bitrate = bitrate,
            maxrate = maxrate,
            bufsize = bufsize
        ).trim().replace("\n", " ")
    }
}
