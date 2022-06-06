use std::{cell::RefCell, sync::Mutex};

use bevy::render::{render_graph::Node, render_resource::TextureView, view::ExtractedWindows};
use iced_native::{
    futures::{executor::LocalPool, task::SpawnExt},
    Size,
};
use iced_wgpu::{
    wgpu::{self, util::StagingBelt},
    Viewport,
};

use crate::DrawFn;

pub const ICED_PASS: &'static str = "bevy_iced_pass";

pub struct IcedRenderData<'a> {
    pub view: &'a TextureView,
    pub staging_belt: &'a mut wgpu::util::StagingBelt,
}

pub struct IcedNode {
    size: wgpu::Extent3d,
    viewport: Viewport,
    staging_belt: Mutex<StagingBelt>,
}

impl IcedNode {
    pub fn new() -> Self {
        Self {
            size: Default::default(),
            viewport: Viewport::with_physical_size(Size::new(100, 100), 1.0),
            staging_belt: Mutex::new(StagingBelt::new(5 * 1024)),
        }
    }
}

impl Node for IcedNode {
    fn update(&mut self, world: &mut bevy::prelude::World) {
        let window = world
            .get_resource::<ExtractedWindows>()
            .unwrap()
            .values()
            .next()
            .unwrap();
        let size = wgpu::Extent3d {
            width: window.physical_width,
            height: window.physical_height,
            depth_or_array_layers: 1,
        };

        if self.size != size {
            self.viewport = Viewport::with_physical_size(Size::new(size.width, size.height), 2.0);
        }

        let mut pool = LocalPool::new();
        pool.spawner()
            .spawn(self.staging_belt.lock().unwrap().recall())
            .unwrap();
        pool.run_until_stalled();
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

        let extracted_window = &world
            .get_resource::<ExtractedWindows>()
            .unwrap()
            .windows
            .values()
            .next()
            .unwrap();
        let swap_chain_texture = extracted_window.swap_chain_texture.as_ref().unwrap();
        let staging_belt = &mut *self.staging_belt.lock().unwrap();

        let mut render_data = IcedRenderData {
            view: &swap_chain_texture,
            staging_belt,
        };
        for f in &mut *draw_fns.borrow_mut() {
            (f)(world, render_context, &self.viewport, &mut render_data);
        }
        staging_belt.finish();

        Ok(())
    }
}
