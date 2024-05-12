use bevy::prelude::*;
use reqwest::Client;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Source};
use std::env;
use std::io::Cursor;
use std::process::Command;
use tokio::runtime::Runtime;

#[derive(Event)]
pub struct RequestAudioEvent {
    pub text: String,
}

pub fn request_audio_system(
    mut event_reader: EventReader<RequestAudioEvent>,
) {
    for event in event_reader.read() {

        // Environment variable for API key
        let api_key = env::var("ELEVEN_LABS_API_KEY").expect("ELEVEN_LABS_API_KEY must be set");
        let voice_id = "ZiJr5cZOXQztQsR7bLrz";
        let text = event.text.clone();
        let model_id = "eleven_multilingual_v2";
        let stability = 0.5;
        let similarity_boost = 0.5;

        // Construct the JSON data to send
        let json_payload = format!(
            r#"{{
            "text": "{}",
            "model_id": "{}",
            "voice_settings": {{
                "stability": {},
                "similarity_boost": {}
            }}
        }}"#,
            text, model_id, stability, similarity_boost
        );

        // Create the full curl command as a shell string
        let command = format!(
            "curl -X POST \"https://api.elevenlabs.io/v1/text-to-speech/{}\" \
        -H \"Content-Type: application/json\" \
        -H \"xi-api-key: {}\" \
        -d '{}' -o temp_audio.mp3 && afplay temp_audio.mp3 && rm temp_audio.mp3",
            voice_id, api_key, json_payload
        );

        // Execute the command in a shell
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .expect("Failed to execute shell command");

        // Check for errors
        if !output.status.success() {
            eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
        }
    }
}
