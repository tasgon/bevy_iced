use bevy::prelude::*;
use bevy_iced::iced::{
    widget::{column, text},
    Font,
};
use bevy_iced::{IcedContext, IcedPlugin};

static ALPHAPROTA_FONT_BYTES: &'static [u8] = include_bytes!("../assets/fonts/AlphaProta.ttf");
static RAINBOW2000_FONT_BYTES: &'static [u8] =
    include_bytes!("../assets/fonts/Rainbow2000-Regular.ttf");

const ALPHAPROTA_FONT: Font = Font::with_name("Alpha Prota");
const RAINBOW2000_FONT: Font = Font::with_name("Rainbow 2000");

#[derive(Event)]
pub enum UiMessage {}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IcedPlugin {
            fonts: vec![ALPHAPROTA_FONT_BYTES, RAINBOW2000_FONT_BYTES],
            default_font: ALPHAPROTA_FONT,
            default_text_size: 40.0,
            ..Default::default()
        })
        .add_event::<UiMessage>()
        .add_systems(Update, ui_system)
        .run();
}

fn ui_system(mut ctx: IcedContext<UiMessage>) {
    ctx.display(column!(
        text(format!("I am the default font")),
        text(format!("I am another font")).font(RAINBOW2000_FONT)
    ));
}
