use std::{any::Any, cell::UnsafeCell, fmt::Debug, marker::PhantomData};

use bevy::{
    ecs::system::{ReadOnlySystemParamFetch, SystemParam, SystemParamFetch, SystemState},
    prelude::{Events, World},
    render::renderer::RenderContext,
    window::Windows,
};
use iced_native::{application, program, Program, Renderer};

use crate::{
    render::{IcedRenderData, ViewportResource},
    Element, IcedProps, IcedResource, Invoker,
};

pub(crate) struct BevyIcedProgram<F, P: SystemParam + 'static>
where
    for<'a> F: Invoker<<P::Fetch as SystemParamFetch<'a, 'a>>::Item, Element<'a>>,
{
    pub(crate) world_ref: Option<*const World>,
    pub(crate) system_state: Option<UnsafeCell<SystemState<P>>>,
    pub(crate) system: F,
}

impl<F, Params: SystemParam + 'static> BevyIcedProgram<F, Params>
where
    F: for<'a> Invoker<<Params::Fetch as SystemParamFetch<'a, 'a>>::Item, Element<'a>>,
{
    pub fn sync_world(&self, world: &mut World) {
        let world_ptr = world as *const World;
        if self.world_ref.map(|x| x == world_ptr).unwrap_or(false) {
            let self_ref = unsafe { &mut *(self as *const Self as *mut Self) };
            self_ref.world_ref = Some(world_ptr);
            self_ref.system_state = Some(UnsafeCell::new(SystemState::<Params>::new(world)))
        }
    }
}

// impl<
//         Params: SystemParam + 'static,
//         F,
//     > BevyIcedProgram<F, Params>
// where
//     BevyIcedProgram<F, Params>: BevyIcedProcessor,
// {
//     pub fn new(system: F) -> Self {
//         Self {
//             world_ref: None,
//             system_state: None,
//             system,
//         }
//     }
// }

impl<
        Params: SystemParam + 'static,
        F: for<'a> Invoker<<Params::Fetch as SystemParamFetch<'a, 'a>>::Item, Element<'a>>,
    > Program for BevyIcedProgram<F, Params>
where
    Params::Fetch: ReadOnlySystemParamFetch,
{
    type Renderer = iced_wgpu::Renderer;
    type Message = ();

    fn update(&mut self, message: Self::Message) -> iced_native::Command<Self::Message> {
        let world = unsafe { &mut *(self.world_ref.unwrap() as *mut World) };
        let mut events = world.resource_mut::<Events<Self::Message>>();
        events.send(message);
        iced_native::Command::none()
    }

    fn view(&self) -> iced_native::Element<'_, Self::Message, Self::Renderer> {
        let world = unsafe { &*self.world_ref.unwrap() };
        let state = self
            .system_state
            .as_ref()
            .map(|x| unsafe { &mut *x.get() })
            .unwrap();
        let params = state.get(world);
        self.system.invoke(params)
    }
}

pub trait BevyIcedProcessor {
    fn tick(&mut self, world: &mut World);
    fn render(&self, world: &World, ctx: &mut RenderContext, data: &mut IcedRenderData);
}

