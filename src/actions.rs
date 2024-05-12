use bevy::prelude::{Color, Event, EventWriter};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use serde::de::Error as de_Error;
use crate::{actions, Point};

pub enum ActionTypes {
    UpdateGame,
    Sorry,
}

#[derive(Event)]
pub enum SceneUpdate {
    UpdateGame {
        clear_grid: Option<bool>,
        update_points: Vec<PointColor>,
        game_end: Option<bool>,
        message: Option<String>,
    },
    Sorry { error: String },
}


#[derive(Deserialize, Serialize)]
pub(crate) struct Sorry {
    pub(crate) error: String,
}

impl StringDefinition for Sorry {
    fn string_definition() -> String {
        serde_json::to_string(&Sorry {
            error: "String".to_string(),
        }).unwrap()
    }
}

#[derive(Deserialize, Serialize)]
pub struct PointHex {
    hex: String,
    point: Point,
}

pub struct PointColor {
    pub(crate) color: Color,
    pub(crate) point: Point,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct UpdateGame {
    clear_grid: Option<bool>,
    update_points: Vec<PointHex>,
    game_end: Option<bool>,
    message: Option<String>,
}

impl StringDefinition for UpdateGame {
    // The JSON schema for UpdateGame
    fn string_definition() -> String {
        "{\"type\":\"object\",\"properties\":{\"Point\":{\"type\":\"object\",\"properties\":{\"x\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":20,\"exclusiveMaximum\":false},\"y\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":20,\"exclusiveMaximum\":false}},\"required\":[\"x\",\"y\"]},\"PointHex\":{\"type\":\"object\",\"properties\":{\"hex\":{\"type\":\"string\"},\"point\":{\"$ref\":\"#/properties/Point\"}},\"required\":[\"hex\",\"point\"]},\"UpdateGame\":{\"type\":\"object\",\"properties\":{\"clear_grid\":{\"type\":[\"boolean\",\"null\"]},\"update_points\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/properties/PointHex\"}},\"game_end\":{\"type\":[\"boolean\",\"null\"]},\"message\":{\"type\":[\"string\",\"null\"]}},\"required\":[\"update_points\"]}}}".to_string()
    }
}

trait StringDefinition {
    fn string_definition() -> String;
}

pub fn handle_request(scene_update_events: &mut EventWriter<SceneUpdate>, request_json: &str) {
    match handle_request_with_result(request_json) {
        Ok(handled_action) => {
            match handled_action.action {
                ActionTypes::UpdateGame => {
                    if let Ok(turn_circle_color) = serde_json::from_str::<actions::UpdateGame>(&handled_action.value) {
                        scene_update_events.send(SceneUpdate::UpdateGame {
                            clear_grid: turn_circle_color.clear_grid,
                            update_points: turn_circle_color.update_points.iter().map(|point_color_raw| PointColor {
                                color: Color::hex(&point_color_raw.hex).unwrap_or(Color::PINK),
                                point: point_color_raw.point.clone(),
                            }).collect(),
                            game_end: turn_circle_color.game_end,
                            message: turn_circle_color.message,
                        });
                    } else {
                        println!("Error deserializing: {}", request_json);
                    }
                },
                ActionTypes::Sorry => {
                    let sorry: actions::Sorry = serde_json::from_str(&handled_action.value).unwrap();
                    scene_update_events.send(SceneUpdate::Sorry {
                        error: sorry.error,
                    });
                },
            }
        },
        Err(e) => {
            println!("Error deserializing: {} -- {}", e, request_json);
            // Execute sorry action?
        }
    }
}

fn handle_request_with_result(request_json: &str) -> Result<HandledAction, Error> {
    let action: Action = serde_json::from_str(request_json)?;
    let action_type = match action.action.as_str() {
        "UpdateGame" => ActionTypes::UpdateGame,
        "Sorry" => ActionTypes::Sorry,
        _ => {
            return Err(serde_json::Error::custom("Invalid action"));
        },
    };
    Ok(
        HandledAction {
            action: action_type,
            value: action.value,
        }
    )
}

pub fn build_system_prompt() -> String {
    let actions = [
        Action {
            action: "UpdateGame".to_string(),
            value: UpdateGame::string_definition(),
        },
        Action {
            action: "Sorry".to_string(),
            value: Sorry::string_definition(),
        }
    ];
    let serialized_actions = serde_json::to_string(&actions).unwrap();
    format!(
        "Always respond with valid JSON. You have access to the following possible actions: {}. \
        If you are updating the game board, YOU MUST FOLLOW THE JSON SCHEMA. \
        Every point requires an x and y value, and every hex requires a hex value. \
        This is how you make the game possible to play - you update the board after the user takes an action, \
        and you provide the user with the next state of the board. \
        You are a game master and you get to decide on a game to play, the rules, and the outcome. \
        You facilitate every interaction by updating a 20x20 square grid. You can make any rules you want, \
        like snake, or breakout or anything else, as long as you follow the JSON schema to represent the board. \
        Do not make up an ascii representation of the board or choose a game that can't be played with keyboard. \
        Always start the player somewhere. There must be one non-white square when the game starts! \
        It is critical that when a new game is being started you provide detailed instructions on how, the \
        game is played. A new game can start either because it's the first message, \
        or because the game has ended, or you, or the player wants to start over, \
        The user will respond to you with a movement action. \
        Always respond with valid JSON. Only respond with valid actions. \
        If the user is attempting a movement action, update the board, and appropriately \
        fill in the values as outlined. Always fill in `action` with the appropriate \
        string and `value` with the appropriate stringified and escaped JSON. \
        It is critical the JSON is escaped. Otherwise, choose the Sorry action. \
        Any other request should use the Sorry action and place \"sorry\" in its `error`.",
        serialized_actions
    )
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Action {
    pub(crate) action: String,
    pub(crate) value: String,
}

struct HandledAction {
    action: ActionTypes,
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt() {
        let prompt = build_system_prompt();
        assert_eq!(prompt, "Always respond with valid JSON. Given the following possible actions: [{\"action\":\"TurnCircleColor\",\"value\":\"{\\\"hex\\\":\\\"String\\\"}\"},{\"action\":\"Sorry\",\"value\":\"{\\\"error\\\":\\\"String\\\"}\"}], choose the one that best represents the user's request, and appropriately fill in the values as outlined. If none match, choose the Sorry action.");
    }
}