//! # Use Iced UI programs in your Bevy application
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_iced::iced::widget::text;
//! use bevy_iced::{IcedContext, IcedPlugin};
//!
//! #[derive(Event)]
//! pub enum UiMessage {}
//!
//! pub fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugin(IcedPlugin::default())
//!         .add_event::<UiMessage>()
//!         .add_systems(Update, ui_system)
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

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::Mutex;

use crate::render::{extract_iced_data, IcedNode, ViewportResource};

use bevy_app::{App, Plugin, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::{EventWriter, Query, With};
use bevy_ecs::system::{NonSendMut, Res, ResMut, Resource, SystemParam};
#[cfg(feature = "touch")]
use bevy_input::touch::Touches;
use bevy_math::Vec2;
use bevy_render::render_graph::RenderGraph;
use bevy_render::renderer::{RenderDevice, RenderQueue};
use bevy_render::{ExtractSchedule, RenderApp};
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window};
use iced::mouse::Cursor;
use iced_graphics::backend::Text;
use iced_runtime::user_interface::UserInterface;
use iced_winit::Viewport;

pub use iced;
pub use iced_core::renderer::Style as IcedStyle;
pub use iced_graphics::Antialiasing as IcedAntialiasing;
pub use iced_wgpu;

mod conversions;
mod render;
mod systems;

use systems::IcedEventQueue;

/// The main feature of `bevy_iced`.
/// Add this to your [`App`] by calling `app.add_plugin(bevy_iced::IcedPlugin::default())`.
pub struct IcedPlugin {
    /// The default [`Font`] to use.
    pub default_font: iced::Font,
    /// The default size of text.
    pub default_text_size: f32,
    /// The antialiasing strategy that will be used for triangle primitives.
    ///
    /// By default, it is `None`.
    pub antialiasing: Option<IcedAntialiasing>,
    /// Font file contents
    pub fonts: Vec<&'static [u8]>,
}

impl Default for IcedPlugin {
    fn default() -> Self {
        Self {
            default_font: iced::Font::default(),
            default_text_size: iced_wgpu::Settings::default().default_text_size,
            antialiasing: None,
            fonts: vec![],
        }
    }
}

impl Plugin for IcedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (systems::process_input, render::update_viewport))
            .insert_resource(DidDraw::default())
            .insert_resource(IcedSettings::default())
            .insert_non_send_resource(IcedCache::default())
            .insert_resource(IcedEventQueue::default());
    }

    fn finish(&self, app: &mut App) {
        let default_viewport = Viewport::with_physical_size(iced::Size::new(1600, 900), 1.0);
        let default_viewport = ViewportResource(default_viewport);
        let iced_resource: IcedResource = IcedProps::new(
            app,
            self.default_font,
            self.default_text_size,
            self.antialiasing,
            &self.fonts,
        )
        .into();

        app.insert_resource(default_viewport.clone())
            .insert_resource(iced_resource.clone());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(default_viewport)
            .insert_resource(iced_resource)
            .add_systems(ExtractSchedule, extract_iced_data);
        setup_pipeline(&mut render_app.world.get_resource_mut().unwrap());
    }
}

struct IcedProps {
    renderer: iced_wgpu::Renderer<iced::Theme>,
    debug: iced_runtime::Debug,
    clipboard: iced_core::clipboard::Null,
}

