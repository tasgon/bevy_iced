use std::{cell::RefCell, sync::Arc};

use crate::render::IcedNode;
use bevy::render::render_graph::RenderGraph;
use bevy::{
    prelude::{App, Plugin, World},
    render::{
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::ExtractedWindows,
        RenderApp,
    },
};
use iced_native::{program, Debug, Program, Size};
use iced_native::futures::executor::LocalPool;
use iced_native::futures::task::SpawnExt;
use iced_wgpu::{
    wgpu::{self, util::StagingBelt, CommandEncoderDescriptor},
    Viewport,
};

pub type IcedState<T> = Arc<RefCell<program::State<T>>>;

mod conversions;
mod render;
mod systems;
pub struct IcedPlugin;

impl Plugin for IcedPlugin {
    fn build(&self, app: &mut App) {
        app //.add_system(systems::process_input)
            .insert_non_send_resource(Vec::<UpdateFn>::new());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_non_send_resource(RefCell::new(Vec::<DrawFn>::new()));
        setup_pipeline(&mut render_app.world.get_resource_mut().unwrap());
    }
}

type UpdateFn = Box<dyn Fn(&mut World, &iced_native::Event)>;
type DrawFn = Box<dyn FnMut(&World, &mut RenderContext)>;

pub trait IcedAppExtensions {
    fn insert_program<M, T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static>(
        &mut self,
        program: T,
    ) -> &mut Self;
}

impl IcedAppExtensions for App {
    fn insert_program<M, T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static>(
        &mut self,
        program: T,
    ) -> &mut Self {
        let device = self
            .sub_app(RenderApp)
            .world
            .get_resource::<RenderDevice>()
            .unwrap()
            .wgpu_device();
        let format = wgpu::TextureFormat::bevy_default();
        let mut renderer =
            iced_wgpu::Renderer::new(iced_wgpu::Backend::new(device, Default::default(), format));
        let viewport = Viewport::with_physical_size(Size::new(100, 100), 1.0);
        let mut debug = Debug::new();
        let program =
            program::State::new(program, viewport.logical_size(), &mut renderer, &mut debug);
        let mut staging_belt = StagingBelt::new(5 * 1024);
        let pool = LocalPool::new();

        let update_fn: UpdateFn = Box::new(move |world: &mut World, event: &iced_native::Event| {
            let mut state = world
                .get_non_send_resource_mut::<program::State<T>>()
                .unwrap();
            state.queue_event(event.clone());
        });
        self.world
            .get_non_send_resource_mut::<Vec<UpdateFn>>()
            .unwrap()
            .push(update_fn);
        
        let draw_fn: DrawFn = Box::new(move |world: &World, ctx: &mut RenderContext| {
            let device = ctx.render_device.wgpu_device();
            let frame = world
                .get_resource::<ExtractedWindows>()
                .unwrap()
                .windows
                .values()
                .last()
                .unwrap()
                .swap_chain_texture
                .as_ref()
                .unwrap();
            renderer.with_primitives(|backend, primitive| {
                backend.present(
                    device,
                    &mut staging_belt,
                    &mut ctx.command_encoder,
                    frame,
                    primitive,
                    &viewport,
                    &debug.overlay(),
                );
            });
            staging_belt.finish();

            let encoder = std::mem::replace(
                &mut ctx.command_encoder,
                device.create_command_encoder(&CommandEncoderDescriptor { label: None }),
            );

            pool.spawner().spawn(staging_belt.recall()).unwrap();

            let queue = world.get_resource::<RenderQueue>().unwrap();
            queue.submit(Some(encoder.finish()));
        });
        self.sub_app_mut(RenderApp)
            .world
            .get_non_send_resource_mut::<RefCell<Vec<DrawFn>>>()
            .unwrap().borrow_mut()
            .push(draw_fn);

        self.insert_non_send_resource(program);
        self
    }
}

pub fn setup_pipeline(graph: &mut RenderGraph) {
    graph.add_node(render::ICED_PASS, IcedNode::new());

    graph
        .add_node_edge(
            bevy::core_pipeline::node::MAIN_PASS_DRIVER,
            render::ICED_PASS,
        )
        .unwrap();
}
