use std::{cell::RefCell, sync::Arc};

use bevy::{
    prelude::{App, Plugin},
    render::{renderer::RenderDevice, RenderApp, texture::BevyDefault},
};
use iced_native::{program, Debug, Program, Size};
use iced_wgpu::{wgpu, Viewport};

pub type IcedState<T> = Arc<RefCell<program::State<T>>>;

mod conversions;
mod render;
mod systems;

pub struct IcedProgram {
    pub state: Arc<dyn IcedEventReceiver>,
    pub(crate) renderer: iced_wgpu::Renderer,
}

pub trait IcedEventReceiver {
    fn process_event(&self, ev: iced_native::Event);
}

impl<M, T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static> IcedEventReceiver
    for RefCell<program::State<T>>
{
    fn process_event(&self, ev: iced_native::Event) {
        self.borrow_mut().queue_event(ev);
    }
}

pub type IcedEventReceivers = Vec<IcedProgram>;

pub struct IcedPlugin;

impl Plugin for IcedPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(systems::process_input);
        app.insert_non_send_resource(IcedEventReceivers::new());
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
        let res: IcedState<T> = Arc::new(RefCell::new(program));
        self.world
            .get_non_send_resource_mut::<IcedEventReceivers>()
            .unwrap()
            .push(IcedProgram {
                state: res.clone(),
                renderer,
            });
        self.insert_non_send_resource(res);
        self
    }
}
