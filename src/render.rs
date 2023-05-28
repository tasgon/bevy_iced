use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::Query;
use bevy_ecs::{
    system::{Commands, Res, Resource},
    world::World,
};
use bevy_render::renderer::{RenderDevice, RenderQueue};
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext},
    renderer::RenderContext,
    view::ExtractedWindows,
    Extract,
};
use bevy_window::Window;
use iced_runtime::core::Size;
use iced_renderer::Backend;
use iced_wgpu::graphics::Viewport;

use crate::{DidDraw, IcedProps, IcedResource, IcedSettings};

pub const ICED_PASS: &str = "bevy_iced_pass";

#[derive(Resource, Deref, DerefMut, Clone)]
pub struct ViewportResource(pub Viewport);

pub(crate) fn update_viewport(
    windows: Query<&Window>,
    iced_settings: Res<IcedSettings>,
    mut commands: Commands,
) {
    let window = windows.single();
    let scale_factor = iced_settings.scale_factor.unwrap_or(window.scale_factor());
    let viewport = Viewport::with_physical_size(
        Size::new(window.physical_width(), window.physical_height()),
        scale_factor,
    );
    commands.insert_resource(ViewportResource(viewport));
}

// Same as DidDraw, but as a regular bool instead of an atomic.
#[derive(Resource, Deref, DerefMut)]
struct DidDrawBasic(bool);

pub(crate) fn extract_iced_data(
    mut commands: Commands,
    viewport: Extract<Res<ViewportResource>>,
    did_draw: Extract<Res<DidDraw>>,
) {
    commands.insert_resource(viewport.clone());
    commands.insert_resource(DidDrawBasic(
        did_draw.swap(false, std::sync::atomic::Ordering::Relaxed),
    ));
}

pub struct IcedNode;

impl Node for IcedNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some(extracted_window) = world
            .get_resource::<ExtractedWindows>()
            .unwrap()
            .windows
            .values()
            .next() else { return Ok(()) };

        let IcedProps {
            renderer, debug, ..
        } = &mut *world.resource::<IcedResource>().lock().unwrap();
        let render_device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();

        if !world
            .get_resource::<DidDrawBasic>()
            .map(|x| x.0)
            .unwrap_or(false)
        {
            return Ok(());
        }

        let view = extracted_window.swap_chain_texture.as_ref().unwrap();

        let viewport = world.resource::<ViewportResource>();
        let device = render_device.wgpu_device();

        renderer.with_primitives(|backend, primitives| {
            let Backend::Wgpu(ref mut backend) = backend else { return; };
            backend.present(
                device,
                queue,
                render_context.command_encoder(),
                None,
                view,
                primitives,
                viewport,
                &debug.overlay(),
            );
        });

        Ok(())
    }
}
