use bevy::prelude::*;
use bevy_iced::{IcedAppExtensions, IcedPlugin};
use iced_native::{
    widget::{button, Button, Row, Text},
    Element, Program,
};

#[derive(Default)]
pub struct MainUi {
    btn: button::State,
    pub count: u32,
}

impl Program for MainUi {
    type Renderer = iced_wgpu::Renderer;
    type Message = ();

    fn update(&mut self, _: ()) -> iced_native::Command<()> {
        self.count += 1;
        iced_native::Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message, Self::Renderer> {
        Row::new()
            .push(Button::new(&mut self.btn, Text::new("Click me!")).on_press(()))
            .push(Text::new(format!("Clicked {} times", self.count)))
            .into()
    }
}

pub fn main() {
    let app = App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(IcedPlugin)
        .insert_program(MainUi::default())
        .run();
}

fn build_program(mut commands: Commands) {
    commands;
}
