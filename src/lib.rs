use std::sync::Arc;

use bevy::{
    prelude::{App, Plugin},
    render::{renderer::RenderDevice, RenderApp},
};
use iced_native::{program, Debug, Program, Size};
use iced_wgpu::{wgpu, Viewport};

pub type IcedState<T> = Arc<program::State<T>>;

mod conversions;
mod systems;

pub struct IcedPlugin;

impl Plugin for IcedPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(systems::process_input);
    }
}

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
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let mut renderer =
            iced_wgpu::Renderer::new(iced_wgpu::Backend::new(device, Default::default(), format));
        let viewport = Viewport::with_physical_size(Size::new(100, 100), 1.0);
        let program = program::State::new(
            program,
            viewport.logical_size(),
            &mut renderer,
            &mut Debug::new(),
        );
        self.insert_non_send_resource(Arc::new(program));
        self
    }
}