impl IcedProps {
    fn new(
        app: &App,
        default_font: iced::Font,
        default_text_size: f32,
        antialiasing: Option<IcedAntialiasing>,
        fonts: &Vec<&'static [u8]>,
    ) -> Self {
        let render_world = &app.sub_app(RenderApp).world;
        let device = render_world
            .get_resource::<RenderDevice>()
            .unwrap()
            .wgpu_device();
        let queue = render_world.get_resource::<RenderQueue>().unwrap();
        let format = iced_wgpu::wgpu::TextureFormat::Bgra8UnormSrgb;
        let settings = iced_wgpu::Settings {
            default_font,
            default_text_size,
            antialiasing,
            ..Default::default()
        };
        let mut backend = iced_wgpu::Backend::new(device, queue, settings, format);
        for font in fonts {
            backend.load_font(Cow::Borrowed(*font));
        }

        Self {
            renderer: iced_wgpu::Renderer::new(backend),
            debug: iced_runtime::Debug::new(),
            clipboard: iced_core::clipboard::Null,
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
    cache: HashMap<TypeId, Option<iced_runtime::user_interface::Cache>>,
}

impl IcedCache {
    fn get<M: Any>(&mut self) -> &mut Option<iced_runtime::user_interface::Cache> {
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
    pub theme: iced::Theme,
    /// The style to use for rendering Iced elements.
    pub style: IcedStyle,
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
            theme: iced::Theme::Dark,
            style: IcedStyle {
                text_color: iced::Color::WHITE,
            },
        }
    }
}

// An atomic flag for updating the draw state.
#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct DidDraw(std::sync::atomic::AtomicBool);

/// The context for interacting with Iced. Add this as a parameter to your system.
/// ```ignore
/// fn ui_system(..., mut ctx: IcedContext<UiMessage>) {
///     let element = ...; // Build your element
///     ctx.display(element);
/// }
/// ```
///
/// `IcedContext<T>` requires an event system to be defined in the [`App`].
/// Do so by invoking `app.add_event::<T>()` when constructing your App.
#[derive(SystemParam)]
pub struct IcedContext<'w, 's, Message: bevy_ecs::event::Event> {
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

impl<'w, 's, M: bevy_ecs::event::Event> IcedContext<'w, 's, M> {
    /// Display an [`Element`] to the screen.
    pub fn display<'a>(
        &'a mut self,
        element: impl Into<iced::Element<'a, M, iced_wgpu::Renderer<iced::Theme>>>,
    ) {
        let IcedProps {
            ref mut renderer,
            ref mut clipboard,
            ..
        } = &mut *self.props.lock().unwrap();
        let bounds = self.viewport.logical_size();

        let element = element.into();

        let cursor = {
            let window = self.windows.single();
            match window.cursor_position() {
                Some(position) => {
                    Cursor::Available(process_cursor_position(position, bounds, window))
                }
                None => process_touch_input(self)
                    .map(Cursor::Available)
                    .unwrap_or(Cursor::Unavailable),
            }
        };

        let mut messages = Vec::<M>::new();
        let cache_entry = self.cache_map.get::<M>();
        let cache = cache_entry.take().unwrap();
        let mut ui = UserInterface::build(element, bounds, cache, renderer);
        let (_, _event_statuses) = ui.update(
            self.events.as_slice(),
            cursor,
            renderer,
            clipboard,
            &mut messages,
        );

        messages.into_iter().for_each(|msg| self.messages.send(msg));

        ui.draw(renderer, &self.settings.theme, &self.settings.style, cursor);

        self.events.clear();
        *cache_entry = Some(ui.into_cache());
        self.did_draw
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

fn process_cursor_position(position: Vec2, bounds: iced::Size, window: &Window) -> iced::Point {
    iced::Point {
        x: position.x * bounds.width / window.width(),
        y: position.y * bounds.height / window.height(),
    }
}

#[cfg(feature = "touch")]
/// To correctly process input as last resort events are used
fn process_touch_input<M: bevy_ecs::event::Event>(context: &IcedContext<M>) -> Option<iced::Point> {
    context
        .touches
        .first_pressed_position()
        .or(context
            .touches
            .iter_just_released()
            .map(|touch| touch.position())
            .next())
        .map(|Vec2 { x, y }| iced::Point { x, y })
        .or(context
            .events
            .iter()
            .filter_map(|ev| {
                if let iced::Event::Touch(
                    iced::touch::Event::FingerLifted { position, .. }
                    | iced::touch::Event::FingerLost { position, .. }
                    | iced::touch::Event::FingerMoved { position, .. }
                    | iced::touch::Event::FingerPressed { position, .. },
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
fn process_touch_input<M: bevy_ecs::event::Event>(_: &IcedContext<M>) -> Option<iced::Point> {
    None
}
