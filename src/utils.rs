use crate::iced;
use crate::IcedContext;
use bevy_math::Vec2;
use bevy_window::Window;

pub(crate) fn process_cursor_position(
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
pub(crate) fn process_touch_input<M: bevy_ecs::event::Event>(
    context: &IcedContext<M>,
) -> Option<iced::Point> {
    context
        .touches
        .first_pressed_position()
        .or(context
            .touches
            .iter_just_released()
            .map(|touch| touch.position())
            .next())
        .map(|Vec2 { x, y }| iced::Point { x, y })
        .or(context
            .events
            .iter()
            .filter_map(|ev| {
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
            .next()
            .copied())
}
