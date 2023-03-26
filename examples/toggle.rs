use bevy::prelude::*;
use bevy_iced::iced::widget::text;
use bevy_iced::{IcedContext, IcedPlugin};
use bevy_input::keyboard::KeyboardInput;
use bevy_input::ButtonState;

#[derive(Debug)]
pub enum UiMessage {}

#[derive(Resource, PartialEq, Eq)]
pub struct UiActive(bool);

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(IcedPlugin)
        .add_event::<UiMessage>()
        .insert_resource(UiActive(true))
        .add_system(toggle_system)
        .add_system(ui_system.run_if(resource_equals(UiActive(true))))
        .run();
}

fn toggle_system(mut keyboard: EventReader<KeyboardInput>, mut active: ResMut<UiActive>) {
    for event in keyboard.iter() {
        if event.key_code == Some(KeyCode::Space) && event.state == ButtonState::Pressed {
            active.0 = !active.0;
        }
    }
}

fn ui_system(mut ctx: IcedContext<UiMessage>) {
    ctx.display(text("Press space to toggle GUI."));
}
