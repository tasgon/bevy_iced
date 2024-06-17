use bevy::prelude::*;
use bevy_iced::iced::{
    font,
    widget::{column, text},
    Font,
};
use bevy_iced::{iced, IcedContext, IcedPlugin};

const ALPHAPROTA_FONT: Font = Font::with_name("Alpha Prota");
const ALPHAPROTA_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/AlphaProta.ttf");

#[derive(Event)]
pub enum UiMessage {}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IcedPlugin {
            fonts: vec![ALPHAPROTA_FONT_BYTES],
            settings: iced::Settings {
                default_text_size: 40.0.into(),
                default_font: ALPHAPROTA_FONT,
                ..Default::default()
            },
        })
        .add_event::<UiMessage>()
        .add_systems(Update, ui_system)
        .run();
}

fn ui_system(mut ctx: IcedContext<UiMessage>) {
    ctx.display(column!(
        text("I am the default font".to_string()).font(font::Font::DEFAULT),
        text("I am another font".to_string())
    ));
}
