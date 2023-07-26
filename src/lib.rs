//! # Use Iced UI programs in your Bevy application
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_iced::iced::widget::text;
//! use bevy_iced::{IcedContext, IcedPlugin};
//!
//! pub enum UiMessage {}
//!
//! pub fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugin(IcedPlugin)
//!         .add_event::<UiMessage>()
//!         .add_system(ui_system)
//!         .run();
//! }
//!
//! fn ui_system(time: Res<Time>, mut ctx: IcedContext<UiMessage>) {
//!     ctx.display(text(format!(
//!         "Hello Iced! Running for {:.2} seconds.",
//!         time.elapsed_seconds()
//!     )));
//! }
//! ```
//!
//! ## Feature flags
//!
//! - `touch`: Enables touch input. Is not exclude input from the mouse.

#![deny(unsafe_code)]
#![deny(missing_docs)]

use std::any::{Any, TypeId};

use std::sync::Arc;
use std::sync::Mutex;

use crate::render::IcedNode;
use crate::render::ViewportResource;

use bevy_app::{App, IntoSystemAppConfig, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::event::Event;
use bevy_ecs::prelude::{EventWriter, Query, With};
use bevy_ecs::system::{NonSendMut, Res, ResMut, Resource, SystemParam};
#[cfg(feature = "touch")]
use bevy_input::touch::Touches;
use bevy_math::Vec2;
use bevy_render::render_graph::RenderGraph;
use bevy_render::renderer::RenderDevice;
use bevy_render::{ExtractSchedule, RenderApp};
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window};
use iced::{user_interface, Element, UserInterface};
pub use iced_native as iced;
use iced_native::{Debug, Size};
pub use iced_wgpu;
use iced_wgpu::{wgpu, Viewport};

mod conversions;
mod render;
mod systems;

use systems::IcedEventQueue;

/// The main feature of `bevy_iced`.
/// Add this to your [`App`] by calling `app.add_plugin(bevy_iced::IcedPlugin)`.
pub struct IcedPlugin;

impl Plugin for IcedPlugin {
    fn build(&self, app: &mut App) {
        let default_viewport = Viewport::with_physical_size(Size::new(1600, 900), 1.0);
        let default_viewport = ViewportResource(default_viewport);
        let iced_resource: IcedResource = IcedProps::new(app).into();

        app.add_system(systems::process_input)
            .add_system(render::update_viewport)
            .insert_resource(DidDraw::default())
            .insert_resource(iced_resource.clone())
            .insert_resource(IcedSettings::default())
            .insert_non_send_resource(IcedCache::default())
            .insert_resource(IcedEventQueue::default())
            .insert_resource(default_viewport.clone());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(default_viewport)
            .insert_resource(iced_resource)
            .add_system(render::extract_iced_data.in_schedule(ExtractSchedule));
        setup_pipeline(&mut render_app.world.get_resource_mut().unwrap());
    }
}

struct IcedProps {
    renderer: iced_wgpu::Renderer,
    debug: iced_native::Debug,
    clipboard: iced_native::clipboard::Null,
}

impl IcedProps {
    fn new(app: &App) -> Self {
        let device = app
            .sub_app(RenderApp)
            .world
            .get_resource::<RenderDevice>()
            .unwrap()
            .wgpu_device();
        #[cfg(target_arch = "wasm32")]
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        #[cfg(not(target_arch = "wasm32"))]
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;

        Self {
            renderer: iced_wgpu::Renderer::new(iced_wgpu::Backend::new(
                device,
                Default::default(),
                format,
            )),
            debug: Debug::new(),
            clipboard: iced_native::clipboard::Null,
        }
    }
}

#[derive(Resource, Clone)]
struct IcedResource(Arc<Mutex<IcedProps>>);

impl IcedResource {
    fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<IcedProps>> {
        self.0.lock()
    }
}

impl From<IcedProps> for IcedResource {
    fn from(value: IcedProps) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}

fn setup_pipeline(graph: &mut RenderGraph) {
    graph.add_node(render::ICED_PASS, IcedNode::new());

    graph.add_node_edge(
        bevy_render::main_graph::node::CAMERA_DRIVER,
        render::ICED_PASS,
    );
}

#[doc(hidden)]
#[derive(Default)]
pub struct IcedCache {
    cache: HashMap<TypeId, Option<user_interface::Cache>>,
}

impl IcedCache {
    fn get<M: Any>(&mut self) -> &mut Option<user_interface::Cache> {
        let id = TypeId::of::<M>();
        if !self.cache.contains_key(&id) {
            self.cache.insert(id, Some(Default::default()));
        }
        self.cache.get_mut(&id).unwrap()
    }
}

