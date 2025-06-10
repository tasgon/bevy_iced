use bevy_input::prelude::*;
use bevy_math::prelude::*;
use bevy_window::prelude::*;

use crate::iced;
use crate::systems::IcedEventQueue;

pub fn process_cursor_position(
    position: Vec2,
    bounds: iced_core::Size,
    window: &Window,
) -> iced_core::Point {
    iced_core::Point {
        x: position.x * bounds.width / window.width(),
        y: position.y * bounds.height / window.height(),
    }
}

/// To correctly process input as last resort events are used
pub fn process_touch_input(touches: &Touches, events: &IcedEventQueue) -> Option<iced::Point> {
    touches
        .first_pressed_position()
        .or_else(|| {
            touches
                .iter_just_released()
                .map(bevy_input::touch::Touch::position)
                .next()
        })
        .map(|Vec2 { x, y }| iced::Point { x, y })
        .or_else(|| {
            events
                .iter()
                .find_map(|ev| {
                    if let iced::Event::Touch(
                        iced::touch::Event::FingerLifted { position, .. }
                        | iced::touch::Event::FingerLost { position, .. }
                        | iced::touch::Event::FingerMoved { position, .. }
                        | iced::touch::Event::FingerPressed { position, .. },
                    ) = ev
                    {
                        Some(position)
                    } else {
                        None
                    }
                })
                .copied()
        })
}
