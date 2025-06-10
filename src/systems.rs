use bevy_core_pipeline::core_2d::Camera2d;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_input::prelude::*;
use bevy_input::touch::TouchInput;
use bevy_input::{
    ButtonState,
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseWheel},
};
use bevy_render::camera::Camera;
use bevy_render::extract_component::ExtractComponent;
use bevy_window::prelude::*;
use bevy_window::{PrimaryWindow, WindowFocused};
use iced_core::window::Event as IcedWindowEvent;
use iced_core::{
    Event as IcedEvent, Point, Theme, keyboard,
    mouse::{self, Cursor},
};
use iced_runtime::UserInterface;

use crate::redraw_requestor::IcedRedrawRequest;
use crate::{
    IcedProps, IcedSettings, Renderer, conversions, iced_resource::IcedResource,
    render::IcedViewport, utils,
};

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IcedEventQueue(Vec<iced_core::Event>);

#[derive(SystemParam)]
pub struct InputEvents<'w, 's> {
    cursor_entered: EventReader<'w, 's, CursorEntered>,
    cursor_left: EventReader<'w, 's, CursorLeft>,
    cursor: EventReader<'w, 's, CursorMoved>,
    mouse_button: EventReader<'w, 's, MouseButtonInput>,
    mouse_wheel: EventReader<'w, 's, MouseWheel>,
    keyboard_input: EventReader<'w, 's, KeyboardInput>,
    touch_input: EventReader<'w, 's, TouchInput>,
    window_focused: EventReader<'w, 's, WindowFocused>,
}

fn compute_modifiers(input_map: &ButtonInput<KeyCode>) -> keyboard::Modifiers {
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
    input_map: Res<ButtonInput<KeyCode>>,
) {
    event_queue.clear();

    for ev in events.cursor.read() {
        event_queue.push(IcedEvent::Mouse(mouse::Event::CursorMoved {
            position: Point::new(ev.position.x, ev.position.y),
        }));
    }

    for ev in events.mouse_button.read() {
        let button = conversions::mouse_button(ev.button);
        event_queue.push(IcedEvent::Mouse(match ev.state {
            ButtonState::Pressed => iced_core::mouse::Event::ButtonPressed(button),
            ButtonState::Released => iced_core::mouse::Event::ButtonReleased(button),
        }));
    }

    for _ev in events.cursor_entered.read() {
        event_queue.push(IcedEvent::Mouse(iced_core::mouse::Event::CursorEntered));
    }

    for _ev in events.cursor_left.read() {
        event_queue.push(IcedEvent::Mouse(iced_core::mouse::Event::CursorLeft));
    }

    for ev in events.mouse_wheel.read() {
        event_queue.push(IcedEvent::Mouse(iced_core::mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: ev.x, y: ev.y },
        }));
    }

    let modifiers = compute_modifiers(&input_map);

    for ev in events.keyboard_input.read() {
        use keyboard::Event::*;
        let event = match ev.key_code {
            KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight => ModifiersChanged(modifiers),
            _ => {
                let key = conversions::key(&ev.logical_key);
                let physical_key = conversions::key_code(ev.key_code);
                if ev.state.is_pressed() {
                    KeyPressed {
                        // NOTE: This is supposed to be the "unmodified" key, but we don't get it from bevy events
                        key: key.clone(),
                        text: conversions::key_text(&key),
                        physical_key,
                        modified_key: key,
                        modifiers,
                        // NOTE: This is a winit thing we don't get from bevy events
                        location: keyboard::Location::Standard,
                    }
                } else {
                    KeyReleased {
                        key: key.clone(),
                        modified_key: key,
                        physical_key,
                        modifiers,
                        // NOTE: This is a winit thing we don't get from bevy events
                        location: keyboard::Location::Standard,
                    }
                }
            }
        };

        event_queue.push(IcedEvent::Keyboard(event));
    }

    for ev in events.touch_input.read() {
        event_queue.push(IcedEvent::Touch(conversions::touch_event(ev)));
    }

    for ev in events.window_focused.read() {
        event_queue.push(IcedEvent::Window(if ev.focused {
            IcedWindowEvent::Focused
        } else {
            IcedWindowEvent::Unfocused
        }));
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IcedCursor(Cursor);

pub fn iced_update<M: bevy_ecs::event::Event>(
    (viewport, windows): (Res<IcedViewport>, Query<&mut Window, With<PrimaryWindow>>),
    #[cfg(target_arch = "wasm32")] props: NonSend<IcedResource>,
    #[cfg(not(target_arch = "wasm32"))] props: Res<IcedResource>,
    (mut events, touches): (ResMut<IcedEventQueue>, Res<Touches>),
    mut ui: NonSendMut<Option<UserInterface<'static, M, Theme, Renderer>>>,
    mut message_writer: EventWriter<M>,
    mut cursor: ResMut<IcedCursor>,
    mut iced_redraw_request: ResMut<IcedRedrawRequest>,
) {
    let bounds = viewport.logical_size();
    let &mut IcedProps {
        ref mut renderer, ..
    } = &mut *props.lock();
    *cursor = IcedCursor({
        let window = windows.single().unwrap();
        match window.cursor_position() {
            Some(position) => {
                Cursor::Available(utils::process_cursor_position(position, bounds, window))
            }
            None => utils::process_touch_input(&touches, &events)
                .map(Cursor::Available)
                .unwrap_or(Cursor::Unavailable),
        }
    });
    let Some(ui) = ui.as_mut() else { return };

    let mut messages = Vec::<M>::new();
    let (state, _event_statuses) = ui.update(
        events.as_slice(),
        **cursor,
        renderer,
        &mut iced_core::clipboard::Null,
        &mut messages,
    );
    events.clear();
    iced_redraw_request.update(state);
    message_writer.write_batch(messages);
}

/// Marker component to differentiate between normal 2D cameras and the iced camera.
#[derive(Default, Component, ExtractComponent, Copy, Clone)]
pub struct IcedCamera;

/// Spawns a 2D camera, which serves as the render target for iced.
pub fn setup_iced_camera(mut commands: Commands, settings: Res<IcedSettings>) {
    commands.spawn((
        Camera {
            order: settings.camera_order,
            ..Default::default()
        },
        Camera2d,
        IcedCamera,
    ));
}
