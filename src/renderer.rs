use crate::mesh::{Mesh, MeshType, Vertex};
use crate::state::AppState;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    pub mvp: [[f32; 4]; 4],          // 64 bytes, offset 0
    pub xy: [f32; 2],                 // 8 bytes, offset 64
    pub xy_offset: [f32; 2],          // 8 bytes, offset 72
    pub x_lfo_arg: f32,               // 4 bytes, offset 80
    pub x_lfo_amp: f32,               // 4 bytes, offset 84
    pub x_lfo_other: f32,             // 4 bytes, offset 88
    pub y_lfo_arg: f32,               // 4 bytes, offset 92
    pub y_lfo_amp: f32,               // 4 bytes, offset 96
    pub y_lfo_other: f32,             // 4 bytes, offset 100
    pub z_lfo_arg: f32,               // 4 bytes, offset 104
    pub z_lfo_amp: f32,               // 4 bytes, offset 108
    pub z_lfo_other: f32,             // 4 bytes, offset 112
    pub luma_key_level: f32,          // 4 bytes, offset 116
    pub invert_switch: f32,           // 4 bytes, offset 120
    pub b_w_switch: f32,              // 4 bytes, offset 124
    pub bright_switch: i32,           // 4 bytes, offset 128
    pub x_lfo_shape: i32,             // 4 bytes, offset 132
    pub y_lfo_shape: i32,             // 4 bytes, offset 136
    pub z_lfo_shape: i32,             // 4 bytes, offset 140
    pub x_ringmod_switch: i32,        // 4 bytes, offset 144
    pub y_ringmod_switch: i32,        // 4 bytes, offset 148
    pub z_ringmod_switch: i32,        // 4 bytes, offset 152
    pub x_phasemod_switch: i32,       // 4 bytes, offset 156
    pub y_phasemod_switch: i32,       // 4 bytes, offset 160
    pub z_phasemod_switch: i32,       // 4 bytes, offset 164
    pub luma_switch: i32,             // 4 bytes, offset 168
    pub width: i32,                   // 4 bytes, offset 172
    pub height: i32,                  // 4 bytes, offset 176
    pub audio_displacement: f32,      // 4 bytes, offset 180
    pub audio_z: f32,                 // 4 bytes, offset 184
    pub audio_wave_phase: f32,        // 4 bytes, offset 188 - wave phase for line undulation
    pub audio_wave_amp: f32,          // 4 bytes, offset 192 - wave amplitude from bass
    pub audio_wave_freq: f32,         // 4 bytes, offset 200 - wave frequency from audio energy
    pub _pad: [f32; 6],               // 24 bytes padding (total 224, matches WGSL alignment)
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline_triangles: wgpu::RenderPipeline,
    render_pipeline_lines: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    video_texture: wgpu::Texture,
    x_noise_texture: wgpu::Texture,
    y_noise_texture: wgpu::Texture,
    z_noise_texture: wgpu::Texture,
    sampler: wgpu::Sampler,
    current_mesh_type: MeshType,
    pub size: winit::dpi::PhysicalSize<u32>,
    // Video/source dimensions for aspect ratio
    pub video_width: u32,
    pub video_height: u32,
}

