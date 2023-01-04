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

use std::any::{TypeId, Any};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::sync::Mutex;
use std::{sync::Arc};

use crate::render::IcedNode;
use crate::render::{ViewportResource};


use bevy::ecs::event::Event;
use bevy::ecs::system::{
    SystemParam,
};
use bevy::prelude::{
    EventWriter, Res, ResMut,
    Resource, NonSendMut,
};
use bevy::render::render_graph::RenderGraph;
use bevy::render::RenderStage;


use bevy::utils::HashMap;
use bevy::window::Windows;
use bevy::{
    prelude::{App, Plugin},
    render::{
        renderer::{RenderDevice},
        RenderApp,
    },
};

use iced::{user_interface, UserInterface};
pub use iced_native as iced;
use iced_native::{Debug, Program, Size};
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
        let iced_resource: IcedResource = IcedProps::new(app).into();

        app.add_system(systems::process_input)
            .add_system(render::update_viewport)
            .insert_resource(iced_resource.clone())
            .insert_non_send_resource(IcedCache::default())
            .insert_resource(IcedEventQueue::default())
            .insert_resource(default_viewport.clone());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(default_viewport);
        render_app.insert_resource(iced_resource);
        render_app.add_system_to_stage(RenderStage::Extract, render::extract_iced_data);
        // render_app.init_resource::<render::IcedPipeline>();
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
pub struct IcedResource(Arc<Mutex<IcedProps>>);

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

pub(crate) fn setup_pipeline(graph: &mut RenderGraph) {
    graph.add_node(render::ICED_PASS, IcedNode::new());

    graph
        .add_node_edge(
            bevy::render::main_graph::node::CAMERA_DRIVER,
            render::ICED_PASS,
        )
        .unwrap();
}

#[derive(Default)]
pub struct IcedCache {
    pub(crate) cache: HashMap<TypeId, Option<user_interface::Cache>>,
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

#[derive(SystemParam)]
pub struct Context<'w, 's, Message: Event> {
    viewport: Res<'w, ViewportResource>,
    props: Res<'w, IcedResource>,
    windows: Res<'w, Windows>,
    events: ResMut<'w, IcedEventQueue>,
    cache_map: NonSendMut<'w, IcedCache>,
    messages: EventWriter<'w, 's, Message>,
}

impl<'w, 's, M: Event + std::fmt::Debug> Context<'w, 's, M> {
    pub fn show<'a>(
        &'a mut self,
        element: impl Into<iced_native::Element<'a, M, iced_wgpu::Renderer>>,
    ) {
        let IcedProps {
            ref mut renderer,
            debug: _,
            ref mut clipboard,
        } = &mut *self.props.lock().unwrap();
        let bounds = self.viewport.logical_size();

        let element = element.into();

        let cursor_position = {
            let window = self.windows.get_primary().unwrap();
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

        let theme = iced_wgpu::Theme::Dark;
        let style = iced_native::renderer::Style {
            text_color: iced_native::Color::WHITE,
        };

        // println!("count: {}", messages.len());
        messages.into_iter().for_each(|msg| self.messages.send(msg));

        ui.draw(renderer, &theme, &style, cursor_position);

        *cache_entry = Some(ui.into_cache());
    }
}
