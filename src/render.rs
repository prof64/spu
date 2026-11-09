//! One renderer and WGSL program for native benchmarks and the browser.
use crate::runtime::{Emitter, Instance, Scene, Settings, battle_camera, battle_placements};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ReflectionCard {
    position: [f32; 4],
    appearance: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    viewport: [f32; 4],
    light: [f32; 4],
    environment: [f32; 4],
    probe: [f32; 4],
    effects: [f32; 4],
    emitters: [Emitter; 16],
    weather: [f32; 4],
    camera: [f32; 4],
    cards: [ReflectionCard; 4],
    world: [f32; 4],
}
impl Uniforms {
    pub fn new(s: &Settings, time: f32, w: u32, h: u32) -> Self {
        let sign = if s.view == 0 { 1.0 } else { -1.0 };
        let depth = if s.source_depth.abs() < 0.04 {
            0.04
        } else {
            s.source_depth.clamp(-2.0, 2.0)
        };
        let facing = -depth.signum() * sign;
        let camera = if s.battle {
            [battle_camera(w, h)[0], battle_camera(w, h)[1], 0.0, 1.0]
        } else if s.background == 7 && !s.reflection_focus {
            [0.45, 0.22, 1.0, sign]
        } else {
            [0.0, 0.0, 0.0, sign]
        };
        let trainer_size = if s.trainers { 0.075 } else { 0.0 };
        let mut result = Self {
            viewport: [w as f32, h as f32, time, s.mode as f32],
            light: [s.light[0], s.light[1], s.light[2], s.light_power],
            environment: [s.ambient, s.exposure, s.reflectivity, s.background as f32],
            probe: [
                s.probe[0],
                s.probe[1],
                u32::from(s.probe_front) as f32,
                u32::from(s.probe_enabled) as f32,
            ],
            effects: [s.bloom, s.spill, 0.0, u32::from(s.battle) as f32],
            emitters: [Emitter::default(); 16],
            weather: [
                s.rain.clamp(0.0, 2.0),
                s.wind.clamp(-1.0, 1.0),
                s.wetness.clamp(0.0, 1.0),
                if s.battle { 2.0 } else { s.count as f32 },
            ],
            camera,
            world: [
                if s.world {
                    1.0 + s.showcase.min(4) as f32
                } else {
                    0.0
                },
                s.reflection_steps.min(32) as f32,
                w.div_ceil(crate::world::TILE_SIZE) as f32,
                crate::world::TILE_SIZE as f32,
            ],
            cards: [
                ReflectionCard {
                    position: [0.0, -0.01, 0.0, 0.5],
                    appearance: [0.5, 0.0, 1.0, sign],
                },
                ReflectionCard {
                    position: [s.source_offset, 0.0, depth * sign, 0.6],
                    appearance: [
                        0.6,
                        if s.source_trainer { -1.0 } else { 2.0 },
                        if s.source_trainer { -1.0 } else { 3.0 },
                        facing,
                    ],
                },
                ReflectionCard {
                    position: [
                        s.source_offset + 0.10,
                        0.10,
                        (depth + depth.signum() * 0.65) * sign,
                        trainer_size,
                    ],
                    appearance: [0.14, -1.0, -1.0, facing],
                },
                ReflectionCard {
                    position: [0.05, 0.02, -0.8 * sign, trainer_size],
                    appearance: [0.14, -1.0, -1.0, sign],
                },
            ],
        };
        if s.battle {
            let positions = battle_placements(w, h);
            for (i, position) in positions.into_iter().enumerate() {
                result.cards[i] = ReflectionCard {
                    position,
                    appearance: [
                        position[3],
                        i as f32 * 2.0,
                        i as f32 * 2.0 + 1.0,
                        if i == 0 { 1.0 } else { -1.0 },
                    ],
                };
            }
            result.cards[2].position = [positions[1][0], positions[1][1], 1.3, trainer_size];
            result.cards[2].appearance[3] = -1.0;
            result.cards[3].position = [positions[0][0], positions[0][1], -1.3, trainer_size];
            result.cards[3].appearance[3] = 1.0;
        }
        if s.reference {
            // Internal background -1 is a neutral reference surface. Keep runtime placement
            // and animation identical, but bypass every material/weather/display effect.
            result.viewport[3] = 1.0;
            result.environment = [0.0, 1.0, 0.0, -1.0];
            result.probe = [0.0; 4];
            result.effects = [0.0; 4];
            result.weather = [0.0; 4];
            result.world = [0.0; 4];
        }
        result
    }
    pub fn with_emitters(mut self, emitters: &[Emitter]) -> Self {
        let count = emitters.len().min(16);
        self.emitters[..count].copy_from_slice(&emitters[..count]);
        self.effects[2] = count as f32;
        self
    }
}
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::AdapterInfo,
    uniform: wgpu::Buffer,
    uniform_group: wgpu::BindGroup,
    textures: wgpu::BindGroup,
    instances: wgpu::Buffer,
    capacity: usize,
    background: wgpu::RenderPipeline,
    sprite: wgpu::RenderPipeline,
    foreground: wgpu::RenderPipeline,
    display: wgpu::RenderPipeline,
    linear_view: wgpu::TextureView,
    linear_group: wgpu::BindGroup,
    linear_layout: wgpu::BindGroupLayout,
    linear_size: (u32, u32),
    bloom_views: [wgpu::TextureView; 2],
    bloom_groups: [wgpu::BindGroup; 2],
    bloom_layout: wgpu::BindGroupLayout,
    bloom_pipelines: [wgpu::RenderPipeline; 2],
    pub instance_count: u32,
    reflection_view: wgpu::TextureView,
    reflection_group: wgpu::BindGroup,
    reflection_buffer: wgpu::Buffer,
    reflection_pipeline: wgpu::RenderPipeline,
    pub reflection_count: u32,
    pub reflections_active: bool,
    pub world_active: bool,
    world_capture_pipeline: wgpu::RenderPipeline,
    world_radiance: wgpu::TextureView,
    world_geometry: wgpu::TextureView,
    world_lights: wgpu::Buffer,
    world_tiles: wgpu::Buffer,
    world_tile_scratch: Vec<u32>,
    reflection_layout: wgpu::BindGroupLayout,
}
impl Renderer {
    pub async fn request(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface,
                force_fallback_adapter: false,
            })
            .await?;
        let limits = wgpu::Limits {
            max_texture_dimension_2d: 4096,
            ..wgpu::Limits::default()
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Sprite Lab"),
                required_limits: limits,
                ..Default::default()
            })
            .await?;
        Ok((adapter, device, queue))
    }
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter: wgpu::AdapterInfo,
        format: wgpu::TextureFormat,
        scene: &Scene,
    ) -> Self {
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene controls"),
            contents: bytemuck::bytes_of(&Uniforms::new(&Settings::default(), 0.0, 1, 1)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let mut texture_entries: Vec<_> = (0..4)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            })
            .collect();
        texture_entries.push(wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shared material atlas"),
            entries: &texture_entries,
        });
        let views: Vec<_> = scene
            .atlas
            .iter()
            .enumerate()
            .map(|(i, image)| {
                let size = wgpu::Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                };
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(
                        [
                            "Color atlas",
                            "Normal atlas",
                            "Surface atlas",
                            "Emission atlas",
                        ][i],
                    ),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    image.as_raw(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(image.width() * 4),
                        rows_per_image: Some(image.height()),
                    },
                    size,
                );
                texture.create_view(&Default::default())
            })
            .collect();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Nearest pixel sampling"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let mut entries: Vec<_> = views
            .iter()
            .enumerate()
            .map(|(i, v)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(v),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });
        let textures = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_layout,
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Reference 2.5D material shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}",
                    include_str!("shader.wgsl"),
                    include_str!("environment.wgsl"),
                    include_str!("world.wgsl")
                )
                .into(),
            ),
        });
        let reflection_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Animated front/back reflection views"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 256,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let reflection_view = reflection_texture.create_view(&Default::default());
        let reflection_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Depth scene views"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let world_lights = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("World point lights"),
            size: (crate::world::MAX_LIGHTS * std::mem::size_of::<Emitter>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let world_tiles = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("64px light lists"),
            size: 64 * 64 * 9 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (world_radiance, world_geometry) = Self::world_targets(&device, 1, 1);
        let reflection_group = Self::scene_group(
            &device,
            &reflection_layout,
            &reflection_view,
            &world_radiance,
            &world_geometry,
            &world_lights,
            &world_tiles,
        );
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform_layout, &texture_layout, &reflection_layout],
            push_constant_ranges: &[],
        });
        let attributes = wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Float32x4,5=>Float32x4,6=>Float32x4];
        let buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        }];
        let make_pipeline = |vertex: &str, fragment: &str, instanced: bool, blend: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fragment),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: if instanced { &buffers } else { &[] },
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };
        let background = make_pipeline("fullscreen", "background", false, false);
        let sprite = make_pipeline("sprite_vertex", "sprite_fragment", true, true);
        let foreground = make_pipeline("fullscreen", "foreground", false, true);
        let capture_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform_layout, &texture_layout],
            push_constant_ranges: &[],
        });
        let reflection_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Capture animated color and emission; no recursive reflection"),
            layout: Some(&capture_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("capture_vertex"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("capture_fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let world_capture_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("World color and depth capture"),
                layout: Some(&capture_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("sprite_vertex"),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("world_capture_fragment"),
                    compilation_options: Default::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba16Float,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rg32Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });
        let reflection_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Four bounded reflection views"),
            size: 1024 * std::mem::size_of::<Instance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let texture_entry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let linear_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Linear frame and bloom"),
            entries: &[
                texture_entry,
                sampler_entry,
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    ..texture_entry
                },
            ],
        });
        let bloom_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bloom source"),
            entries: &[texture_entry, sampler_entry],
        });
        let (linear_view, linear_group, bloom_views, bloom_groups) =
            Self::linear_target(&device, 1, 1, &linear_layout, &bloom_layout);
        let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Quarter resolution HDR bloom"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        });
        let bloom_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bloom_layout],
                push_constant_ranges: &[],
            });
        let bloom_pipelines = ["horizontal", "vertical"].map(|entry| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&bloom_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &bloom_shader,
                    entry_point: Some("fullscreen"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &bloom_shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        });
        let display_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Display transform"),
            source: wgpu::ShaderSource::Wgsl(include_str!("display.wgsl").into()),
        });
        let display_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform_layout, &linear_layout],
            push_constant_ranges: &[],
        });
        let display = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Linear composite to display"),
            layout: Some(&display_layout),
            vertex: wgpu::VertexState {
                module: &display_shader,
                entry_point: Some("fullscreen"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &display_shader,
                entry_point: Some("display"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let capacity = 256;
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batched sprite parts"),
            size: (capacity * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            device,
            queue,
            adapter,
            uniform,
            uniform_group,
            textures,
            instances,
            capacity,
            background,
            sprite,
            foreground,
            display,
            linear_view,
            linear_group,
            linear_layout,
            linear_size: (1, 1),
            bloom_views,
            bloom_groups,
            bloom_layout,
            bloom_pipelines,
            instance_count: 0,
            reflection_view,
            reflection_group,
            reflection_buffer,
            reflection_pipeline,
            reflection_count: 0,
            reflections_active: false,
            world_active: false,
            world_capture_pipeline,
            world_radiance,
            world_geometry,
            world_lights,
            world_tiles,
            world_tile_scratch: Vec::new(),
            reflection_layout,
        }
    }
    fn world_targets(
        device: &wgpu::Device,
        w: u32,
        h: u32,
    ) -> (wgpu::TextureView, wgpu::TextureView) {
        let make = |format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("World reflection source"),
                    size: wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        (
            make(wgpu::TextureFormat::Rgba16Float),
            make(wgpu::TextureFormat::Rg32Float),
        )
    }
    fn scene_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        reflection: &wgpu::TextureView,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        lights: &wgpu::Buffer,
        tiles: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene reflection and lights"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(reflection),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(color),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: lights.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: tiles.as_entire_binding(),
                },
            ],
        })
    }
    pub fn upload_world_lights(&mut self, lights: &[Emitter], w: u32, h: u32) {
        let lights = &lights[..lights.len().min(crate::world::MAX_LIGHTS)];
        if !lights.is_empty() {
            self.queue
                .write_buffer(&self.world_lights, 0, bytemuck::cast_slice(lights));
        }
        crate::world::light_grid(lights, w, h, &mut self.world_tile_scratch);
        self.queue.write_buffer(
            &self.world_tiles,
            0,
            bytemuck::cast_slice(&self.world_tile_scratch),
        );
    }
    fn linear_target(
        device: &wgpu::Device,
        w: u32,
        h: u32,
        layout: &wgpu::BindGroupLayout,
        bloom_layout: &wgpu::BindGroupLayout,
    ) -> (
        wgpu::TextureView,
        wgpu::BindGroup,
        [wgpu::TextureView; 2],
        [wgpu::BindGroup; 2],
    ) {
        let make_texture = |w, h| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Linear HDR composite"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let texture = make_texture(w, h);
        let view = texture.create_view(&Default::default());
        let bloom_views = std::array::from_fn(|_| {
            make_texture(w.div_ceil(4), h.div_ceil(4)).create_view(&Default::default())
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Linear bloom sampling"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bloom_groups = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: bloom_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(if i == 0 {
                            &view
                        } else {
                            &bloom_views[0]
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            })
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&bloom_views[1]),
                },
            ],
        });
        (view, group, bloom_views, bloom_groups)
    }
    pub fn upload(&mut self, instances: &[Instance], uniforms: &Uniforms) {
        self.world_active = uniforms.world[0] > 0.5;
        self.reflections_active =
            !self.world_active && (uniforms.environment[3] >= 6.0 || uniforms.effects[3] > 0.5);
        let size = (uniforms.viewport[0] as u32, uniforms.viewport[1] as u32);
        if size != self.linear_size {
            (
                self.linear_view,
                self.linear_group,
                self.bloom_views,
                self.bloom_groups,
            ) = Self::linear_target(
                &self.device,
                size.0,
                size.1,
                &self.linear_layout,
                &self.bloom_layout,
            );
            (self.world_radiance, self.world_geometry) =
                Self::world_targets(&self.device, size.0, size.1);
            self.reflection_group = Self::scene_group(
                &self.device,
                &self.reflection_layout,
                &self.reflection_view,
                &self.world_radiance,
                &self.world_geometry,
                &self.world_lights,
                &self.world_tiles,
            );
            self.linear_size = size;
        }
        if instances.len() > self.capacity {
            self.capacity = instances.len().next_power_of_two().min(
                (self.device.limits().max_buffer_size / std::mem::size_of::<Instance>() as u64)
                    as usize,
            );
            self.instances = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Batched sprite parts"),
                size: (self.capacity * std::mem::size_of::<Instance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue
            .write_buffer(&self.uniform, 0, bytemuck::bytes_of(uniforms));
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instances, 0, bytemuck::cast_slice(instances));
        }
        self.instance_count = instances.len() as u32;
    }
    pub fn upload_reflections(&mut self, instances: &[Instance]) {
        assert!(instances.len() <= 1024, "Four views of at most 256 parts");
        self.reflection_count = instances.len() as u32;
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.reflection_buffer, 0, bytemuck::cast_slice(instances));
        }
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        if self.world_active {
            let mut capture = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("World radiance and depth"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.world_radiance,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.world_geometry,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: -1000.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                ..Default::default()
            });
            capture.set_pipeline(&self.world_capture_pipeline);
            capture.set_bind_group(0, &self.uniform_group, &[]);
            capture.set_bind_group(1, &self.textures, &[]);
            capture.set_vertex_buffer(0, self.instances.slice(..));
            capture.draw(0..6, 0..self.instance_count);
        }
        if self.reflections_active {
            let mut capture = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Capture front and back views"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.reflection_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            capture.set_pipeline(&self.reflection_pipeline);
            capture.set_bind_group(0, &self.uniform_group, &[]);
            capture.set_bind_group(1, &self.textures, &[]);
            capture.set_vertex_buffer(0, self.reflection_buffer.slice(..));
            capture.draw(0..6, 0..self.reflection_count);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Reference renderer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.linear_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_bind_group(0, &self.uniform_group, &[]);
        pass.set_bind_group(1, &self.textures, &[]);
        pass.set_bind_group(2, &self.reflection_group, &[]);
        pass.set_pipeline(&self.background);
        pass.draw(0..3, 0..1);
        pass.set_pipeline(&self.sprite);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        pass.draw(0..6, 0..self.instance_count);
        pass.set_pipeline(&self.foreground);
        pass.draw(0..3, 0..1);
        drop(pass);
        for i in 0..2 {
            let mut bloom = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HDR bloom"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_views[i],
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            bloom.set_pipeline(&self.bloom_pipelines[i]);
            bloom.set_bind_group(0, &self.bloom_groups[i], &[]);
            bloom.draw(0..3, 0..1);
        }
        let mut display = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Display encoding"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        display.set_pipeline(&self.display);
        display.set_bind_group(0, &self.uniform_group, &[]);
        display.set_bind_group(1, &self.linear_group, &[]);
        display.draw(0..3, 0..1);
    }
    pub fn draw(&self, target: &wgpu::TextureView) -> wgpu::SubmissionIndex {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.encode(&mut encoder, target);
        self.queue.submit([encoder.finish()])
    }
}
