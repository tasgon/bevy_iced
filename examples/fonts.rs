use std::{fs, io};

use bevy::prelude::*;
use bevy_iced::iced::{
    font,
    widget::{column, text},
    Font,
};
use bevy_iced::{iced, IcedContext, IcedPlugin};

const ALPHAPROTA_FONT: Font = Font::with_name("Alpha Prota");

#[derive(Event)]
pub enum UiMessage {}

pub fn main() -> io::Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IcedPlugin {
            fonts: vec![fs::read("assets/fonts/AlphaProta.ttf")?.leak()],
            settings: iced::Settings {
                default_text_size: 40.0,
                default_font: ALPHAPROTA_FONT,
                ..Default::default()
            },
        })
        .add_event::<UiMessage>()
        .add_systems(Update, ui_system)
        .run();

    Ok(())
}

fn ui_system(mut ctx: IcedContext<UiMessage>) {
    ctx.display(column!(
        text(format!("I am the default font")).font(font::Font::DEFAULT),
        text(format!("I am another font"))
    ));
}
