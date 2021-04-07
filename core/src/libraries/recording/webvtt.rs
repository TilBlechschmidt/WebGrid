use chrono::{DateTime, Duration, Utc};
use tokio::io::{AsyncWrite, AsyncWriteExt};

fn format_webvtt_duration(duration: Duration) -> String {
    let std_duration = duration.to_std().unwrap_or_default();
    let millis = std_duration.subsec_millis();
    let seconds = std_duration.as_secs();
    let whole_hours = seconds / 60 / 60;
    let whole_minutes = (seconds / 60) % 60;
    let remaining_seconds = seconds % 60;

    let formatted_hours = if whole_hours > 0 {
        format!("{:0>2}:", whole_hours)
    } else {
        String::new()
    };

    format!(
        "{}{:0>2}:{:0>2}.{:0>3}",
        formatted_hours, whole_minutes, remaining_seconds, millis
    )
}

pub struct SequentialWebVttWriter<F> {
    output: F,
    start_time: DateTime<Utc>,
    current_content: Option<(Duration, String)>,
}

impl<F: AsyncWrite + Unpin> SequentialWebVttWriter<F> {
    pub async fn new(mut writable: F, start_time: DateTime<Utc>) -> Result<Self, std::io::Error> {
        writable.write_all("WEBVTT\n".as_bytes()).await?;

        Ok(Self {
            output: writable,
            start_time,
            current_content: None,
        })
    }

    pub async fn write(&mut self, message: String) -> Result<(), std::io::Error> {
        let current_offset = Utc::now().signed_duration_since(self.start_time);

        // Drain and write any previously written message
        if let Some((offset, content)) = self.current_content.take() {
            let string = format!(
                "\n{} --> {}\n{}\n",
                format_webvtt_duration(offset),
                format_webvtt_duration(current_offset),
                content
            );

            self.output.write_all(string.as_bytes()).await?
        }

        // Save the new content
        self.current_content = Some((current_offset, message));

        Ok(())
    }

    pub async fn finish(&mut self) -> Result<(), std::io::Error> {
        // Flush the last message by storing a dummy message
        self.write(String::new()).await?;

        self.output.flush().await?;
        Ok(())
    }
}
