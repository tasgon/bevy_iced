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

use crate::render::IcedPass;
use crate::systems::{IcedCamera, setup_iced_camera};
use bevy_app::prelude::*;
use bevy_core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_render::extract_component::ExtractComponentPlugin;
use bevy_render::prelude::*;
use bevy_render::render_graph::{RenderGraphApp, ViewNodeRunner};
use bevy_render::renderer::{RenderAdapter, RenderDevice, RenderQueue, render_system};
use bevy_render::{Render, RenderApp, RenderSet};
use bevy_winit::WakeUp;
use cfg_if::cfg_if;
use iced_core::Theme;
use iced_resource::IcedResource;
use iced_runtime::user_interface::UserInterface;
use iced_widget::graphics::Viewport;
pub use redraw_requestor::RedrawRequestVariant;
use redraw_requestor::{IcedRedrawRequest, RedrawRequestor};
use render::IcedViewport;
use std::borrow::Cow;
use std::marker::PhantomData;
use systems::{IcedCursor, IcedEventQueue};

/// Basic re-exports for all Iced-related stuff.
///
/// This module attempts to emulate the `iced` package's API
/// as much as possible.
pub mod iced;

mod conversions;
mod redraw_requestor;
mod render;
mod systems;
mod utils;

/// The default renderer.
pub type Renderer = iced_wgpu::Renderer;

/// The main feature of `bevy_iced`.
/// Add this to your [`App`] by calling `app.add_plugin(bevy_iced::IcedPlugin::<Message>::default())`.
///
/// `Message` is the type of of message that is produced by the UI.
/// `WinitUserEvent` is the UserEvent type for the Winit event loop.
/// If you are not overriding this type in the `WinitPlugin`, you don't need to set this manually.
pub struct IcedPlugin<Message, WinitUserEvent = WakeUp> {
    settings: iced::Settings,
    fonts: Vec<&'static [u8]>,
    _marker: PhantomData<(Message, WinitUserEvent)>,
}

impl<Message, WinitUserEvent> Default for IcedPlugin<Message, WinitUserEvent> {
    fn default() -> Self {
        Self {
            settings: Default::default(),
            fonts: Default::default(),
            _marker: PhantomData,
        }
    }
}

impl<Message, WinitUserEvent> IcedPlugin<Message, WinitUserEvent> {
    /// Set the Iced settings.
    pub fn settings(mut self, settings: iced::Settings) -> Self {
        self.settings = settings;
        self
    }

    /// Set the fonts to preload in Iced.
    pub fn fonts(mut self, fonts: Vec<&'static [u8]>) -> Self {
        self.fonts = fonts;
        self
    }
}

impl<M: Event, U: RedrawRequestVariant> Plugin for IcedPlugin<M, U> {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<IcedCamera>::default())
            .add_systems(
                PreUpdate,
                (
                    (systems::process_input, render::update_viewport)
                        .before(systems::iced_update::<M>),
                    systems::iced_update::<M>,
                ),
            )
            .init_resource::<DidDraw>()
            .init_resource::<IcedSettings>()
            .insert_non_send_resource::<Option<UserInterface<M, Theme, Renderer>>>(None)
            .init_resource::<IcedEventQueue>()
            .init_resource::<IcedCursor>()
            .init_resource::<IcedRedrawRequest>()
            .add_systems(Startup, setup_iced_camera)
            .configure_sets(Update, IcedProgramSet::View.after(IcedProgramSet::Update));

        app.sub_app_mut(RenderApp)
            .add_render_graph_node::<ViewNodeRunner<IcedPass>>(Core2d, IcedPass)
            .add_render_graph_edges(Core2d, (Node2d::EndMainPass, IcedPass));
    }

    fn finish(&self, app: &mut App) {
        let default_viewport = Viewport::with_physical_size(iced_core::Size::new(1600, 900), 1.0);
        let default_viewport = IcedViewport(default_viewport);
        let iced_resource: IcedResource = IcedProps::new(app, self).into();

        app.insert_resource(default_viewport.clone());
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                app.insert_non_send_resource(iced_resource.clone());
            } else {
                app.insert_resource(iced_resource.clone());
            }
        }

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(default_viewport)
            .add_systems(ExtractSchedule, render::extract_iced_data)
            .add_systems(
                Render,
                render::recall_staging_belt
                    .after(render_system)
                    .in_set(RenderSet::Render),
            );
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                render_app.world_mut().insert_non_send_resource(iced_resource);
            } else {
                render_app.world_mut().insert_resource(iced_resource);
            }
        }
    }
}

/// SystemSet for specifying which systems perform view and update logic.
#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub enum IcedProgramSet {
    /// The set of systems that update the UI state.
    Update,
    /// The system that renders the UI.
    View,
}

struct IcedProps {
    renderer: Renderer,
}

