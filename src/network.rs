use std::env;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use bevy_http_client::{HttpClient, HttpClientPlugin};
use bevy_http_client::prelude::{HttpTypedRequestTrait, TypedRequest, TypedResponse};
use crate::actions::{build_system_prompt, handle_request, SceneUpdate};
use crate::MovementEvent;

#[derive(Resource)]
pub struct Prompt {
    pub(crate) text: String,
    pub(crate) response: String,
}
impl Default for Prompt {
    fn default() -> Self {
        Self {
            text: "".to_string(),
            response: "".to_string(),
        }
    }
}

// Define request message structure
#[derive(Deserialize, Serialize, Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

impl ChatMessage {
    pub fn system_prompt() -> Self {
        Self {
            role: "system".to_string(),
            content: build_system_prompt()
        }
    }
}

// Define request structure
#[derive(Serialize, Debug)]
struct ChatCompletionRequest {
    messages: Vec<ChatMessage>,
    model: String,
    temperature: f32,
    max_tokens: u32,
    top_p: f32,
    stream: bool,
    response_format: ResponseFormat,
    stop: Option<String>,
}

// Define response format type
#[derive(Serialize, Debug)]
struct ResponseFormat {
    r#type: String,
}

// Define the main response structure
#[derive(Deserialize, Debug)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
    system_fingerprint: String,
    x_groq: Groq,
}

// Nested response structures
#[derive(Deserialize, Debug)]
struct Choice {
    index: u32,
    message: Message,
    logprobs: Option<String>,
    finish_reason: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: u32,
    prompt_time: f64,
    completion_tokens: u32,
    completion_time: f64,
    total_tokens: u32,
    total_time: f64,
}

#[derive(Deserialize, Serialize, Debug, Resource)]
struct AllMessages {
    messages: Vec<ChatMessage>,
}

#[derive(Deserialize, Debug)]
struct Groq {
    id: String,
}

#[derive(Event)]
struct ApiResponseEvent {
    response: ChatCompletionResponse,
}

// Plugin encapsulating the network functionality
pub struct GroqPlugin;

impl Plugin for GroqPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HttpClientPlugin)
            .add_event::<TypedResponse<ChatCompletionResponse>>()
            .add_event::<ChatInputRequest>()
            .add_event::<SceneUpdate>()
            .insert_resource(AllMessages { messages: vec![] })
            .add_systems(Startup, initialize)
            .add_systems(Update, chat_reader)
            .add_systems(Update, handle_response)
            .register_request_type::<ChatCompletionResponse>();
    }
}

fn initialize(mut all_messages: ResMut<AllMessages>, mut ev_request: EventWriter<TypedRequest<ChatCompletionResponse>>) {
    let api_key = env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set");

    all_messages.messages.push(ChatMessage::system_prompt());
    all_messages.messages.push(ChatMessage { role: "user".to_string(), content: "Let's play a game!".to_string() });

    let request_body = ChatCompletionRequest {
        messages: all_messages.messages.clone(),
        model: "llama3-70b-8192".to_string(),
        temperature: 1.0,
        max_tokens: 1024,
        top_p: 1.0,
        stream: false,
        response_format: ResponseFormat { r#type: "json_object".to_string() },
        stop: None,
    };

    ev_request.send(
        HttpClient::new()
            .post("https://api.groq.com/openai/v1/chat/completions")
            .headers(&[
                ("Authorization", &format!("Bearer {}", api_key)),
                ("Content-Type", "application/json")
            ])
            .json(&request_body)
            .with_type::<ChatCompletionResponse>(),
    );
}

#[derive(Event)]
pub struct ChatInputRequest {
    pub text: String
}

#[derive(Deserialize, Serialize)]
struct MoveRequest {
    pub move_action: String
}

pub fn chat_writer(
    mut event_reader: EventReader<MovementEvent>,
    mut event_writer: EventWriter<ChatInputRequest>
) {
    for event in event_reader.read() {
        let text = match event {
            MovementEvent::Left => "left",
            MovementEvent::Right => "right",
            MovementEvent::Up => "up",
            MovementEvent::Down => "down",
        };
        event_writer.send(ChatInputRequest {
            text: serde_json::to_string(&MoveRequest {
                move_action: text.to_string()
            }).unwrap()
        });
    }
}


fn chat_reader(mut all_messages: ResMut<AllMessages>, mut event_reader: EventReader<ChatInputRequest>, mut ev_request: EventWriter<TypedRequest<ChatCompletionResponse>>) {
    for event in event_reader.read() {
        let api_key = env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set");

        all_messages.messages.push(ChatMessage { role: "user".to_string(), content: event.text.clone() });
        let request_body = ChatCompletionRequest {
            messages: all_messages.messages.clone(),
            model: "llama3-70b-8192".to_string(),
            temperature: 1.0,
            max_tokens: 1024,
            top_p: 1.0,
            stream: false,
            response_format: ResponseFormat { r#type: "json_object".to_string() },
            stop: None,
        };

        ev_request.send(
            HttpClient::new()
                .post("https://api.groq.com/openai/v1/chat/completions")
                .headers(&[
                    ("Authorization", &format!("Bearer {}", api_key)),
                    ("Content-Type", "application/json")
                ])
                .json(&request_body)
                .with_type::<ChatCompletionResponse>(),
        );
    }
}

fn handle_response(
    mut prompt: ResMut<Prompt>,
    mut ev_response: EventReader<TypedResponse<ChatCompletionResponse>>,
    mut scene_update_events: EventWriter<SceneUpdate>,
    mut all_messages: ResMut<AllMessages>,
) {
    for response in ev_response.read() {
        prompt.response = response.choices[0].message.content.clone();
        all_messages.messages.push(ChatMessage { role: "assistant".to_string(), content: prompt.response.clone() });
        handle_request(&mut scene_update_events, &prompt.response);
    }
}
