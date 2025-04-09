use std::{
    borrow::Cow, 
    sync::{Arc, Mutex}
};

use bytemuck::{
    Pod, 
    Zeroable
};

use wgpu::{
    util::DeviceExt, Device, RenderPipeline, Surface
};
use winit::window::Window;

use crate::game::common::Vector2F;

use super::{AppData, EntityView};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
}

impl Vertex {
    fn from_position(x: f32, y: f32) -> Self {
        Vertex{
            _pos: [x, y, 1.0, 1.0]
        }
    }
}

// Define the uniform structure (color as vec4)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    color: [f32; 4],
}

fn create_ndc_rect_quad_vertices(x: f32, y: f32, w: f32, h: f32) -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = vec![
        Vertex::from_position(x, y),           // Bottom-left
        Vertex::from_position(x + w, y),       // Bottom-right
        Vertex::from_position(x + w, y + h),   // Top-right
        Vertex::from_position(x, y + h),       // Top-left
    ];
    
    let indices_data = vec![
        0, 1, 2, // First triangle
        2, 3, 0, // Second triangle
    ];

    (vertex_data, indices_data)
}

pub struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    render_pipeline: RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor::default(),
                None
            )
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];
        
        // Create uniform bind group layout.
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        
        let render_pipeline = Self::prepare_pipeline(
            &device,
            &surface,
            &adapter,
            &uniform_bind_group_layout
        );

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            render_pipeline,
            uniform_bind_group_layout
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn prepare_pipeline(
        device: &Device,
        surface: &Surface, 
        adapter: &wgpu::Adapter,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> RenderPipeline {
        // Load the shaders from disk
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Describe the vertex buffer layout.
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // The position attribute is at location 0 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let swapchain_capabilities = surface.get_capabilities(adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
    
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();

        // TODO maybe configure pipeline (surface dependant)
    }

    pub fn render(&mut self,
        app_data: &AppData
    ) {
        // Create texture view
        let surface_texture = self.surface.get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture.texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Renders a GREEN screen
        let mut encoder = self.device.create_command_encoder(&Default::default());

        // Create the renderpass which will clear the screen.
        {
            let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clear with a dark gray color.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // renderpass.push_debug_group("Prepare data for draw.");

            // If you wanted to call any drawing commands, they would go here.
            renderpass.set_pipeline(&self.render_pipeline);

            let aspect_ratio = self.size.width as f32 / self.size.height as f32;
            let scale_x = app_data.scale / aspect_ratio;
            let scale_y = app_data.scale;

            app_data.entities.iter().for_each(|ev| {

                let uniform = Uniforms { color: [ev.color[0], ev.color[1], ev.color[2], 1.0] };
                let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Entity Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniform]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
        
                // Create a bind group for this entity's uniform buffer.
                let entity_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Entity Bind Group"),
                    layout: &self.uniform_bind_group_layout, // stored during initialization
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    }],
                });

                // --- Create the rectangle vertex and index buffers ---
                let (vertices, indices) = create_ndc_rect_quad_vertices(
                    (ev.position.x - app_data.camera_position.x) * scale_x,
                    (ev.position.y - app_data.camera_position.y) * scale_y,
                    ev.size.x * scale_x,
                    ev.size.y * scale_y
                );

                let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Rect Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Rect Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        
                // Bind the uniform bind group (group 0).
                renderpass.set_bind_group(0, &entity_bind_group, &[]);

                renderpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                renderpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);


                // Draw the rectangle using 6 indices.
                renderpass.draw_indexed(0..indices.len() as u32, 0, 0..1);
            });   

            // renderpass.pop_debug_group();
            // renderpass.insert_debug_marker("Draw!");
        }

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}