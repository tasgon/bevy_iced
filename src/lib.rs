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

use std::any::TypeId;
use std::cell::UnsafeCell;
use std::marker::PhantomData;

use std::sync::Mutex;
use std::{cell::RefCell, sync::Arc};

use crate::render::IcedNode;
use crate::render::{IcedRenderData, ViewportResource};

use bevy::ecs::all_tuples;
use bevy::ecs::system::{
    ReadOnlySystemParamFetch, SystemParam, SystemParamFetch, SystemParamItem, SystemState,
};
use bevy::prelude::{Deref, DerefMut, IntoSystem, NonSendMut, Res, ResMut, Resource, System, Events};
use bevy::render::render_graph::RenderGraph;
use bevy::render::RenderStage;
use bevy::time::Time;
use bevy::utils::HashMap;
use bevy::window::Windows;
use bevy::{
    prelude::{App, Plugin, World},
    render::{
        renderer::{RenderContext, RenderDevice},
        RenderApp,
    },
};
pub use iced_native as iced;
use iced_native::{program, Debug, Program, Size};
pub use iced_wgpu;
use iced_wgpu::{wgpu, Viewport};

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

pub struct IcedContext {
    active: Option<TypeId>,
    update_fns: HashMap<TypeId, fn(&mut World)>,
    draw_fns: HashMap<TypeId, fn(&World, &mut RenderContext, &mut render::IcedRenderData)>,
}

struct IcedProps {
    renderer: iced_wgpu::Renderer,
    debug: iced_native::Debug,
    clipboard: iced_native::clipboard::Null,
}

#[derive(Resource, Deref, DerefMut)]
struct IcedPropsResource(Arc<Mutex<IcedProps>>);

struct IcedProgramData<T> {
    renderer: iced_wgpu::Renderer,
    debug: iced_native::Debug,
    _phantom: PhantomData<T>,
}

/// A trait that adds the necessary features for an [`App`](`bevy::prelude::App`)
/// to handle Iced.
pub trait IcedAppExtensions {
    /// Insert a new [`Program`](`iced::Program`) and make it accessible as a resource.
    fn insert_program<M, T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static>(
        &mut self,
        program: T,
    ) -> &mut Self;
}

struct IcedProgram<M, T>(PhantomData<(M, T)>);

impl<M, T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static> IcedProgram<M, T> {
    fn update_fn(world: &mut World) {
        world.resource_scope::<program::State<T>, _>(|world, mut state| {
            for ev in &**world.resource::<IcedEventQueue>() {
                state.queue_event(ev.clone());
            }
            let bounds = world.resource::<ViewportResource>().logical_size();

            world.resource_scope::<IcedPropsResource, _>(|world, ctx| {
                let IcedProps {
                    ref mut renderer,
                    ref mut debug,
                    ref mut clipboard,
                    ..
                } = &mut *ctx.lock().unwrap();
                let windows = world.resource::<Windows>();
                if !state.is_queue_empty() {
                    let window = windows.get_primary().unwrap();
                    let cursor_position = window.cursor_position().map_or(
                        iced_native::Point { x: 0.0, y: 0.0 },
                        |p| iced_native::Point {
                            x: p.x * bounds.width / window.width(),
                            y: (window.height() - p.y) * bounds.height / window.height(),
                        },
                    );

                    state.update(
                        bounds,
                        cursor_position,
                        renderer,
                        &iced_wgpu::Theme::Dark,
                        &iced_native::renderer::Style {
                            text_color: iced_native::Color::WHITE,
                        },
                        clipboard,
                        debug,
                    );
                }
            });
        });
    }

    fn draw_fn(world: &World, ctx: &mut RenderContext, data: &mut render::IcedRenderData) {
        let viewport = unsafe { world.get_resource_unchecked_mut::<ViewportResource>() }.unwrap();
        let IcedProps {
            ref mut renderer,
            ref mut debug,
            ..
        } = &mut *world.resource::<IcedPropsResource>().lock().unwrap();

        let device = ctx.render_device.wgpu_device();
        renderer.with_primitives(|backend, primitive| {
            backend.present(
                device,
                data.staging_belt,
                &mut ctx.command_encoder,
                data.view,
                primitive,
                &viewport,
                &debug.overlay(),
            );
        });
    }
}

