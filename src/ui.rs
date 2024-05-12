use crate::{Prompt};
use bevy::prelude::*;

use bevy_egui::{egui, EguiContexts};
use crate::network::ChatInputRequest;

#[derive(Default)]
pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, uniform_update_ui_system);
    }
}

fn uniform_update_ui_system(
    mut ctx: EguiContexts,
    mut prompt: ResMut<Prompt>,
    mut event_writer: EventWriter<ChatInputRequest>
) {
    let context = ctx.ctx_mut();
    let mut clicked = false;
    egui::Window::new("Prompt")
        .default_open(false)
        .show(context, |ui| {
        ui.horizontal(|ui| {
            ui.label("Prompt:");
            ui.text_edit_singleline(&mut prompt.text);
            clicked = ui.button("Submit").clicked();
        });
        ui.horizontal(|ui| {
            ui.label("Response:");
            ui.label(&prompt.response);
        });
    });

    if clicked {
        event_writer.send(ChatInputRequest {
            text: prompt.text.clone()
        });
    }
}