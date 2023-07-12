use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::mouse::{MouseButtonInput, MouseWheel},
    prelude::*,
};
use bevy_iced::{
    widget::{slider, text, text_input, Button, Column, Row},
    IcedContext, IcedPlugin, IcedSettings,
};
use rand::random as rng;

#[derive(Clone, Event)]
enum UiMessage {
    BoxRequested,
    Scale(f32),
    Text(String),
}

#[derive(Resource, Deref, DerefMut)]
pub struct UiActive(bool);

#[derive(Resource)]
pub struct UiData {
    scale: f32,
    text: String,
}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: bevy_window::PresentMode::AutoNoVsync,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(IcedPlugin)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_event::<UiMessage>()
        .insert_resource(UiActive(true))
        .insert_resource(UiData {
            scale: 50.0,
            text: "Welcome to Iced!".to_owned(),
        })
        .insert_resource(IcedSettings {
            scale_factor: None,
            theme: bevy_iced::style::Theme::Light,
            style: bevy_iced::iced::core::renderer::Style {
                text_color: bevy_iced::iced::core::Color::from_rgb(0.0, 1.0, 1.0),
            },
        })
        .add_systems(Startup, build_program)
        .add_systems(
            Update,
            (tick, box_system, update_scale_factor, toggle_ui, ui_system),
        )
        .run();
}

fn build_program(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn tick(mut sprites: Query<&mut Sprite>, time: Res<Time>, data: Res<UiData>) {
    let scale = data.scale;
    for mut s in sprites.iter_mut() {
        s.custom_size = Some(Vec2::new(scale, scale) * time.elapsed_seconds().sin().abs());
    }
}

fn box_system(
    mut commands: Commands,
    mut messages: EventReader<UiMessage>,
    mut data: ResMut<UiData>,
    mut sprites: Query<&mut Sprite>,
) {
    let pos = (Vec3::new(rng(), rng(), 0.0) - Vec3::new(0.5, 0.5, 0.0)) * 300.0;
    for msg in messages.iter() {
        match msg {
            UiMessage::BoxRequested => {
                commands.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba_u8(rng(), rng(), rng(), rng()),
                        custom_size: Some(Vec2::new(50.0, 50.0)),
                        ..Default::default()
                    },
                    transform: Transform::from_translation(pos),
                    ..Default::default()
                });
            }
            UiMessage::Scale(new_scale) => {
                data.scale = *new_scale;
            }
            UiMessage::Text(s) => {
                data.text = s.clone();
                for mut i in &mut sprites.iter_mut() {
                    i.color = Color::rgba_u8(rng(), rng(), rng(), rng());
                }
            }
        }
    }
}

fn update_scale_factor(
    mut wheel: EventReader<MouseWheel>,
    mut iced_settings: ResMut<IcedSettings>,
) {
    if wheel.is_empty() {
        return;
    }
    for event in wheel.iter() {
        let scale_factor =
            (iced_settings.scale_factor.unwrap_or(1.0) + (event.y / 10.0) as f64).max(1.0);
        iced_settings.set_scale_factor(scale_factor);
    }
}

fn toggle_ui(mut buttons: EventReader<MouseButtonInput>, mut ui_active: ResMut<UiActive>) {
    for ev in buttons.iter() {
        if ev.button == MouseButton::Right {
            **ui_active = !**ui_active;
        }
    }
}

fn ui_system(
    mut ctx: IcedContext<UiMessage>,
    data: Res<UiData>,
    sprites: Query<(&Sprite,)>,
    ui_active: Res<UiActive>,
) {
    if !**ui_active {
        return;
    }

    let row = Row::new()
        .spacing(10)
        .align_items(bevy_iced::iced::core::Alignment::Center)
        .push(Button::new(text("Request box")).on_press(UiMessage::BoxRequested))
        .push(text(format!(
            "{} boxes (amplitude: {})",
            sprites.iter().len(),
            data.scale
        )));
    let edit = text_input("", &data.text).on_input(UiMessage::Text);
    let column = Column::new()
        .align_items(bevy_iced::iced::core::Alignment::Center)
        .spacing(10)
        .push(edit)
        .push(slider(0.0..=100.0, data.scale, UiMessage::Scale))
        .push(row);
    ctx.display(column);
}