impl Renderer {
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        log::info!("Using adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Displacement Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/displace.wgsl").into()),
        });

        // Create textures
        let video_texture = Self::create_texture(&device, 640, 480, "video");
        let x_noise_texture = Self::create_texture(&device, 180, 120, "x_noise");
        let y_noise_texture = Self::create_texture(&device, 180, 120, "y_noise");
        let z_noise_texture = Self::create_texture(&device, 180, 120, "z_noise");

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create uniform buffer
        let uniforms = Uniforms {
            mvp: Mat4::IDENTITY.to_cols_array_2d(),
            xy: [0.0, 0.0],
            xy_offset: [0.0, 0.0],
            x_lfo_arg: 0.0,
            x_lfo_amp: 0.0,
            x_lfo_other: 0.0,
            y_lfo_arg: 0.0,
            y_lfo_amp: 0.0,
            y_lfo_other: 0.0,
            z_lfo_arg: 0.0,
            z_lfo_amp: 0.0,
            z_lfo_other: 0.0,
            luma_key_level: 0.0,
            invert_switch: 0.0,
            b_w_switch: 0.0,
            bright_switch: 0,
            x_lfo_shape: 0,
            y_lfo_shape: 0,
            z_lfo_shape: 0,
            x_ringmod_switch: 0,
            y_ringmod_switch: 0,
            z_ringmod_switch: 0,
            x_phasemod_switch: 0,
            y_phasemod_switch: 0,
            z_phasemod_switch: 0,
            luma_switch: 0,
            width: 640,
            height: 480,
            audio_displacement: 0.0,
            audio_z: 0.0,
            audio_wave_phase: 0.0,
            audio_wave_amp: 0.0,
            audio_wave_freq: 10.0,
            _pad: [0.0; 6],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("bind_group_layout"),
        });

        let bind_group = Self::create_bind_group(
            &device,
            &bind_group_layout,
            &uniform_buffer,
            &video_texture,
            &x_noise_texture,
            &y_noise_texture,
            &z_noise_texture,
            &sampler,
        );

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipelines (one for triangles, one for lines)
        let render_pipeline_triangles = Self::create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            surface_format,
            wgpu::PrimitiveTopology::TriangleList,
        );

        let render_pipeline_lines = Self::create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            surface_format,
            wgpu::PrimitiveTopology::LineList,
        );

        // Create initial mesh
        let mesh = Mesh::triangle_mesh(100, 640.0, 480.0);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline_triangles,
            render_pipeline_lines,
            vertex_buffer,
            vertex_count: mesh.vertices.len() as u32,
            uniform_buffer,
            bind_group,
            bind_group_layout,
            video_texture,
            x_noise_texture,
            y_noise_texture,
            z_noise_texture,
            sampler,
            current_mesh_type: MeshType::Triangles,
            size,
            video_width: 640,
            video_height: 480,
        }
    }

    fn create_texture(device: &wgpu::Device, width: u32, height: u32, label: &str) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform_buffer: &wgpu::Buffer,
        video_texture: &wgpu::Texture,
        x_noise_texture: &wgpu::Texture,
        y_noise_texture: &wgpu::Texture,
        z_noise_texture: &wgpu::Texture,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &video_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        &x_noise_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        &y_noise_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(
                        &z_noise_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some("bind_group"),
        })
    }

    fn create_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        format: wgpu::TextureFormat,
        topology: wgpu::PrimitiveTopology,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Get video dimensions for mesh generation
    pub fn video_dimensions(&self) -> (f32, f32) {
        (self.video_width as f32, self.video_height as f32)
    }

    pub fn update_mesh(&mut self, mesh: &Mesh) {
        if mesh.mesh_type != self.current_mesh_type || mesh.vertices.len() as u32 != self.vertex_count {
            self.vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.vertex_count = mesh.vertices.len() as u32;
            self.current_mesh_type = mesh.mesh_type;
        } else {
            self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));
        }
    }

    pub fn update_video_texture(&mut self, data: &[u8], width: u32, height: u32) {
        // Recreate texture if dimensions changed
        if width != self.video_width || height != self.video_height {
            self.video_width = width;
            self.video_height = height;
            self.video_texture = Self::create_texture(&self.device, width, height, "video");
            // Recreate bind group with new texture
            self.bind_group = Self::create_bind_group(
                &self.device,
                &self.bind_group_layout,
                &self.uniform_buffer,
                &self.video_texture,
                &self.x_noise_texture,
                &self.y_noise_texture,
                &self.z_noise_texture,
                &self.sampler,
            );
        }

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.video_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn update_noise_texture(&mut self, axis: usize, data: &[u8], width: u32, height: u32) {
        // Convert grayscale to RGBA
        let rgba: Vec<u8> = data.iter().flat_map(|&g| [g, g, g, 255]).collect();

        let texture = match axis {
            0 => &self.x_noise_texture,
            1 => &self.y_noise_texture,
            _ => &self.z_noise_texture,
        };

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn update_uniforms(&mut self, state: &AppState) {
        let params = state.calculate_render_params();

        // Use video dimensions for base coordinates
        let vw = self.video_width as f32;
        let vh = self.video_height as f32;
        let half_w = vw / 2.0;
        let half_h = vh / 2.0;

        // Create MVP matrix with correct aspect ratio
        let window_aspect = self.size.width as f32 / self.size.height as f32;
        let video_aspect = vw / vh;

        // Adjust projection to fit video aspect ratio into window
        let (proj_w, proj_h) = if window_aspect > video_aspect {
            // Window is wider than video - letterbox horizontally
            (half_h * window_aspect, half_h)
        } else {
            // Window is taller than video - letterbox vertically
            (half_w, half_w / window_aspect)
        };

        let projection = Mat4::orthographic_rh(-proj_w, proj_w, -proj_h, proj_h, -1000.0, 1000.0);

        let view = Mat4::from_translation(Vec3::new(0.0, 0.0, params.zoom))
            * Mat4::from_rotation_x(state.rotate_x)
            * Mat4::from_rotation_y(state.rotate_y)
            * Mat4::from_rotation_z(state.rotate_z);

        let model = Mat4::from_translation(Vec3::new(
            -half_w + state.global_x_displace,
            -half_h + state.global_y_displace,
            0.0,
        ));

        let mvp = projection * view * model;

        let uniforms = Uniforms {
            mvp: mvp.to_cols_array_2d(),
            xy: [params.displace_x, params.displace_y],
            xy_offset: [params.center_x, params.center_y],
            x_lfo_arg: state.x_lfo_arg,
            x_lfo_amp: params.x_lfo_amp,
            x_lfo_other: params.x_frequency,
            y_lfo_arg: state.y_lfo_arg,
            y_lfo_amp: params.y_lfo_amp,
            y_lfo_other: params.y_frequency,
            z_lfo_arg: state.z_lfo_arg,
            z_lfo_amp: params.z_lfo_amp,
            z_lfo_other: params.z_frequency,
            luma_key_level: params.luma_key_level,
            invert_switch: if state.invert { 1.0 } else { 0.0 },
            b_w_switch: if state.greyscale { 1.0 } else { 0.0 },
            bright_switch: if state.bright_switch { 1 } else { 0 },
            x_lfo_shape: state.x_lfo_shape,
            y_lfo_shape: state.y_lfo_shape,
            z_lfo_shape: state.z_lfo_shape,
            x_ringmod_switch: if state.x_ringmod { 1 } else { 0 },
            y_ringmod_switch: if state.y_ringmod { 1 } else { 0 },
            z_ringmod_switch: if state.z_ringmod { 1 } else { 0 },
            x_phasemod_switch: if state.x_phasemod { 1 } else { 0 },
            y_phasemod_switch: if state.y_phasemod { 1 } else { 0 },
            z_phasemod_switch: if state.z_phasemod { 1 } else { 0 },
            luma_switch: if state.luma_switch { 1 } else { 0 },
            width: state.width as i32,
            height: state.height as i32,
            audio_displacement: params.audio_displacement,
            audio_z: params.audio_z,
            audio_wave_phase: state.audio_wave_phase,
            audio_wave_amp: state.audio_wave_amp,
            audio_wave_freq: state.audio_wave_freq,
            _pad: [0.0; 6],
        };

        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let pipeline = match self.current_mesh_type {
                MeshType::Triangles => &self.render_pipeline_triangles,
                MeshType::HorizontalLines | MeshType::VerticalLines | MeshType::Grid => &self.render_pipeline_lines,
            };

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.vertex_count, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