/// Settings used to independently customize Iced rendering.
#[derive(Clone, Resource)]
pub struct IcedSettings {
    /// The scale factor to use for rendering Iced elements.
    /// Setting this to `None` defaults to using the `Window`s scale factor.
    pub scale_factor: Option<f64>,
    /// The theme to use for rendering Iced elements.
    pub theme: iced_wgpu::Theme,
    /// The style to use for rendering Iced elements.
    pub style: iced_native::renderer::Style,
}

impl IcedSettings {
    /// Set the `scale_factor` used to render Iced elements.
    pub fn set_scale_factor(&mut self, factor: impl Into<Option<f64>>) {
        self.scale_factor = factor.into();
    }
}

impl Default for IcedSettings {
    fn default() -> Self {
        Self {
            scale_factor: None,
            theme: iced_wgpu::Theme::Dark,
            style: iced_native::renderer::Style {
                text_color: iced_native::Color::WHITE,
            },
        }
    }
}

// An atomic flag for updating the draw state.
#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct DidDraw(std::sync::atomic::AtomicBool);

/// The context for interacting with Iced. Add this as a parameter to your system.
/// ```no_run
/// fn ui_system(..., mut ctx: IcedContext<UiMessage>) {
///     let element = ...; // Build your element
///     ctx.display(element);
/// }
/// ```
///
/// `IcedContext<T>` requires an event system to be defined in the [`App`].
/// Do so by invoking `app.add_event::<T>()` when constructing your App.
#[derive(SystemParam)]
pub struct IcedContext<'w, 's, Message: Event> {
    viewport: Res<'w, ViewportResource>,
    props: Res<'w, IcedResource>,
    settings: Res<'w, IcedSettings>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    events: ResMut<'w, IcedEventQueue>,
    cache_map: NonSendMut<'w, IcedCache>,
    messages: EventWriter<'w, Message>,
    did_draw: ResMut<'w, DidDraw>,
    #[cfg(feature = "touch")]
    touches: Res<'w, Touches>,
}

impl<'w, 's, M: Event> IcedContext<'w, 's, M> {
    /// Display an [`Element`] to the screen.
    pub fn display<'a>(&'a mut self, element: impl Into<Element<'a, M, iced_wgpu::Renderer>>) {
        let IcedProps {
            ref mut renderer,
            ref mut clipboard,
            ..
        } = &mut *self.props.lock().unwrap();
        let bounds = self.viewport.logical_size();

        let element = element.into();

        let cursor_position = {
            let window = self.windows.single();

            window
                .cursor_position()
                .map(|Vec2 { x, y }| iced_native::Point {
                    x: x * bounds.width / window.width(),
                    y: (window.height() - y) * bounds.height / window.height(),
                })
                .or_else(|| {
                    process_touch_input(self).map(|iced_native::Point { x, y }| {
                        iced_native::Point {
                            x: x * bounds.width / window.width(),
                            y: y * bounds.height / window.height(),
                        }
                    })
                })
                .unwrap_or(iced_native::Point::ORIGIN)
        };

        let mut messages = Vec::<M>::new();
        let cache_entry = self.cache_map.get::<M>();
        let cache = cache_entry.take().unwrap();
        let mut ui = UserInterface::build(element, bounds, cache, renderer);
        let (_, _event_statuses) = ui.update(
            self.events.as_slice(),
            cursor_position,
            renderer,
            clipboard,
            &mut messages,
        );

        messages.into_iter().for_each(|msg| self.messages.send(msg));

        ui.draw(
            renderer,
            &self.settings.theme,
            &self.settings.style,
            cursor_position,
        );

        self.events.clear();
        *cache_entry = Some(ui.into_cache());
        self.did_draw
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(feature = "touch")]
/// To correctly process input as last resort events are used
fn process_touch_input<M: Event>(context: &IcedContext<M>) -> Option<iced_native::Point> {
    context
        .touches
        .first_pressed_position()
        .or(context
            .touches
            .iter_just_released()
            .map(|touch| touch.position())
            .next())
        .map(|Vec2 { x, y }| iced_native::Point { x, y })
        .or(context
            .events
            .iter()
            .filter_map(|ev| {
                if let iced_native::Event::Touch(
                    iced_native::touch::Event::FingerLifted { position, .. }
                    | iced_native::touch::Event::FingerLost { position, .. }
                    | iced_native::touch::Event::FingerMoved { position, .. }
                    | iced_native::touch::Event::FingerPressed { position, .. },
                ) = ev
                {
                    Some(position)
                } else {
                    None
                }
            })
            .next()
            .copied())
}

#[cfg(not(feature = "touch"))]
fn process_touch_input<M: Event>(_: &IcedContext<M>) -> Option<iced_native::Point> {
    None
}
