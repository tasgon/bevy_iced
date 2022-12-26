//! # Use Iced UI programs in your Bevy application
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_iced::{
//!     IcedAppExtensions, IcedPlugin,
//!     iced::{Program, program::State}
//! };
//!
//! #[derive(Default)]
//! pub struct Ui {
//!     // Set up your UI state
//! }
//!
//! impl Program for Ui {
//!     // Set up your program logic
//! }
//!
//! pub fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugin(IcedPlugin)
//!         .insert_program(Ui::default())
//!         .add_system(ui_system)
//!         .run();
//! }
//!
//! pub fn ui_system(mut ui_state: NonSendMut<State<Ui>>, /* ... */) {
//!     // Do some work here, then modify your ui state by running
//!     // ui_state.queue_message(..);
//! }
//! ```

use std::marker::PhantomData;
use std::{cell::RefCell, sync::Arc};

use crate::render::IcedNode;
use bevy::prelude::{NonSendMut, Res, ResMut, Resource, Deref, DerefMut, Component};
use bevy::render::render_graph::RenderGraph;
use bevy::render::RenderStage;
use bevy::window::Windows;
use bevy::{
    prelude::{App, Commands, Plugin, World},
    render::{
        renderer::{RenderContext, RenderDevice},
        texture::BevyDefault,
        RenderApp,
    },
};
pub use iced_native as iced;
use iced_native::{program, Debug, Program, Size};
pub use iced_wgpu;
use iced_wgpu::{wgpu, Viewport};
use crate::render::{IcedRenderData, ViewportResource};

mod conversions;
mod render;
mod systems;

pub use render::IcedSettings;
use systems::IcedEventQueue;

/// The main feature of `bevy_iced`.
/// Add this to your [`App`](`bevy::prelude::App`) by calling `app.add_plugin(bevy_iced::IcedPlugin)`.
pub struct IcedPlugin;

impl Plugin for IcedPlugin {
    fn build(&self, app: &mut App) {
        let default_viewport = Viewport::with_physical_size(Size::new(1600, 900), 1.0);
        let default_viewport = ViewportResource(default_viewport);

        app.add_system(systems::process_input)
            .add_system(render::update_viewport)
            .insert_resource(IcedEventQueue::default())
            .insert_resource(default_viewport.clone());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_non_send_resource(RefCell::new(Vec::<DrawFn>::new()));
        render_app.insert_resource(default_viewport);
        render_app.add_system_to_stage(RenderStage::Extract, render::extract_iced_data);
        // render_app.init_resource::<render::IcedPipeline>();
        setup_pipeline(&mut render_app.world.get_resource_mut().unwrap());
    }
}

type DrawFn = Box<dyn FnMut(&World, &mut RenderContext, &Viewport, &mut render::IcedRenderData)>;

#[derive(Resource)]
struct IcedProgramData<T> {
    renderer: iced_wgpu::Renderer,
    debug: iced_native::Debug,
    _phantom: PhantomData<T>,
}

/// The user-defined rendering state of an Iced program.
#[derive(Resource)]
pub struct IcedRenderState<T> {
    /// Used to set whether an Iced program should be updated and displayed.
    pub active: bool,
    _phantom: PhantomData<T>,
}

impl<T> IcedRenderState<T> {
    pub fn active(active: bool) -> Self {
        Self {
            active,
            _phantom: Default::default(),
        }
    }
}

impl<T> Default for IcedRenderState<T> {
    fn default() -> Self {
        Self {
            active: true,
            _phantom: Default::default(),
        }
    }
}

impl<T> Clone for IcedRenderState<T> {
    fn clone(&self) -> Self {
        Self {
            active: self.active,
            _phantom: Default::default(),
        }
    }
}

unsafe impl<T> Send for IcedRenderState<T> {}
unsafe impl<T> Sync for IcedRenderState<T> {}

/// A trait that adds the necessary features for an [`App`](`bevy::prelude::App`)
/// to handle Iced.
pub trait IcedAppExtensions {
    /// Insert a new [`Program`](`iced::Program`) and make it accessible as a resource.
    fn insert_program<
        M,
        T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static,
    >(
        &mut self,
        program: T,
    ) -> &mut Self;
}

macro_rules! base_insert_proc {
    ($app:expr, $program:expr, $state_type:ty) => {{
        let device = $app
            .sub_app(RenderApp)
            .world
            .get_resource::<RenderDevice>()
            .unwrap()
            .wgpu_device();
        // let format = wgpu::TextureFormat::bevy_default();
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let mut renderer =
            iced_wgpu::Renderer::new(iced_wgpu::Backend::new(device, Default::default(), format));
        let viewport = Viewport::with_physical_size(Size::new(1600, 900), 1.0);
        let mut debug = Debug::new();
        let mut clipboard = iced_native::clipboard::Null;
        let program =
            program::State::new($program, viewport.logical_size(), &mut renderer, &mut debug);

        let update_data = Arc::new(IcedProgramData::<T> {
            renderer,
            debug,
            _phantom: Default::default(),
        });
        let draw_data = update_data.clone();
        $app.insert_non_send_resource(update_data.clone());

        $app.add_system(
            move |program_state: Option<$state_type>,
                  mut data: NonSendMut<Arc<IcedProgramData<T>>>,
                  windows: Res<Windows>,
                  viewport: Res<ViewportResource>,
                  events: Res<IcedEventQueue>| {
                if let Some(mut state) = program_state {
                    let IcedProgramData::<T> {
                        renderer,
                        debug,
                        _phantom,
                    } = unsafe { get_rc_mut(&mut *data) };

                    for ev in &**events {
                        state.queue_event(ev.clone());
                    }

                    let size = viewport.logical_size();

                    if !state.is_queue_empty() {
                        let window = windows.get_primary().unwrap();
                        let cursor_position = window.cursor_position().map_or(
                            iced_native::Point { x: 0.0, y: 0.0 },
                            |p| iced_native::Point {
                                x: p.x * size.width / window.width(),
                                y: (window.height() - p.y) * size.height / window.height(),
                            },
                        );

                        state.update(
                            viewport.logical_size(),
                            cursor_position,
                            renderer,
                            &iced_wgpu::Theme::Dark,
                            &iced_native::renderer::Style { text_color: iced_native::Color::WHITE },
                            &mut clipboard,
                            debug,
                        );
                    }
                }
            },
        );

        $app.sub_app_mut(RenderApp).add_system_to_stage(
            RenderStage::Extract,
            |mut commands: Commands, state: Option<Res<IcedRenderState<T>>>| {
                commands.insert_resource(state.map(|x| x.clone()).unwrap_or_default());
            },
        );

        let draw_fn: DrawFn = Box::new(
            move |world: &World,
                  ctx: &mut RenderContext,
                  current_viewport: &Viewport,
                  data: &mut IcedRenderData| {
                if !world.get_resource::<IcedRenderState<T>>().unwrap().active {
                    return;
                }

                let IcedProgramData::<T> {
                    renderer,
                    debug,
                    _phantom,
                } = unsafe { get_rc_mut(&draw_data) };

                let device = ctx.render_device.wgpu_device();
                renderer.with_primitives(|backend, primitive| {
                    backend.present(
                        device,
                        data.staging_belt,
                        &mut ctx.command_encoder,
                        data.view,
                        primitive,
                        current_viewport,
                        &debug.overlay(),
                    );
                });
            },
        );

        $app.sub_app_mut(RenderApp)
            .world
            .get_non_send_resource_mut::<RefCell<Vec<DrawFn>>>()
            .unwrap()
            .borrow_mut()
            .push(draw_fn);

        program
    }};
}

impl IcedAppExtensions for App {
    fn insert_program<
        M,
        T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static,
    >(
        &mut self,
        program: T,
    ) -> &mut Self {
        let resource = base_insert_proc!(self, program, NonSendMut<program::State<T>>);
        self.insert_non_send_resource(resource)
    }
}

pub(crate) fn setup_pipeline(graph: &mut RenderGraph) {
    graph.add_node(render::ICED_PASS, IcedNode::new());

    graph
        .add_node_edge(
            bevy::render::main_graph::node::CAMERA_DRIVER,
            render::ICED_PASS,
        )
        .unwrap();
}

// TODO: find a cleaner way to share data between the update and render cycles; this needs to go.
unsafe fn get_rc_mut<'a, T>(rc: &'a Arc<T>) -> &'a mut T {
    let data = &**rc as *const T as *mut T;
    &mut *data
}
