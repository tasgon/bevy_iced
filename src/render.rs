use std::cell::RefCell;

use bevy::{
    prelude::FromWorld,
    render::{
        render_graph::Node,
        render_resource::{
            BindGroup, BindGroupLayout, Buffer, RenderPipeline, Sampler, Texture, TextureView,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::ExtractedWindows,
    },
};
use iced_native::{
    futures::{executor::LocalPool, task::SpawnExt},
    Size,
};
use iced_wgpu::{
    wgpu::{self},
    Viewport,
};

use crate::DrawFn;

pub const ICED_PASS: &'static str = "bevy_iced_pass";

const INDICES: &[u16] = &[0, 2, 1, 1, 2, 3];
const NUM_INDICES: u32 = 6;

pub struct IcedPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    index_buffer: Buffer,
}

impl FromWorld for IcedPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.get_resource::<RenderDevice>().unwrap();
        let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("bevy_iced shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("iced.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bevy_iced bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bevy_iced pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bevy_iced pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::bevy_default(),
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::OVER,
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let index_buffer = device.create_buffer_with_data(&wgpu::util::BufferInitDescriptor {
            label: Some("bevy_iced index buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            index_buffer,
        }
    }
}

pub struct IcedRenderData<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a TextureView,
    pub staging_belt: &'a mut wgpu::util::StagingBelt,
}

pub struct IcedNode {
    texture_data: Option<(Texture, TextureView, BindGroup)>,
    size: wgpu::Extent3d,
    viewport: Viewport,
}

impl IcedNode {
    pub fn new() -> Self {
        Self {
            texture_data: None,
            size: Default::default(),
            viewport: Viewport::with_physical_size(Size::new(100, 100), 1.0),
        }
    }
}

impl Node for IcedNode {
    fn update(&mut self, world: &mut bevy::prelude::World) {
        let window = world
            .get_resource::<ExtractedWindows>()
            .unwrap()
            .values()
            .next()
            .unwrap();
        let size = wgpu::Extent3d {
            width: window.physical_width,
            height: window.physical_height,
            depth_or_array_layers: 1,
        };

        if self.size != size || self.texture_data.is_none() {
            let iced_pipeline = world.get_resource::<IcedPipeline>().unwrap();
            let device = world.get_resource::<RenderDevice>().unwrap();
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bevy_iced tex"),
                size,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bevy_iced bg"),
                layout: &iced_pipeline.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&iced_pipeline.sampler),
                    },
                ],
            });

            self.texture_data = Some((texture, view, bind_group));
            self.viewport = Viewport::with_physical_size(Size::new(size.width, size.height), 1.0);
        }
    }

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let iced_pipeline = world.get_resource::<IcedPipeline>().unwrap();
        let queue = world.get_resource::<RenderQueue>().unwrap();
        let (_texture, view, bind_group) = self.texture_data.as_ref().unwrap();
        let draw_fns = world
            .get_non_send_resource::<RefCell<Vec<DrawFn>>>()
            .unwrap();

        let mut encoder =
            render_context
                .render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("bevy_iced encoder"),
                });

        let extracted_window = &world
            .get_resource::<ExtractedWindows>()
            .unwrap()
            .windows
            .values()
            .next()
            .unwrap();
        let swap_chain_texture = extracted_window
            .swap_chain_texture
            .as_ref()
            .unwrap()
            .clone();

        let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
        let mut pool = LocalPool::new();

        {
            {
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bevy_iced internal pass"),
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });
            }

            let mut render_data = IcedRenderData {
                encoder: &mut encoder,
                view,
                staging_belt: &mut staging_belt,
            };
            for f in &mut *draw_fns.borrow_mut() {
                (f)(world, render_context, &self.viewport, &mut render_data);
            }
            render_data.staging_belt.finish();
        }
        queue.submit(Some(encoder.finish()));
        pool.spawner().spawn(staging_belt.recall()).unwrap();
        pool.run_until_stalled();

        let mut pass =
            render_context
                .command_encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bevy_iced main pass"),
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: &swap_chain_texture,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

        pass.set_pipeline(&iced_pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_index_buffer(
            *iced_pipeline.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        pass.draw_indexed(0..NUM_INDICES, 0, 0..1);

        Ok(())
    }
}
