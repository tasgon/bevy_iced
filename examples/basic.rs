use bevy::prelude::*;
use bevy_iced::iced::widget::text;
use bevy_iced::{IcedContext, IcedPlugin};

#[derive(Debug)]
pub enum UiMessage {}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(IcedPlugin)
        .add_event::<UiMessage>()
        .add_system(ui_system)
        .run();
}

fn ui_system(time: Res<Time>, mut ctx: IcedContext<UiMessage>) {
    ctx.show(text(format!(
        "Hello Iced! Running for {:.2} seconds.",
        time.elapsed_seconds()
    )));
}
