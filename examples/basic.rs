use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
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
    App::new()
        .insert_resource(WindowDescriptor {
            vsync: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(IcedPlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .insert_program(MainUi::default())
        .add_startup_system(build_program)
        .add_system(tick)
        .run();
}

fn build_program(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..Default::default()
        },
        ..Default::default()
    });
}

pub fn tick(mut sprites: Query<(&mut Sprite,)>, time: Res<Time>) {
    for (mut s,) in sprites.iter_mut() {
        s.custom_size =
            Some(Vec2::new(50.0, 50.0) * time.time_since_startup().as_secs_f32().sin().abs());
    }
}
