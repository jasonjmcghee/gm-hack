use bevy::prelude::*;
use serde::Deserialize;
use warp::Filter;
use crate::network::ChatInputRequest;

#[derive(Resource)]
struct CrossbeamReceiver(pub crossbeam_channel::Receiver<String>);

pub struct ServerPlugin;

#[derive(Deserialize)]
struct Message {
    text: String,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        // Create a channel to send requests from the API to the Bevy app
        let (sender, receiver) = crossbeam_channel::unbounded();

        // Spawn a new thread to run the API server
        std::thread::spawn(move || {
            let routes = warp::post()
                .and(warp::path("chat"))
                .and(warp::body::json())
                .map(move |input: Message| {
                    sender.send(input.text).expect("Failed to send chat input");
                    warp::reply::json(&serde_json::json!({ "status": "ok" }))
                });

            let server = warp::serve(routes).run(([0, 0, 0, 0], 3030));

            // Use tokio to spawn the server future
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime")
                .block_on(server);
        });

        // Add a system to handle the chat input requests from the API
        app.insert_resource(CrossbeamReceiver(receiver))
            .add_systems(Update, handle_chat_input_requests);
    }
}

fn handle_chat_input_requests(
    mut chat_input_events: EventWriter<ChatInputRequest>,
    receiver: Res<CrossbeamReceiver>
) {
    for input in receiver.0.try_iter() {
        chat_input_events.send(ChatInputRequest { text: input });
    }
}
