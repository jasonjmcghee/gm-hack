use bevy::{
    prelude::*
    ,
    window::{WindowResolution},
};
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::sprite::{ MaterialMesh2dBundle, Mesh2dHandle};
use bevy_egui::EguiPlugin;
use dotenv::dotenv;
use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::actions::SceneUpdate;
use crate::audio_plugin::{request_audio_system, RequestAudioEvent};
use crate::network::{chat_writer, GroqPlugin, Prompt};
use crate::server::ServerPlugin;
use crate::ui::UIPlugin;

mod screen_space_quad;
mod custom_material;
mod ui;
mod network;
mod server;
mod actions;
mod base_screen_space_material;
mod audio_plugin;

pub const WIDTH: f32 = 720.0;
pub const HEIGHT: f32 = 720.0;

/// System set to allow ordering of camera systems
#[derive(Debug, Clone, Copy, SystemSet, PartialEq, Eq, Hash)]
pub struct CamSystemSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Event)]
enum MovementEvent {
    Left,
    Right,
    Up,
    Down,
}

fn main() {
    dotenv().ok();
    let mut app = App::new();

    app.insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(Msaa::Sample4)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(WIDTH, HEIGHT),
                        title: "Sim".to_string(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    ..default()
                })
        )
        .add_plugins(ServerPlugin)
        .add_plugins(GroqPlugin)
        // .add_plugins(Material2dPlugin::<CustomMaterial>::default())
        // .add_plugins(MaterialPlugin::<ScreenSpaceMaterial>::default())
        .add_plugins((EguiPlugin, UIPlugin))
        //Create the aspect ratio as a resource. Only one instance of this data is needed so a global resource was chosen
        .init_resource::<Prompt>()
        .add_event::<MovementEvent>()
        .add_event::<RequestAudioEvent>()
        .add_systems(Startup, setup)
        .add_systems(Update, request_audio_system)
        .add_systems(Update, keyboard_input)
        .add_systems(Update, update_map)
        .add_systems(Update, chat_writer);

    app.init_resource::<EguiWantsFocus>()
        .add_systems(PostUpdate, check_egui_wants_focus)
        .configure_sets(
            Update,
            CamSystemSet.run_if(resource_equals(EguiWantsFocus(false))),
        );

    app.run();
}

#[derive(Resource, Deref, DerefMut, PartialEq, Eq, Default)]
struct EguiWantsFocus(bool);

// todo: make run condition when Bevy supports mutable resources in them
fn check_egui_wants_focus(
    mut contexts: Query<&mut bevy_egui::EguiContext>,
    mut wants_focus: ResMut<EguiWantsFocus>,
) {
    let ctx = contexts.iter_mut().next();
    let new_wants_focus = if let Some(ctx) = ctx {
        let ctx = ctx.into_inner().get_mut();
        ctx.wants_pointer_input() || ctx.wants_keyboard_input()
    } else {
        false
    };
    wants_focus.set_if_neq(EguiWantsFocus(new_wants_focus));
}

#[derive(Component, Deserialize, Serialize, Clone, PartialEq)]
struct Point { x: u8, y: u8 }

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    let padding = 2;
    let count: usize = 20;
    let size: usize = 20;
    let width = WIDTH + (padding * (count - 1)) as f32;
    let height = HEIGHT + (padding * (count - 1)) as f32;


    for i in 0..count {
        for j in 0..count {
            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: Mesh2dHandle(meshes.add(Rectangle::new(
                        (size) as f32, (size) as f32
                    ))),
                    material: materials.add(Color::rgb(1.0, 1.0, 1.0)),
                    transform: Transform::from_xyz(
                        -width * 0.25 + (i * (size + padding)) as f32 - padding as f32,
                        -height * 0.25 + (j * (size + padding)) as f32 - padding as f32,
                        1.0
                    ),
                    ..default()
                },
                Point { x: i as u8, y: j as u8 },
            ));
        }
    }
}

fn update_map(
    mut event_reader: EventReader<SceneUpdate>,
    mut event_writer: EventWriter<RequestAudioEvent>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(&Point, &mut Handle<ColorMaterial>)>,
) {
    for event in event_reader.read() {
        match event {
            SceneUpdate::UpdateGame {
                clear_grid,
                update_points,
                game_end,
                message,
            } => {
                if let Some(clear_grid) = clear_grid {
                    if *clear_grid {
                        // clear grid
                        for (_, col) in materials.iter_mut() {
                            col.color = Color::rgb(1.0, 1.0, 1.0);
                        }
                    }
                }

                for point_color in update_points {
                    let current_point = point_color.point.clone();
                    for (point, material) in query.iter() {
                        if *point == current_point {
                            materials.get_mut(material).unwrap().color = point_color.color;
                        }
                    }
                }

                if let Some(game_end) = game_end {
                    println!("Win?: {}", game_end);
                    let text = if *game_end {
                        "Everyone wins sometimes."
                    } else {
                        "Game over man, game over."
                    };
                    event_writer.send(RequestAudioEvent {
                        text: text.to_string(),
                    });
                }

                if let Some(message) = message {
                    println!("Message: {}", message);
                    event_writer.send(RequestAudioEvent {
                        text: message.to_string(),
                    });
                }
            }
            SceneUpdate::Sorry { error } => {
                println!("Error Message: {}", error);
                event_writer.send(RequestAudioEvent {
                    text: error.to_string(),
                });
            }
        }
    }
}

fn gamepad_input(
    gamepads: Res<Gamepads>,
    axes: Res<Axis<GamepadAxis>>,
    mut action_writer: EventWriter<MovementEvent>,
) {
    let gamepad = gamepads.iter().next();

    if let Some(gamepad) = gamepad {
        let left_stick_x = axes
            .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
            .unwrap_or(0.0);

        let left_stick_y = axes
            .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
            .unwrap_or(0.0);

        if left_stick_x < -0.5 {
            action_writer.send(MovementEvent::Left);
        } else if left_stick_x > 0.5 {
            action_writer.send(MovementEvent::Right);
        } else if left_stick_y < -0.5 {
            action_writer.send(MovementEvent::Up);
        } else if left_stick_y > 0.5 {
            action_writer.send(MovementEvent::Down);
        }
    }
}

fn keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut action_writer: EventWriter<MovementEvent>,
) {
    // Check for left movement keys
    if keyboard.pressed(KeyCode::KeyA) {
        action_writer.send(MovementEvent::Left);
    }

    // Check for right movement keys
    if keyboard.pressed(KeyCode::KeyD) {
        action_writer.send(MovementEvent::Right);
    }

    // Check for left movement keys
    if keyboard.pressed(KeyCode::KeyW) {
        action_writer.send(MovementEvent::Up);
    }

    // Check for right movement keys
    if keyboard.pressed(KeyCode::KeyS) {
        action_writer.send(MovementEvent::Down);
    }
}