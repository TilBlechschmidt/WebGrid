use super::super::Context;
use crate::libraries::recording::SequentialWebVttWriter;
use crate::libraries::recording::VideoRecorder;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::storage::StorageHandler;
use crate::with_shared_redis_resource;
use anyhow::{bail, Result};
use async_trait::async_trait;
use chrono::Utc;
use jatsl::{Job, TaskManager};
use log::warn;
use tokio::{fs::File, task};

#[derive(Clone)]
pub struct RecorderJob {}

#[async_trait]
impl Job for RecorderJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        if manager.context.options.storage_directory.is_none() {
            warn!("No storage directory provided. Video recording has been disabled.");
            return Ok(());
        }

        let storage = manager.context.options.storage_directory.clone().unwrap();
        let input = manager.context.options.recording_input.clone();
        let input_framerate = manager.context.options.recording_framerate;
        let quality_preset = manager.context.options.recording_quality();

        // Because we are only inserting files we can safely set the thresholds to 0
        // let handler = StorageHandler::new(storage.clone(), 0.0, 0.0).await?;
        let storage_id = StorageHandler::storage_id(&storage).await?;

        let prefix = &manager.context.id;
        let output_manifest = storage.join(format!("{}.m3u8", prefix));
        let output_stream = storage.join(format!("{}.m4s", prefix));
        let log = storage.join(format!("{}.log", prefix));

        // TODO Currently we are guessing the output_stream filename by evaluting the ffmpeg defaults
        // Instead use a flag and pass the path in as a parameter as well!
        let recorder = VideoRecorder::record(
            input.clone(),
            input_framerate,
            quality_preset,
            output_manifest.clone(),
            log.clone(),
        )?;
        let recording_start = Utc::now();

        // Create a WebVTT output and store it in the service context
        let webvtt_path = storage.join(format!("{}.vtt", prefix));
        let webvtt_file = File::create(&webvtt_path).await?;
        let webvtt = SequentialWebVttWriter::new(webvtt_file, recording_start).await?;
        *(manager.context.webvtt.lock().await) = Some(webvtt);

        // Register the files
        {
            let mut redis = with_shared_redis_resource!(manager);
            StorageHandler::queue_file_metadata(&webvtt_path, &storage_id, &mut redis).await?;
            StorageHandler::queue_file_metadata(&output_manifest, &storage_id, &mut redis).await?;
            StorageHandler::queue_file_metadata(&output_stream, &storage_id, &mut redis).await?;
            StorageHandler::queue_file_metadata(&log, &storage_id, &mut redis).await?;
        }

        // Wait for system termination
        manager.ready().await;
        manager.termination_signal().await;

        // Stop recording and await clean shutdown
        let res = task::spawn_blocking(move || recorder.stop_capture()).await??;

        // Update the file metadata
        {
            let mut redis = with_shared_redis_resource!(manager);
            StorageHandler::queue_file_metadata(&webvtt_path, &storage_id, &mut redis).await?;
            StorageHandler::queue_file_metadata(&output_manifest, &storage_id, &mut redis).await?;
            StorageHandler::queue_file_metadata(&output_stream, &storage_id, &mut redis).await?;
            StorageHandler::queue_file_metadata(&log, &storage_id, &mut redis).await?;
        }

        // Flush the WebVTT outputs
        if let Some(webvtt) = &mut *manager.context.webvtt.lock().await {
            webvtt.finish().await?;
        }

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
