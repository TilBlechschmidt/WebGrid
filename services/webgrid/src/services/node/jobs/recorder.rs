use super::super::{recorder::VideoRecorder, Context};
use anyhow::{bail, Result};
use async_trait::async_trait;
use scheduling::{Job, TaskManager};
use storage_lib::StorageHandler;
use tokio::task;

#[derive(Clone)]
pub struct RecorderJob {}

#[async_trait]
impl Job for RecorderJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let storage = manager.context.options.storage_directory.clone();
        let input = manager.context.options.recording_input.clone();
        let input_framerate = manager.context.options.recording_framerate;

        // Because we are only inserting files we can safely set the thresholds to 0
        let handler = StorageHandler::new(storage.clone(), 0.0, 0.0).await?;

        let prefix = &manager.context.id;
        let output_manifest = storage.join(format!("{}.m3u8", prefix));
        let output_stream = storage.join(format!("{}.m4s", prefix));
        let log = storage.join(format!("{}.log", prefix));

        // TODO Currently we are guessing the output_stream filename by evaluting the ffmpeg defaults
        // Instead use a flag and pass the path in as a parameter as well!
        let recorder = VideoRecorder::record(
            input.clone(),
            input_framerate,
            output_manifest.clone(),
            log.clone(),
        )?;

        // Register the files
        handler.add_file(&output_manifest).await?;
        handler.add_file(&output_stream).await?;
        handler.add_file(&log).await?;

        // Wait for system termination
        manager.ready().await;
        manager.termination_signal().await;

        // Stop recording and await clean shutdown
        let res = task::spawn_blocking(move || recorder.stop_capture()).await??;

        // Update the file metadata
        handler.add_file(&output_manifest).await?;
        handler.add_file(&output_stream).await?;
        handler.add_file(&log).await?;

        // Check the return code
        if !res.success() {
            bail!("Recorder finished with non-zero exit code.");
        }

        Ok(())
    }
}

impl RecorderJob {
    pub fn new() -> Self {
        Self {}
    }
}
