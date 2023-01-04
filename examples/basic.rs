use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::mouse::MouseWheel,
    prelude::*,
};
use bevy_iced::{
    iced::{
        widget::{Button, Row},
    },
    IcedSettings,
};
use bevy_iced::{IcedPlugin};
use bevy_inspector_egui::WorldInspectorPlugin;

use iced_native::widget::{text};
use rand::random as rng;

#[derive(Debug, Clone)]
pub enum UiMessage {
    BoxRequested,
    BoxAdded,
    Rescaled(f64),
}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                // present_mode: bevy::window::PresentMode::AutoNoVsync,
                ..Default::default()
            },
            ..Default::default()
        }))
        .add_plugin(IcedPlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(WorldInspectorPlugin::new())
        .init_resource::<Events<UiMessage>>()
        .add_startup_system(build_program)
        .add_system(tick)
        .add_system(box_system)
        .add_system(update_scale_factor)
        .add_system(ui_system)
        .run();
}

fn build_program(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn tick(mut sprites: Query<(&mut Sprite,)>, time: Res<Time>) {
    for (mut s,) in sprites.iter_mut() {
        s.custom_size = Some(Vec2::new(50.0, 50.0) * time.elapsed_seconds().sin().abs());
    }
}

pub fn box_system(mut commands: Commands, mut messages: EventReader<UiMessage>) {
    let pos = (Vec3::new(rng(), rng(), 0.0) - Vec3::new(0.5, 0.5, 0.0)) * 300.0;
    for msg in messages.iter() {
        println!("Msg: {msg:?}");
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
            _ => {}
        }
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

// pub fn toggle_ui(
//     mut commands: Commands,
//     mut buttons: EventReader<MouseButtonInput>,
//     mut render_state: Option<ResMut<IcedRenderState<MainUi>>>,
// ) {
//     for ev in buttons.iter() {
//         if ev.button == MouseButton::Right {
//             if let Some(ref mut state) = render_state {
//                 state.active = !state.active;
//             } else {
//                 commands.insert_resource(IcedRenderState::<MainUi>::active(false));
//             }
//         }
//     }
// }

pub fn ui_system(mut ctx: bevy_iced::Context<UiMessage>, sprites: Query<(&Sprite,)>) {
    let row = Row::new()
            .push(Button::new(text("Request box")).on_press(UiMessage::BoxRequested))
            .push(text(format!("{} boxes", sprites.iter().len())));
    ctx.show(row);
        // Column::new()
        //     .push(row)
        //     .push(Text::new(format!("Scale factor: {}", self.scale_factor)))
        //     .into()
}
