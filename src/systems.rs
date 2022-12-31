use crate::conversions;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::{Deref, DerefMut, ResMut, Resource};
use bevy::{
    ecs::system::SystemParam,
    input::{keyboard::KeyboardInput, mouse::MouseWheel},
    prelude::EventReader,
    window::{CursorEntered, CursorLeft, CursorMoved, ReceivedCharacter},
};
use iced_native::{keyboard, mouse, Event as IcedEvent, Point};

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IcedEventQueue(Vec<iced_native::Event>);

#[derive(SystemParam)]
pub struct InputEvents<'w, 's> {
    cursor_entered: EventReader<'w, 's, CursorEntered>,
    cursor_left: EventReader<'w, 's, CursorLeft>,
    cursor: EventReader<'w, 's, CursorMoved>,
    mouse_button: EventReader<'w, 's, MouseButtonInput>,
    mouse_wheel: EventReader<'w, 's, MouseWheel>,
    received_character: EventReader<'w, 's, ReceivedCharacter>,
    keyboard_input: EventReader<'w, 's, KeyboardInput>,
}

pub fn process_input(mut events: InputEvents, mut event_queue: ResMut<IcedEventQueue>) {
    event_queue.clear();

    for ev in events.cursor.iter() {
        event_queue.push(IcedEvent::Mouse(mouse::Event::CursorMoved {
            position: Point::new(ev.position.x, ev.position.y),
        }));
    }

    for ev in events.mouse_button.iter() {
        let button = conversions::mouse_button(ev.button);
        event_queue.push(IcedEvent::Mouse(match ev.state {
            bevy::input::ButtonState::Pressed => iced_native::mouse::Event::ButtonPressed(button),
            bevy::input::ButtonState::Released => iced_native::mouse::Event::ButtonReleased(button),
        }))
    }

    for _ev in events.cursor_entered.iter() {
        event_queue.push(IcedEvent::Mouse(iced_native::mouse::Event::CursorEntered));
    }

    for _ev in events.cursor_left.iter() {
        event_queue.push(IcedEvent::Mouse(iced_native::mouse::Event::CursorLeft));
    }

    for ev in events.mouse_wheel.iter() {
        event_queue.push(IcedEvent::Mouse(iced_native::mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: ev.x, y: ev.y },
        }));
    }

    for ev in events.received_character.iter() {
        event_queue.push(IcedEvent::Keyboard(
            iced_native::keyboard::Event::CharacterReceived(ev.char),
        ));
    }

    for ev in events.keyboard_input.iter() {
        if let Some(code) = ev.key_code {
            let key_code = conversions::key_code(code);
            let modifiers = keyboard::Modifiers::default();
            let ev = if ev.state.is_pressed() {
                keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                }
            } else {
                keyboard::Event::KeyReleased {
                    key_code,
                    modifiers,
                }
            };
            event_queue.push(IcedEvent::Keyboard(ev));
        }
    }
}
