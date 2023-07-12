use bevy::prelude::*;
use bevy_iced::iced::core::widget::text;
use bevy_iced::{IcedContext, IcedPlugin};

#[derive(Event)]
pub enum UiMessage {}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IcedPlugin)
        .add_event::<UiMessage>()
        .add_systems(Update, ui_system)
        .run();
}

fn ui_system(time: Res<Time>, mut ctx: IcedContext<UiMessage>) {
    ctx.display(text::Text::new(format!(
        "Hello Iced! Running for {:.2} seconds.",
        time.elapsed_seconds()
    )));
}
