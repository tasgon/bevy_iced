use bevy::prelude::*;
use bevy_iced::iced::widget::text;
use bevy_iced::{IcedContext, IcedPlugin, IcedProgramSet, iced};

const NOTOSANS_REGULAR: iced::Font = iced::Font::with_name("Noto Sans");
const NOTOSANS_REGULAR_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");

#[derive(Event)]
pub enum UiMessage {}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(
            IcedPlugin::<UiMessage>::default()
                .fonts(vec![NOTOSANS_REGULAR_BYTES])
                .settings(iced::Settings {
                    default_font: NOTOSANS_REGULAR,
                    ..Default::default()
                }),
        )
        .add_event::<UiMessage>()
        .add_systems(Update, ui_system.in_set(IcedProgramSet::View))
        .run();
}

fn ui_system(time: Res<Time>, mut ctx: IcedContext<UiMessage>) {
    ctx.display(text(format!(
        "Hello Iced! Running for {:.2} seconds.",
        time.elapsed_secs()
    )));
}
