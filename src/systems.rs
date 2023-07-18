use crate::conversions;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::EventReader,
    system::{Res, ResMut, Resource, SystemParam},
};
use bevy_input::keyboard::KeyCode;
#[cfg(feature = "touch")]
use bevy_input::touch::TouchInput;
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseWheel},
    ButtonState, Input,
};
use bevy_window::{CursorEntered, CursorLeft, CursorMoved, ReceivedCharacter};
use iced::{keyboard, mouse, Event as IcedEvent, Point};

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IcedEventQueue(Vec<iced::Event>);

#[derive(SystemParam)]
pub struct InputEvents<'w, 's> {
    cursor_entered: EventReader<'w, 's, CursorEntered>,
    cursor_left: EventReader<'w, 's, CursorLeft>,
    cursor: EventReader<'w, 's, CursorMoved>,
    mouse_button: EventReader<'w, 's, MouseButtonInput>,
    mouse_wheel: EventReader<'w, 's, MouseWheel>,
    received_character: EventReader<'w, 's, ReceivedCharacter>,
    keyboard_input: EventReader<'w, 's, KeyboardInput>,
    #[cfg(feature = "touch")]
    touch_input: EventReader<'w, 's, TouchInput>,
}

fn compute_modifiers(input_map: &Input<KeyCode>) -> keyboard::Modifiers {
    let mut modifiers = keyboard::Modifiers::default();
    if input_map.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        modifiers |= keyboard::Modifiers::CTRL;
    }
    if input_map.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        modifiers |= keyboard::Modifiers::SHIFT;
    }
    if input_map.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) {
        modifiers |= keyboard::Modifiers::ALT;
    }
    if input_map.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]) {
        modifiers |= keyboard::Modifiers::LOGO;
    }
    modifiers
}

pub fn process_input(
    mut events: InputEvents,
    mut event_queue: ResMut<IcedEventQueue>,
    input_map: Res<Input<KeyCode>>,
) {
    event_queue.clear();

    for ev in events.cursor.iter() {
        event_queue.push(IcedEvent::Mouse(mouse::Event::CursorMoved {
            position: Point::new(ev.position.x, ev.position.y),
        }));
    }

    for ev in events.mouse_button.iter() {
        let button = conversions::mouse_button(ev.button);
        event_queue.push(IcedEvent::Mouse(match ev.state {
            ButtonState::Pressed => iced::mouse::Event::ButtonPressed(button),
            ButtonState::Released => iced::mouse::Event::ButtonReleased(button),
        }))
    }

    for _ev in events.cursor_entered.iter() {
        event_queue.push(IcedEvent::Mouse(iced::mouse::Event::CursorEntered));
    }

    for _ev in events.cursor_left.iter() {
        event_queue.push(IcedEvent::Mouse(iced::mouse::Event::CursorLeft));
    }

    for ev in events.mouse_wheel.iter() {
        event_queue.push(IcedEvent::Mouse(iced::mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: ev.x, y: ev.y },
        }));
    }

    for ev in events.received_character.iter() {
        event_queue.push(IcedEvent::Keyboard(
            iced::keyboard::Event::CharacterReceived(ev.char),
        ));
    }

    for ev in events.keyboard_input.iter() {
        if let Some(code) = ev.key_code {
            use keyboard::Event::*;
            let modifiers = compute_modifiers(&input_map);
            let event = match code {
                KeyCode::ControlLeft
                | KeyCode::ControlRight
                | KeyCode::ShiftLeft
                | KeyCode::ShiftRight
                | KeyCode::AltLeft
                | KeyCode::AltRight
                | KeyCode::SuperLeft
                | KeyCode::SuperRight => ModifiersChanged(modifiers),
                code => {
                    let key_code = conversions::key_code(code);
                    if ev.state.is_pressed() {
                        KeyPressed {
                            key_code,
                            modifiers,
                        }
                    } else {
                        KeyReleased {
                            key_code,
                            modifiers,
                        }
                    }
                }
            };

            event_queue.push(IcedEvent::Keyboard(event));
        }
    }

    #[cfg(feature = "touch")]
    for ev in events.touch_input.iter() {
        event_queue.push(IcedEvent::Touch(conversions::touch_event(ev)));
    }
}
