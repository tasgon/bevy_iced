use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_iced::iced::{
    program::State,
    widget::{button, Button, Row, Text},
    Element, Program,
};
use bevy_iced::{IcedAppExtensions, IcedPlugin};
use bevy_inspector_egui::WorldInspectorPlugin;

use rand::random as rng;

#[derive(Debug, Clone)]
pub enum UiMessage {
    BoxRequested,
    BoxAdded,
}

#[derive(Default)]
pub struct MainUi {
    btn: button::State,
    pub count: u32,
    pub box_requested: bool,
}

impl Program for MainUi {
    type Renderer = bevy_iced::iced_wgpu::Renderer;
    type Message = UiMessage;

    fn update(&mut self, msg: UiMessage) -> iced_native::Command<UiMessage> {
        match msg {
            UiMessage::BoxRequested => self.box_requested = true,
            UiMessage::BoxAdded => {
                self.box_requested = false;
                self.count += 1;
            }
        }
        iced_native::Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message, Self::Renderer> {
        Row::new()
            .push(
                Button::new(&mut self.btn, Text::new("Request box"))
                    .on_press(UiMessage::BoxRequested),
            )
            .push(Text::new(format!("{} boxes", self.count)))
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
        .add_plugin(WorldInspectorPlugin::new())
        .insert_program(MainUi::default())
        .add_startup_system(build_program)
        .add_system(tick)
        .add_system(box_system)
        .run();
}

fn build_program(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

pub fn tick(mut sprites: Query<(&mut Sprite,)>, time: Res<Time>) {
    for (mut s,) in sprites.iter_mut() {
        s.custom_size =
            Some(Vec2::new(50.0, 50.0) * time.time_since_startup().as_secs_f32().sin().abs());
    }
}

pub fn box_system(mut commands: Commands, mut program: NonSendMut<State<MainUi>>) {
    let pos = (Vec3::new(rng(), rng(), 0.0) - Vec3::new(0.5, 0.5, 0.0)) * 300.0;
    if program.program().box_requested {
        commands.spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba_u8(rng(), rng(), rng(), rng()),
                custom_size: Some(Vec2::new(50.0, 50.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(pos),
            ..Default::default()
        });
        program.queue_message(UiMessage::BoxAdded);
    }
}
