/// Background transcoding queue
use crate::{
    config::{AudioFormat, Quality},
    services::{FileStorage, TranscodingService},
};
use soul_core::TrackId;
use std::{collections::VecDeque, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct TranscodeJob {
    pub track_id: TrackId,
    pub input_path: PathBuf,
    pub qualities: Vec<(Quality, AudioFormat)>,
}

pub struct TranscodingQueue {
    queue: Arc<Mutex<VecDeque<TranscodeJob>>>,
    transcoding_service: Arc<TranscodingService>,
    file_storage: Arc<FileStorage>,
    workers: usize,
}

impl TranscodingQueue {
    pub fn new(
        transcoding_service: Arc<TranscodingService>,
        file_storage: Arc<FileStorage>,
        workers: usize,
    ) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            transcoding_service,
            file_storage,
            workers,
        }
    }

    /// Start worker tasks
    pub async fn start(self: Arc<Self>) {
        for worker_id in 0..self.workers {
            let queue = Arc::clone(&self);
            tokio::spawn(async move {
                tracing::info!("Transcoding worker {} started", worker_id);
                queue.worker_loop(worker_id).await;
            });
        }
    }

    /// Enqueue a transcoding job
    pub async fn enqueue(&self, job: TranscodeJob) {
        let mut queue = self.queue.lock().await;
        tracing::info!("Enqueued transcoding job for track {}", job.track_id.as_str());
        queue.push_back(job);
    }

    /// Worker loop - processes jobs from the queue
    async fn worker_loop(&self, worker_id: usize) {
        loop {
            // Get next job
            let job = {
                let mut queue = self.queue.lock().await;
                queue.pop_front()
            };

            if let Some(job) = job {
                tracing::info!(
                    "Worker {} processing track {}",
                    worker_id,
                    job.track_id.as_str()
                );

                // Process each quality variant
                for (quality, format) in &job.qualities {
                    if let Err(e) = self.process_variant(&job, *quality, *format).await {
                        tracing::error!(
                            "Worker {} failed to transcode track {} to {:?}/{:?}: {}",
                            worker_id,
                            job.track_id.as_str(),
                            quality,
                            format,
                            e
                        );
                    } else {
                        tracing::info!(
                            "Worker {} transcoded track {} to {:?}/{:?}",
                            worker_id,
                            job.track_id.as_str(),
                            quality,
                            format
                        );
                    }
                }

                tracing::info!(
                    "Worker {} completed track {}",
                    worker_id,
                    job.track_id.as_str()
                );
            } else {
                // No jobs available, sleep briefly
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    /// Process a single quality variant
    async fn process_variant(
        &self,
        job: &TranscodeJob,
        quality: Quality,
        format: AudioFormat,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create temporary output path
        let temp_dir = std::env::temp_dir();
        let output_filename = format!(
            "{}-{:?}.{}",
            job.track_id.as_str(),
            quality,
            format.extension()
        );
        let output_path = temp_dir.join(output_filename);

        // Transcode
        self.transcoding_service
            .transcode(&job.input_path, &output_path, quality, format)
            .await?;

        // Read transcoded file
        let transcoded_data = tokio::fs::read(&output_path).await?;

        // Store in file storage
        self.file_storage
            .store_variant(&job.track_id, quality, format, &transcoded_data)
            .await?;

        // Clean up temporary file
        tokio::fs::remove_file(&output_path).await?;

        // TODO: Update database to mark variant as available

        Ok(())
    }

    /// Get queue length
    pub async fn queue_length(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }
}