impl<F, P: SystemParam + 'static> BevyIcedProcessor for program::State<BevyIcedProgram<F, P>>
where
    BevyIcedProgram<F, P>: Program<Renderer = iced_wgpu::Renderer>,
    F: for<'a> Invoker<<P::Fetch as SystemParamFetch<'a, 'a>>::Item, Element<'a>>,
{
    fn tick(&mut self, world: &mut World) {
        self.program().sync_world(world);
        let bounds = world.resource::<ViewportResource>().logical_size();
        let cursor_position = {
            let windows = world.resource::<Windows>();
            let window = windows.get_primary().unwrap();
            let cursor_position =
                window
                    .cursor_position()
                    .map_or(iced_native::Point { x: 0.0, y: 0.0 }, |p| {
                        iced_native::Point {
                            x: p.x * bounds.width / window.width(),
                            y: (window.height() - p.y) * bounds.height / window.height(),
                        }
                    });
            cursor_position
        };

        let IcedProps {
            ref mut renderer,
            ref mut debug,
            ref mut clipboard,
        } = &mut *world.resource::<IcedResource>().lock().unwrap();

        self.update(
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

    fn render(&self, world: &World, ctx: &mut RenderContext, data: &mut IcedRenderData) {
        let IcedProps {
            ref mut renderer,
            ref mut debug,
            ..
        } = &mut *world.resource::<IcedResource>().lock().unwrap();
        let viewport = &*world.resource::<ViewportResource>();
        let device = ctx.render_device.wgpu_device();
        renderer.with_primitives(|backend, primitives| {
            backend.present(
                device,
                data.staging_belt,
                &mut ctx.command_encoder,
                data.view,
                primitives,
                viewport,
                &debug.overlay(),
            );
        });
    }
}

#[derive(Default)]
pub(crate) struct BIP<Message: Send + Sync + Debug> {
    element: UnsafeCell<Option<iced_native::Element<'static, Message, iced_wgpu::Renderer>>>,
}

impl<M: Send + Sync + Debug> BIP<M> {
    pub(crate) fn new() -> Self {
        Self {
            element: UnsafeCell::new(None),
        }
    }
}

impl<M: Send + Sync + Debug> Program for BIP<M> {
    type Renderer = iced_wgpu::Renderer;
    type Message = M;

    fn update(&mut self, message: Self::Message) -> iced_native::Command<Self::Message> {
        iced_native::Command::none()
    }

    fn view(&self) -> iced_native::Element<'_, Self::Message, Self::Renderer> {
        let el = unsafe { &mut *self.element.get() }.take().unwrap();
        el
    }
}

trait EventProcessor: Any {
    fn process_event(&mut self, event: iced_native::Event);
}

impl<M: Send + Sync + Debug> EventProcessor for iced_native::program::State<BIP<M>> {
    fn process_event(&mut self, event: iced_native::Event) {
        self.queue_event(event);
    }
}

// pub(crate) mod thonk {
//     pub fn update<E: Debug + Send>(
//         &mut self,
//         bounds: Size,
//         cursor_position: Point,
//         renderer: &mut P::Renderer,
//         theme: &<P::Renderer as crate::Renderer>::Theme,
//         style: &renderer::Style,
//         clipboard: &mut dyn Clipboard,
//         debug: &mut Debug,
//     ) -> (Vec<Event>, Option<Command<P::Message>>) {
//         let mut user_interface = build_user_interface(
//             &mut self.program,
//             self.cache.take().unwrap(),
//             renderer,
//             bounds,
//             debug,
//         );

//         debug.event_processing_started();
//         let mut messages = Vec::new();

//         let (_, event_statuses) = user_interface.update(
//             &self.queued_events,
//             cursor_position,
//             renderer,
//             clipboard,
//             &mut messages,
//         );

//         let uncaptured_events = self
//             .queued_events
//             .iter()
//             .zip(event_statuses)
//             .filter_map(|(event, status)| {
//                 matches!(status, event::Status::Ignored).then_some(event)
//             })
//             .cloned()
//             .collect();

//         self.queued_events.clear();
//         messages.append(&mut self.queued_messages);
//         debug.event_processing_finished();

//         let command = if messages.is_empty() {
//             debug.draw_started();
//             self.mouse_interaction =
//                 user_interface.draw(renderer, theme, style, cursor_position);
//             debug.draw_finished();

//             self.cache = Some(user_interface.into_cache());

//             None
//         } else {
//             // When there are messages, we are forced to rebuild twice
//             // for now :^)
//             let temp_cache = user_interface.into_cache();

//             let commands =
//                 Command::batch(messages.into_iter().map(|message| {
//                     debug.log_message(&message);

//                     debug.update_started();
//                     let command = self.program.update(message);
//                     debug.update_finished();

//                     command
//                 }));

//             let mut user_interface = build_user_interface(
//                 &mut self.program,
//                 temp_cache,
//                 renderer,
//                 bounds,
//                 debug,
//             );

//             debug.draw_started();
//             self.mouse_interaction =
//                 user_interface.draw(renderer, theme, style, cursor_position);
//             debug.draw_finished();

//             self.cache = Some(user_interface.into_cache());

//             Some(commands)
//         };

//         (uncaptured_events, command)
//     }
// }
