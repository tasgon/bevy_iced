use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::mouse::{MouseButtonInput, MouseWheel},
    prelude::*,
};
use bevy_iced::{
    iced::{
        program::State,
        widget::{button, Button, Row, Text},
        Element, Program,
    },
    IcedRenderState, IcedSettings,
};
use bevy_iced::{IcedAppExtensions, IcedPlugin};
use bevy_inspector_egui::WorldInspectorPlugin;

use iced_native::widget::Column;
use rand::random as rng;

#[derive(Debug, Clone)]
pub enum UiMessage {
    BoxRequested,
    BoxAdded,
    Rescaled(f64),
}

#[derive(Default)]
pub struct MainUi {
    btn: button::State,
    pub count: u32,
    pub box_requested: bool,
    pub scale_factor: f64,
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
            UiMessage::Rescaled(factor) => {
                self.scale_factor = factor;
            }
        }
        iced_native::Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        let row = Row::new()
            .push(
                Button::new(Text::new("Request box"))
                    .on_press(UiMessage::BoxRequested),
            )
            .push(Text::new(format!("{} boxes", self.count)));
        Column::new()
            .push(row)
            .push(Text::new(format!("Scale factor: {}", self.scale_factor)))
            .into()
    }
}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(IcedPlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(WorldInspectorPlugin::new())
        .insert_program(MainUi::default())
        .add_startup_system(build_program)
        .add_system(tick)
        .add_system(box_system)
        .add_system(update_scale_factor)
        .add_system(update_ui_scale_data)
        .add_system(toggle_ui)
        .run();
}

fn build_program(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn tick(mut sprites: Query<(&mut Sprite,)>, time: Res<Time>) {
    for (mut s,) in sprites.iter_mut() {
        s.custom_size =
            Some(Vec2::new(50.0, 50.0) * time.elapsed_seconds().sin().abs());
    }
}

pub fn box_system(mut commands: Commands, program: Option<NonSendMut<State<MainUi>>>) {
    if program.is_none() {
        return;
    }
    let mut program = program.unwrap();
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

pub fn update_scale_factor(
    mut commands: Commands,
    windows: Res<Windows>,
    mut wheel: EventReader<MouseWheel>,
    iced_settings: Option<ResMut<IcedSettings>>,
) {
    if wheel.is_empty() {
        return;
    }
    if let Some(mut settings) = iced_settings {
        for event in wheel.iter() {
            settings.scale_factor = (settings.scale_factor + (event.y / 10.0) as f64).max(1.0);
        }
    } else {
        commands.insert_resource(IcedSettings {
            scale_factor: windows.primary().scale_factor(),
        });
    }
}

pub fn toggle_ui(
    mut commands: Commands,
    mut buttons: EventReader<MouseButtonInput>,
    mut render_state: Option<ResMut<IcedRenderState<MainUi>>>,
) {
    for ev in buttons.iter() {
        if ev.button == MouseButton::Right {
            if let Some(ref mut state) = render_state {
                state.active = !state.active;
            } else {
                commands.insert_resource(IcedRenderState::<MainUi>::active(false));
            }
        }
    }
}

pub fn update_ui_scale_data(
    windows: Res<Windows>,
    program: Option<NonSendMut<State<MainUi>>>,
    iced_settings: Option<ResMut<IcedSettings>>,
) {
    if program.is_none() {
        return;
    }
    let scale_factor = iced_settings
        .map(|x| x.scale_factor)
        .unwrap_or(windows.primary().scale_factor());
    program
        .unwrap()
        .queue_message(UiMessage::Rescaled(scale_factor));
}
