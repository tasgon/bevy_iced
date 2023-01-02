use std::{cell::RefCell, sync::Mutex};

use bevy::{
    prelude::{Commands, Deref, DerefMut, Res, Resource},
    render::{render_graph::Node, render_resource::TextureView, view::ExtractedWindows, Extract},
    window::Windows,
};
use iced_native::Size;
use iced_wgpu::{
    wgpu::{self, util::StagingBelt},
    Viewport,
};

use crate::DrawFn;

pub const ICED_PASS: &'static str = "bevy_iced_pass";

/// Settings used to independently customize Iced rendering.
#[derive(Clone, Resource)]
pub struct IcedSettings {
    /// The scale factor to use for rendering Iced windows.
    pub scale_factor: f64,
}

#[derive(Resource, Deref, DerefMut, Clone)]
pub(crate) struct ViewportResource(pub Viewport);

pub(crate) fn update_viewport(
    windows: Res<Windows>,
    iced_settings: Option<Res<IcedSettings>>,
    mut commands: Commands,
) {
    let window = windows.get_primary().unwrap();
    let scale_factor = if let Some(settings) = iced_settings {
        settings.scale_factor
    } else {
        window.scale_factor()
    };
    let viewport = Viewport::with_physical_size(
        Size::new(window.physical_width(), window.physical_height()),
        scale_factor,
    );
    commands.insert_resource(ViewportResource(viewport));
}

pub(crate) fn extract_iced_data(mut commands: Commands, viewport: Extract<Res<ViewportResource>>) {
    commands.insert_resource(viewport.clone());
}

pub struct IcedRenderData<'a> {
    pub view: &'a TextureView,
    pub staging_belt: &'a mut wgpu::util::StagingBelt,
}

pub struct IcedNode {
    staging_belt: Mutex<StagingBelt>,
}

impl IcedNode {
    pub fn new() -> Self {
        Self {
            staging_belt: Mutex::new(StagingBelt::new(5 * 1024)),
        }
    }
}

impl Node for IcedNode {
    fn update(&mut self, _world: &mut bevy::prelude::World) {
        self.staging_belt.lock().unwrap().recall()
    }

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let draw_fns = world
            .get_non_send_resource::<RefCell<Vec<DrawFn>>>()
            .unwrap();

        let viewport = world.get_resource::<ViewportResource>().unwrap();

        let Some(extracted_window) = world
            .get_resource::<ExtractedWindows>()
            .unwrap()
            .windows
            .values()
            .next() else { return Ok(()) };
        let swap_chain_texture = extracted_window.swap_chain_texture.as_ref().unwrap();
        let staging_belt = &mut *self.staging_belt.lock().unwrap();

        let mut render_data = IcedRenderData {
            view: &swap_chain_texture,
            staging_belt,
        };
        for f in &mut *draw_fns.borrow_mut() {
            (f)(world, render_context, viewport, &mut render_data);
        }
        staging_belt.finish();

        Ok(())
    }
}
