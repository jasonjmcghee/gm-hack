use bevy::prelude::*;
use reqwest::Client;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Source};
use std::env;
use std::io::Cursor;
use tokio::runtime::Runtime;

#[derive(Event)]
pub struct RequestAudioEvent {
    pub text: String,
}

pub fn request_audio_system(
    mut event_reader: EventReader<RequestAudioEvent>,
) {
    for event in event_reader.read() {
        let request_body = serde_json::json!({ "text": event.text });

        // Environment variable for API key
        let api_key = env::var("ELEVEN_LABS_API_KEY").expect("ELEVEN_LABS_API_KEY must be set");

        // Setup HTTP client
        let client = Client::new();
        let request = client.post("https://api.elevenlabs.io/v1/stream")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send();

        // Create a new runtime for the async block. This may need to be managed more globally depending on your app structure.
        let mut rt = Runtime::new().unwrap();
        rt.block_on(async {
            if let Ok(mut response) = request.await {
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();  // Create locally and use immediately

                while let Some(chunk) = response.chunk().await.unwrap() {
                    let cursor = Cursor::new(chunk.to_vec());
                    if let Ok(source) = Decoder::new_mp3(cursor) {
                        stream_handle.play_raw(source.convert_samples()).unwrap();
                    }
                }
            }
        });
    }
}
