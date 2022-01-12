use std::{cell::RefCell, sync::Arc};

use bevy::{
    prelude::{App, Plugin, World},
    render::{renderer::RenderDevice, RenderApp, texture::BevyDefault},
};
use iced_native::{program, Debug, Program, Size};
use iced_wgpu::{wgpu, Viewport};

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
type DrawFn = Box<dyn Fn(&mut World)>;

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
        let program = program::State::new(
            program,
            viewport.logical_size(),
            &mut renderer,
            &mut Debug::new(),
        );

        let update_fn: UpdateFn = Box::new(move |world: &mut World, event: &iced_native::Event| {
            let mut state = world.get_non_send_resource_mut::<program::State<T>>().unwrap();
            state.queue_event(event.clone());
        });
        self.world.get_non_send_resource_mut::<Vec<UpdateFn>>().unwrap().push(update_fn);

        // let draw_fn: DrawFn = Box::new(move |world: &mut World| {
        //     let device = world.get_resource::<RenderDevice>().unwrap().wgpu_device();
        //     renderer.with_primitives(|backend, primitive| {
        //         backend.present(
        //             device,
        //             &mut self.staging_belt,
        //             &mut encoder,
        //             &self.dest_view,
        //             primitive,
        //             &viewport,
        //             &[],
        //         );
        //     });
        // });
        // self.world.get_non_send_resource_mut::<Vec<DrawFn>>().unwrap().push(draw_fn);

        self.insert_non_send_resource(program);
        self
    }
}
