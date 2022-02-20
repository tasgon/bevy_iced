use crate::conversions;
use bevy::ecs::query::WorldQuery;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::{Query, ResMut};
use bevy::sprite::Sprite;
use bevy::{
    ecs::system::SystemParam,
    input::{keyboard::KeyboardInput, mouse::MouseWheel},
    prelude::{EventReader, NonSend, World},
    window::{
        CursorEntered, CursorLeft, CursorMoved, ReceivedCharacter, WindowCreated, WindowFocused,
        WindowResized,
    },
};
use iced_native::{keyboard, mouse, Event as IcedEvent, Point};

#[derive(SystemParam)]
pub struct InputEvents<'w, 's> {
    cursor_entered: EventReader<'w, 's, CursorEntered>,
    cursor_left: EventReader<'w, 's, CursorLeft>,
    cursor: EventReader<'w, 's, CursorMoved>,
    mouse_button: EventReader<'w, 's, MouseButtonInput>,
    mouse_wheel: EventReader<'w, 's, MouseWheel>,
    received_character: EventReader<'w, 's, ReceivedCharacter>,
    keyboard_input: EventReader<'w, 's, KeyboardInput>,
    window_focused: EventReader<'w, 's, WindowFocused>,
    window_created: EventReader<'w, 's, WindowCreated>,
    window_resized: EventReader<'w, 's, WindowResized>,
}

pub fn process_input(mut events: InputEvents, mut event_queue: ResMut<Vec<IcedEvent>>) {
    event_queue.clear();

    for ev in events.cursor.iter() {
        event_queue.push(IcedEvent::Mouse(mouse::Event::CursorMoved {
            position: Point::new(ev.position.x, ev.position.y),
        }));
    }

    for ev in events.mouse_button.iter() {
        let button = conversions::mouse_button(ev.button);
        event_queue.push(IcedEvent::Mouse(match ev.state {
            bevy::input::ElementState::Pressed => iced_native::mouse::Event::ButtonPressed(button),
            bevy::input::ElementState::Released => {
                iced_native::mouse::Event::ButtonReleased(button)
            }
        }))
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