impl IcedProps {
    fn new<M, U>(app: &App, config: &IcedPlugin<M, U>) -> Self {
        let render_world = &app.sub_app(RenderApp).world();
        let device = render_world
            .get_resource::<RenderDevice>()
            .unwrap()
            .wgpu_device();
        let queue: &iced_wgpu::wgpu::Queue = render_world.get_resource::<RenderQueue>().unwrap();
        let adapter = render_world.get_resource::<RenderAdapter>().unwrap();
        let engine = iced_wgpu::Engine::new(
            adapter,
            device.clone(),
            queue.clone(),
            render::TEXTURE_FMT,
            Some(iced_wgpu::graphics::Antialiasing::MSAAx4),
        );

        for &font in &config.fonts {
            iced_graphics::text::font_system()
                .write()
                .expect("write lock on global FontSystem")
                .load_font(Cow::from(font));
        }

        Self {
            renderer: iced_wgpu::Renderer::new(
                engine,
                config.settings.default_font,
                config.settings.default_text_size,
            ),
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(private_interfaces)]
mod iced_resource {
    use super::*;

    use std::cell::{RefCell, RefMut};
    use std::rc::Rc;

    #[derive(Clone)]
    pub struct IcedResource(Rc<RefCell<IcedProps>>);

    impl IcedResource {
        pub fn lock(&self) -> RefMut<IcedProps> {
            self.0.borrow_mut()
        }
    }

    impl From<IcedProps> for IcedResource {
        fn from(value: IcedProps) -> Self {
            Self(Rc::new(RefCell::new(value)))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(private_interfaces)]
mod iced_resource {
    use super::*;

    use std::sync::{Arc, Mutex, MutexGuard};

    #[derive(Resource, Clone)]
    pub struct IcedResource(Arc<Mutex<IcedProps>>);

    impl IcedResource {
        pub fn lock(&self) -> MutexGuard<IcedProps> {
            self.0.lock().unwrap()
        }
    }

    impl From<IcedProps> for IcedResource {
        fn from(value: IcedProps) -> Self {
            Self(Arc::new(Mutex::new(value)))
        }
    }
}

/// Settings used to independently customize Iced rendering.
#[derive(Clone, Resource)]
pub struct IcedSettings {
    /// The scale factor to use for rendering Iced elements.
    /// Setting this to `None` defaults to using the `Window`s scale factor.
    pub scale_factor: Option<f64>,
    /// The theme to use for rendering Iced elements.
    pub theme: Theme,
    /// The style to use for rendering Iced elements.
    pub style: iced::Style,
    /// The order of the bevy iced camera. A higher value is drawn later, than a lower value.
    pub camera_order: isize,
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
            theme: Theme::Dark,
            style: iced::Style {
                text_color: iced_core::Color::WHITE,
            },
            camera_order: 10,
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
pub struct IcedContext<'w, 's, Message, WinitUserEvent = WakeUp>
where
    Message: bevy_ecs::event::Event,
    WinitUserEvent: RedrawRequestVariant,
{
    viewport: Res<'w, IcedViewport>,
    #[cfg(target_arch = "wasm32")]
    props: NonSend<'w, IcedResource>,
    #[cfg(not(target_arch = "wasm32"))]
    props: Res<'w, IcedResource>,
    settings: Res<'w, IcedSettings>,
    did_draw: ResMut<'w, DidDraw>,
    ui: NonSendMut<'w, Option<UserInterface<'static, Message, Theme, Renderer>>>,
    cursor: Res<'w, IcedCursor>,
    message_writer: EventWriter<'w, Message>,
    redraw_requestor: RedrawRequestor<'w, 's, WinitUserEvent>,
}

impl<M, U> IcedContext<'_, '_, M, U>
where
    M: bevy_ecs::event::Event,
    U: RedrawRequestVariant,
{
    /// Display an [`Element`] to the screen.
    pub fn display(&mut self, element: impl Into<iced_core::Element<'static, M, Theme, Renderer>>) {
        let &mut IcedProps {
            ref mut renderer, ..
        } = &mut *self.props.lock();
        let bounds = self.viewport.logical_size();

        // Rebuild the UI using the new element.
        let element = element.into();
        let cache = self
            .ui
            .take()
            .map(UserInterface::into_cache)
            .unwrap_or_default();
        let mut ui = UserInterface::build(element, bounds, cache, renderer);

        // Run the UI update function with a single redraw request.
        // This is necessary to account for widget state that depends on external state (like time).
        let mut messages = Vec::<M>::new();
        let events = [iced_core::Event::Window(
            iced_core::window::Event::RedrawRequested(iced_core::time::Instant::now()),
        )];
        let (state, _event_statuses) = ui.update(
            events.as_slice(),
            **self.cursor,
            renderer,
            &mut iced_core::clipboard::Null,
            &mut messages,
        );
        self.redraw_requestor.finish(state);
        self.message_writer.write_batch(messages);

        // Draw the UI.
        ui.draw(
            renderer,
            &self.settings.theme,
            &self.settings.style,
            **self.cursor,
        );
        *self.ui = Some(ui);
        self.did_draw
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
