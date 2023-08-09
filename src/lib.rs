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
//!         .add_plugins(IcedPlugin::default())
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
use bevy_input::touch::Touches;
use bevy_render::render_graph::RenderGraph;
use bevy_render::renderer::{RenderDevice, RenderQueue};
use bevy_render::{ExtractSchedule, RenderApp};
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window};
use iced_core::mouse::Cursor;
use iced_runtime::user_interface::UserInterface;
use iced_widget::graphics::backend::Text;
use iced_widget::graphics::Viewport;

/// Basic re-exports for all Iced-related stuff.
///
/// This module attempts to emulate the `iced` package's API
/// as much as possible.
pub mod iced;

mod conversions;
mod render;
mod systems;
mod utils;

use systems::IcedEventQueue;

/// The default renderer.
pub type Renderer = iced_renderer::Renderer<iced::Theme>;

/// The main feature of `bevy_iced`.
/// Add this to your [`App`] by calling `app.add_plugin(bevy_iced::IcedPlugin::default())`.
#[derive(Default)]
pub struct IcedPlugin {
    /// The settings that Iced should use.
    pub settings: iced::Settings,
    /// Font file contents
    pub fonts: Vec<&'static [u8]>,
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
        let default_viewport = Viewport::with_physical_size(iced_core::Size::new(1600, 900), 1.0);
        let default_viewport = ViewportResource(default_viewport);
        let iced_resource: IcedResource = IcedProps::new(app, self).into();

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
    renderer: Renderer,
    debug: iced_runtime::Debug,
    clipboard: iced_core::clipboard::Null,
}

impl IcedProps {
    fn new(app: &App, config: &IcedPlugin) -> Self {
        let render_world = &app.sub_app(RenderApp).world;
        let device = render_world
            .get_resource::<RenderDevice>()
            .unwrap()
            .wgpu_device();
        let queue = render_world.get_resource::<RenderQueue>().unwrap();
        #[cfg(target_arch = "wasm32")]
        let format = iced_wgpu::wgpu::TextureFormat::Rgba8UnormSrgb;
        #[cfg(not(target_arch = "wasm32"))]
        let format = iced_wgpu::wgpu::TextureFormat::Bgra8UnormSrgb;
        let mut backend = iced_wgpu::Backend::new(device, queue, config.settings, format);
        for font in &config.fonts {
            backend.load_font(Cow::Borrowed(*font));
        }

        Self {
            renderer: Renderer::Wgpu(iced_wgpu::Renderer::new(backend)),
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

#[derive(Default)]
struct IcedCache {
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
    pub theme: iced_widget::style::Theme,
    /// The style to use for rendering Iced elements.
    pub style: iced::Style,
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
            theme: iced_widget::style::Theme::Dark,
            style: iced::Style {
                text_color: iced_core::Color::WHITE,
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
    touches: Res<'w, Touches>,
}

impl<'w, 's, M: bevy_ecs::event::Event> IcedContext<'w, 's, M> {
    /// Display an [`Element`] to the screen.
    pub fn display<'a>(&'a mut self, element: impl Into<iced_core::Element<'a, M, Renderer>>) {
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
                    Cursor::Available(utils::process_cursor_position(position, bounds, window))
                }
                None => utils::process_touch_input(self)
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
