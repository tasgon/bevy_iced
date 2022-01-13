use std::{cell::RefCell, sync::Arc};

use bevy::{
    prelude::{App, Plugin, World},
    render::{renderer::{RenderDevice, RenderContext}, RenderApp, texture::BevyDefault, view::ExtractedWindows},
};
use iced_native::{program, Debug, Program, Size};
use iced_wgpu::{wgpu::{self, util::StagingBelt}, Viewport};

pub type IcedState<T> = Arc<RefCell<program::State<T>>>;

mod conversions;
mod render;
mod systems;
pub struct IcedPlugin;

impl Plugin for IcedPlugin {
    fn build(&self, app: &mut App) {
        app//.add_system(systems::process_input)
           .insert_non_send_resource(Vec::<UpdateFn>::new());
    }
}

type UpdateFn = Box<dyn Fn(&mut World, &iced_native::Event)>;
type DrawFn = Box<dyn FnMut(&mut World, &mut RenderContext)>;

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
        let program = program::State::new(
            program,
            viewport.logical_size(),
            &mut renderer,
            &mut debug,
        );
        let mut staging_belt = StagingBelt::new(5 * 1024);

        let update_fn: UpdateFn = Box::new(move |world: &mut World, event: &iced_native::Event| {
            let mut state = world.get_non_send_resource_mut::<program::State<T>>().unwrap();
            state.queue_event(event.clone());
        });
        self.world.get_non_send_resource_mut::<Vec<UpdateFn>>().unwrap().push(update_fn);

        let draw_fn: DrawFn = Box::new(move |world: &mut World, ctx: &mut RenderContext| {
            let device = world.get_resource::<RenderDevice>().unwrap().wgpu_device();
            let frame = world.get_resource::<ExtractedWindows>().unwrap().windows.values().last().unwrap().swap_chain_texture.as_ref().unwrap();
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
        });
        self.world.get_non_send_resource_mut::<Vec<DrawFn>>().unwrap().push(draw_fn);

        self.insert_non_send_resource(program);
        self
    }
}
