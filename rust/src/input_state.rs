use godot::classes::{InputEvent, InputEventMouseButton};
use godot::prelude::*;

#[derive(Default)]
pub struct InputState {
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub jump_pressed: bool,
    pub mouse_pressed: bool,
    pub mouse_position: Vector2,
}

pub fn handle_input(event: Gd<InputEvent>, input: &mut InputState) {
    if event.is_action_pressed("move_left") {
        input.left_pressed = true;
    }
    if event.is_action_pressed("move_right") {
        input.right_pressed = true;
    }
    if event.is_action_released("move_left") {
        input.left_pressed = false;
    }
    if event.is_action_released("move_right") {
        input.right_pressed = false;
    }
    if event.is_action_pressed("jump") {
        input.jump_pressed = true;
    }
    if event.is_action_released("jump") {
        input.jump_pressed = false;
    }

    if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
        if mouse_event.is_pressed() {
            input.mouse_pressed = true;
            input.mouse_position = mouse_event.get_position();
        } else {
            input.mouse_pressed = false;
        }
    }
}