fn create_state<M, T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static>(
    app: &mut App,
    program: T,
) {
    let render_world = &mut app.sub_app_mut(RenderApp).world;
    let bounds = render_world.resource::<ViewportResource>().logical_size();
    let IcedProps {
        ref mut renderer,
        ref mut debug,
        ..
    } = &mut *render_world.non_send_resource_mut::<_>();

    let _state = program::State::new(program, bounds, renderer, debug);
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
                            &iced_native::renderer::Style {
                                text_color: iced_native::Color::WHITE,
                            },
                            &mut clipboard,
                            debug,
                        );
                    }
                }
            },
        );

        let draw_fn: DrawFn = Box::new(
            move |_world: &World,
                  ctx: &mut RenderContext,
                  current_viewport: &Viewport,
                  data: &mut IcedRenderData| {
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
    fn insert_program<M, T: Program<Renderer = iced_wgpu::Renderer, Message = M> + 'static>(
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

type Element<'a> = iced::Element<'a, (), iced_wgpu::Renderer>;

trait Invoker<Params, Out> {
    fn invoke(&self, params: Params) -> Out;
}

macro_rules! invoker_impl {
    ($($param: ident),*) => {
        impl<$($param,)* Out, F> Invoker<($($param,)*), Out> for F
        where
            F: Fn($($param,)*) -> Out
        {
            fn invoke(&self, params: ($($param,)*)) -> Out {
                let ($($param,)*) = params;
                (self)($($param,)*)
            }
        }
    };
}
all_tuples!(invoker_impl, 0, 16, P);

pub trait ElementProvider {
    type In: SystemParam + 'static;

    fn process<'a>(&self, input: Self::In) -> Element<'a>;

    fn get_state(&self, world: &mut World) -> SystemState<Self::In> {
        SystemState::<Self::In>::new(world)
    }
}

fn thonk<Params: SystemParam + 'static, Out, F: Invoker<Params, Out>>(
    f: &F,
    world: &mut World,
) -> SystemState<Params> {
    let state = SystemState::<Params>::new(world);
    state
}

fn doomer<'a, Params: SystemParam + 'static, Out, F: Invoker<<Params::Fetch as SystemParamFetch<'a, 'a>>::Item, Out>>(
    f: &F,
    world: &'a World,
    state: &'a mut SystemState<Params>,
) -> Out
where
    Params::Fetch: ReadOnlySystemParamFetch,
{
    let params = state.get(world);
    f.invoke(params)
}

pub struct Ctx {
    prog: Box<dyn BevyIcedProcessor>
}

struct BevyIcedProgram<F, P: SystemParam + 'static> {
    world_ref: Option<*const World>,
    system_state: Option<UnsafeCell<SystemState<P>>>,
    system: F,
    _p: PhantomData<P>
}

impl<Params: SystemParam + 'static, F: for<'a> Invoker<<Params::Fetch as SystemParamFetch<'a, 'a>>::Item, Element<'a>>> BevyIcedProgram<F, Params>
where Params::Fetch: ReadOnlySystemParamFetch
{
    pub fn new(system: F) -> Self {
        Self {
            world_ref: None,
            system_state: None,
            system,
            _p: Default::default(),
        }
    }

    pub fn update(&mut self, world: &mut World) {
        let world_ptr = world as *const World;
        if self.world_ref.map(|x| x == world_ptr).unwrap_or(false) {
            self.world_ref = Some(world_ptr);
            self.system_state = Some(UnsafeCell::new(SystemState::<Params>::new(world)))
        }
    }
}

impl<Params: SystemParam + 'static, F: for<'a> Invoker<<Params::Fetch as SystemParamFetch<'a, 'a>>::Item, Element<'a>>> Program for BevyIcedProgram<F, Params>
where Params::Fetch: ReadOnlySystemParamFetch
{
    type Renderer = iced_wgpu::Renderer;
    type Message = ();

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        let world = unsafe { &mut *(self.world_ref.unwrap() as *mut World) };
        let mut events = world.resource_mut::<Events<Self::Message>>();
        events.send(message);
        iced::Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Renderer> {
        let world = unsafe { &*self.world_ref.unwrap() };
        let state = self.system_state.as_ref().map(|x| unsafe { &mut *x.get() }).unwrap();
        let params = state.get(world);
        self.system.invoke(params)
    }
}

trait BevyIcedProcessor {
    fn update(&mut self);
    fn render(&self);
}

impl<F, P: SystemParam + 'static> BevyIcedProcessor for program::State<BevyIcedProgram<F, P>> where BevyIcedProgram<F, P>: Program {
    fn update(&mut self) {
        todo!()
    }

    fn render(&self) {
        todo!()
    }
}