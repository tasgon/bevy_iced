use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_render::render_graph::{RenderLabel, ViewNode};
use bevy_render::view::ViewTarget;
use bevy_render::{
    Extract,
    render_graph::{NodeRunError, RenderGraphContext},
    renderer::RenderContext,
};
use bevy_window::prelude::*;
use cfg_if::cfg_if;
use iced_core::Size;
use iced_wgpu::wgpu::TextureFormat;
use iced_widget::graphics::Viewport;

use crate::systems::IcedCamera;
use crate::{DidDraw, IcedProps, IcedResource, IcedSettings};

pub const TEXTURE_FMT: TextureFormat = TextureFormat::Rgba8UnormSrgb; // must be equal to the format of the iced 2D camera

#[derive(Resource, Deref, DerefMut, Clone)]
pub struct IcedViewport(pub Viewport);

pub fn update_viewport(
    windows: Query<&Window>,
    iced_settings: Res<IcedSettings>,
    mut commands: Commands,
) {
    let window = windows.single().unwrap();
    let scale_factor = iced_settings
        .scale_factor
        .unwrap_or_else(|| window.scale_factor().into());
    let viewport = Viewport::with_physical_size(
        Size::new(window.physical_width(), window.physical_height()),
        scale_factor,
    );
    commands.insert_resource(IcedViewport(viewport));
}

// Same as DidDraw, but as a regular bool instead of an atomic.
#[derive(Resource, Deref, DerefMut)]
struct DidDrawBasic(bool);

pub fn extract_iced_data(
    mut commands: Commands,
    viewport: Extract<Res<IcedViewport>>,
    did_draw: Extract<Res<DidDraw>>,
) {
    commands.insert_resource(viewport.clone());
    commands.insert_resource(DidDrawBasic(
        did_draw.swap(false, std::sync::atomic::Ordering::Relaxed),
    ));
}

pub fn recall_staging_belt(
    #[cfg(target_arch = "wasm32")] iced: NonSend<IcedResource>,
    #[cfg(not(target_arch = "wasm32"))] iced: Res<IcedResource>,
) {
    iced.lock().renderer.staging_belt_recall();
}

#[derive(Clone, Hash, Default, Debug, Eq, PartialEq, RenderLabel)]
pub struct IcedPass;

impl ViewNode for IcedPass {
    type ViewQuery = (&'static ViewTarget, Has<IcedCamera>);
    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (target, is_iced): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        if !is_iced {
            return Ok(());
        }

        if !world.get_resource::<DidDrawBasic>().is_some_and(|x| x.0) {
            return Ok(());
        }

        let texture_view = target.main_texture_view();
        let viewport = world.resource::<IcedViewport>();

        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let IcedProps {
                    renderer,
                    ..
                } = &mut *world.non_send_resource::<IcedResource>().lock();
            } else {
                let IcedProps {
                    renderer,
                    ..
                } = &mut *world.resource::<IcedResource>().lock();
            }
        };

        let encoder = renderer.draw(None, texture_view, viewport);
        render_context.add_command_buffer(encoder.finish());
        renderer.staging_belt_finish();

        Ok(())
    }
}
