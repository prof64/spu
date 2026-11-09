#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    native::run()
}
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use anyhow::{Context, Result, ensure};
    use serde_json::json;
    use sprite_lab::{
        format::Package,
        render::{Renderer, Uniforms},
        runtime::{Scene, Settings},
    };
    use std::{
        path::PathBuf,
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    fn stats(mut samples: Vec<f64>) -> serde_json::Value {
        samples.sort_by(f64::total_cmp);
        let n = samples.len();
        json!({"medianMs":samples[n/2],"p95Ms":samples[((n as f64*0.95).ceil()as usize).saturating_sub(1).min(n-1)],"minMs":samples[0],"maxMs":samples[n-1],"samples":n})
    }
    fn fingerprint(chunks: &[&[u8]]) -> String {
        let mut h = 0xcbf29ce484222325u64;
        for chunk in chunks {
            for &byte in *chunk {
                h ^= byte as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        }
        format!("fnv1a64:{h:016x}")
    }
    fn wait(renderer: &Renderer) -> Result<()> {
        renderer.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(20)),
        })?;
        Ok(())
    }
    fn snapshot(
        renderer: &Renderer,
        texture: &wgpu::Texture,
        path: &std::path::Path,
        w: u32,
        h: u32,
    ) -> Result<()> {
        let stride = (w * 4).div_ceil(256) * 256;
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Snapshot readback"),
            size: (stride * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        wait(renderer)?;
        rx.recv()??;
        let data = buffer.slice(..).get_mapped_range();
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for row in data.chunks(stride as usize) {
            pixels.extend_from_slice(&row[..(w * 4) as usize]);
        }
        image::save_buffer(path, &pixels, w, h, image::ColorType::Rgba8)?;
        drop(data);
        buffer.unmap();
        Ok(())
    }
    pub fn run() -> Result<()> {
        let mut args = std::env::args().skip(1);
        let mut paths = Vec::new();
        let mut output = PathBuf::from("web/fixtures/benchmark.json");
        let mut snapshots = None;
        let mut environment = Settings::default();
        let mut scene_name = "studio".to_string();
        let mut frames = 60usize;
        let (mut w, mut h) = (1280u32, 720u32);
        let mut counts = vec![1, 100, 1000, 5000, 10000];
        let mut battle = false;
        let mut world = false;
        let mut lights = 32u32;
        let mut reflection_steps = 16u32;
        let mut world_size = 48.0f32;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--battle" => battle = true,
                "--world" => world = true,
                "--lights" => {
                    lights = args
                        .next()
                        .context("--lights needs count")?
                        .parse::<u32>()?
                        .min(512)
                }
                "--reflection-steps" => {
                    reflection_steps = args
                        .next()
                        .context("--reflection-steps needs count")?
                        .parse::<u32>()?
                        .min(32)
                }
                "--object-size" => {
                    world_size = args
                        .next()
                        .context("--object-size needs pixels")?
                        .parse::<f32>()?
                        .clamp(16.0, 160.0)
                }
                "--scene" => {
                    scene_name = args
                        .next()
                        .context("--scene needs studio, cave, reflections, rain, or depth")?;
                    environment = match scene_name.as_str() {
                        "studio" => Settings::default(),
                        "cave" => Settings {
                            background: 4,
                            ambient: 0.07,
                            light_power: 0.05,
                            probe_enabled: false,
                            emission: 1.6,
                            transmission: 0.4,
                            bloom: 0.9,
                            spill: 1.4,
                            ..Default::default()
                        },
                        "reflections" => Settings {
                            background: 5,
                            probe_front: true,
                            ambient: 0.2,
                            light_power: 0.12,
                            probe: [0.5, 0.5],
                            emission: 0.15,
                            transmission: 0.0,
                            reflectivity: 2.0,
                            roughness: 0.7,
                            normals: 0.4,
                            bloom: 0.2,
                            ..Default::default()
                        },
                        "rain" => Settings {
                            background: 6,
                            ambient: 0.65,
                            light_power: 1.0,
                            probe_enabled: false,
                            emission: 1.1,
                            transmission: 0.6,
                            reflectivity: 1.8,
                            bloom: 0.45,
                            ..Default::default()
                        },
                        "depth" => Settings {
                            background: 7,
                            ambient: 0.28,
                            light_power: 0.35,
                            probe_enabled: false,
                            emission: 0.15,
                            transmission: 0.0,
                            reflectivity: 4.0,
                            roughness: 0.35,
                            normals: 0.18,
                            bloom: 0.2,
                            ..Default::default()
                        },
                        _ => anyhow::bail!("Unknown scene: {scene_name}"),
                    };
                }
                "--output" => output = PathBuf::from(args.next().context("--output needs path")?),
                "--snapshots" => {
                    snapshots = Some(PathBuf::from(
                        args.next().context("--snapshots needs path")?,
                    ))
                }
                "--frames" => {
                    frames = args
                        .next()
                        .context("--frames needs count")?
                        .parse::<usize>()?;
                    ensure!((5..=36000).contains(&frames), "--frames must be 5–36000");
                }
                "--counts" => {
                    counts = args
                        .next()
                        .context("--counts needs comma list")?
                        .split(',')
                        .map(str::parse)
                        .collect::<std::result::Result<_, _>>()?
                }
                "--size" => {
                    let size = args.next().context("--size needs WIDTHxHEIGHT")?;
                    let (a, b) = size.split_once('x').context("Use WIDTHxHEIGHT")?;
                    w = a.parse()?;
                    h = b.parse()?;
                }
                "--quick" => {
                    frames = 12;
                    counts = vec![1, 100, 1000];
                }
                "--help" => {
                    println!(
                        "benchmark [assets.sprite ...] [--scene studio|cave|reflections|rain|depth] [--battle|--world] [--lights 32] [--reflection-steps 16] [--object-size 48] [--output file.json] [--snapshots directory] [--frames 60] [--counts 1,100,1000,5000,10000] [--size 1280x720] [--quick]"
                    );
                    return Ok(());
                }
                _ => paths.push(PathBuf::from(arg)),
            }
        }
        anyhow::ensure!(!(battle && world), "Choose battle or world");
        if world {
            environment.world = true;
            environment.lights = lights;
            environment.reflection_steps = reflection_steps;
            environment.world_size = world_size;
            environment.probe_enabled = false;
            let rain = scene_name == "rain";
            environment.ambient = if rain {
                0.27
            } else if scene_name == "studio" {
                0.65
            } else {
                0.10
            };
            environment.light_power = if rain {
                0.75
            } else if scene_name == "studio" {
                1.1
            } else {
                0.15
            };
            environment.emission = if rain { 1.1 } else { 1.6 };
            environment.transmission = if rain { 0.65 } else { 1.0 };
            environment.reflectivity = if rain { 2.0 } else { 1.5 };
            environment.bloom = if rain { 0.45 } else { 0.7 };
            environment.spill = 1.3;
            scene_name = format!("world-{scene_name}");
        }
        if battle {
            counts = vec![2];
            environment.battle = true;
            environment.partner = 1;
            environment.probe_enabled = false;
            scene_name = format!("battle-{scene_name}");
        }
        anyhow::ensure!(
            w > 0 && h > 0 && w <= 4096 && h <= 4096,
            "Viewport must be 1–4096 per dimension"
        );
        anyhow::ensure!(
            counts.iter().all(|&n| (1..=10000).contains(&n)),
            "Counts must be 1–10000"
        );
        if paths.is_empty() {
            let catalog: serde_json::Value =
                serde_json::from_slice(&std::fs::read("web/examples/catalog.json")?)?;
            paths = catalog["assets"]
                .as_array()
                .context("Example catalog")?
                .iter()
                .map(|a| PathBuf::from("web").join(a["file"].as_str().unwrap()))
                .collect();
        }
        let mut packages = Vec::new();
        let mut loads = Vec::new();
        for path in paths {
            let bytes = std::fs::read(&path).with_context(|| format!("Read {}", path.display()))?;
            let mut samples = Vec::new();
            for _ in 0..12 {
                let start = Instant::now();
                std::hint::black_box(Scene::load(vec![Package::read(&bytes)?])?);
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            let package = Package::read(&bytes)?;
            loads.push(json!({"asset":package.document.name,"path":path,"contentFingerprint":fingerprint(&[&bytes]),"compressedBytes":bytes.len(),"archiveAndTextureDecode":stats(samples)}));
            packages.push(package);
        }
        let mut scene = Scene::load(packages)?;
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let (adapter, device, queue) = pollster::block_on(Renderer::request(&instance, None))?;
        let adapter_info = adapter.get_info();
        let mut renderer = Renderer::new(
            device,
            queue,
            adapter_info.clone(),
            wgpu::TextureFormat::Rgba8Unorm,
            &scene,
        );
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Benchmark target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let mut instances = Vec::new();
        let mut rows = Vec::new();
        eprintln!(
            "{} · {:?} · {}×{} · {} measured frames/case",
            adapter_info.name, adapter_info.backend, w, h, frames
        );
        for overlap in [false, true] {
            if (battle || world) && overlap {
                continue;
            }
            for &count in &counts {
                if overlap && count == 1 {
                    continue;
                }
                let settings = Settings {
                    count,
                    mixed: true,
                    overlap,
                    ..environment.clone()
                };
                let (mut cpu, mut submit, mut completed) = (Vec::new(), Vec::new(), Vec::new());
                for frame in 0..frames + 12 {
                    let start = Instant::now();
                    let time = frame as f32 / 60.0;
                    scene.prepare(&settings, time, w, h, &mut instances);
                    let prepared = Instant::now();
                    if world {
                        renderer.upload_world_lights(&scene.emitters, w, h);
                    }
                    renderer.upload_reflections(&scene.reflection_instances);
                    renderer.upload(
                        &instances,
                        &Uniforms::new(&settings, time, w, h).with_emitters(&scene.emitters),
                    );
                    renderer.draw(&view);
                    let submitted = Instant::now();
                    wait(&renderer)?;
                    if frame >= 12 {
                        cpu.push((prepared - start).as_secs_f64() * 1000.0);
                        submit.push((submitted - prepared).as_secs_f64() * 1000.0);
                        completed.push(start.elapsed().as_secs_f64() * 1000.0);
                    }
                }
                let completed = stats(completed);
                eprintln!(
                    "{} sprites · {} · {:.2} ms median / {:.2} ms p95 · {} parts",
                    count,
                    if world {
                        "world fixed size"
                    } else if battle {
                        "face to face"
                    } else if overlap {
                        "overlap 64px"
                    } else {
                        "fit to viewport"
                    },
                    completed["medianMs"].as_f64().unwrap(),
                    completed["p95Ms"].as_f64().unwrap(),
                    instances.len()
                );
                rows.push(json!({"sprites":count,"layout":if world{"world"}else if battle{"battle"}else if overlap{"overlap-64px"}else{"fit"},"visibleParts":instances.len(),"instanceBytesPerFrame":instances.len()*std::mem::size_of::<sprite_lab::runtime::Instance>(),"drawCalls":6+u32::from(renderer.reflections_active||renderer.world_active),"terrainTiles":scene.terrain_tiles,"activeLights":if world{scene.emitters.len()}else{0},"lightTileSize":64,"maxLightsPerTile":8,"reflectionSteps":if world{reflection_steps}else{0},"worldCaptureParts":if world{instances.len()}else{0},"reflectionCaptureInstances":renderer.reflection_count,"animationAndBatch":stats(cpu),"uploadAndSubmit":stats(submit),"completedFrame":completed}));
            }
        }
        if let Some(dir) = snapshots {
            std::fs::create_dir_all(&dir)?;
            for asset in 0..if world { 1 } else { scene.assets.len() } {
                for (label, mode, front) in [
                    ("material", 0, false),
                    ("color", 1, false),
                    ("normals", 2, false),
                    ("foreground", 0, true),
                ] {
                    let settings = Settings {
                        count: if world { counts[0] } else { 1 },
                        asset,
                        mode,
                        probe_front: front,
                        ..environment.clone()
                    };
                    scene.prepare(&settings, 0.0, w, h, &mut instances);
                    if world {
                        renderer.upload_world_lights(&scene.emitters, w, h);
                    }
                    renderer.upload_reflections(&scene.reflection_instances);
                    renderer.upload(
                        &instances,
                        &Uniforms::new(&settings, 0.0, w, h).with_emitters(&scene.emitters),
                    );
                    renderer.draw(&view);
                    wait(&renderer)?;
                    snapshot(
                        &renderer,
                        &texture,
                        &dir.join(format!("asset-{asset}-{label}.png")),
                        w,
                        h,
                    )?;
                }
            }
        }
        let source_fingerprint = fingerprint(&[
            include_bytes!("../format.rs"),
            include_bytes!("../runtime.rs"),
            include_bytes!("../render.rs"),
            include_bytes!("../shader.wgsl"),
            include_bytes!("../display.wgsl"),
            include_bytes!("../bloom.wgsl"),
            include_bytes!("../environment.wgsl"),
            include_bytes!("../world.wgsl"),
            include_bytes!("../world.rs"),
            include_bytes!("benchmark.rs"),
            include_bytes!("../../Cargo.lock"),
        ]);
        let mut json = json!({"kind":"sprite-lab-native-benchmark","version":1,"scene":scene_name,"sceneSettings":environment,"sourceFingerprint":source_fingerprint,"createdUnix":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            "adapter":{"name":adapter_info.name,"backend":format!("{:?}",adapter_info.backend),"deviceType":format!("{:?}",adapter_info.device_type),"driver":adapter_info.driver,"driverInfo":adapter_info.driver_info},
            "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"release":!cfg!(debug_assertions),"viewport":[w,h],"warmupFrames":12,"measuredFrames":frames,
            "textureBytes":scene.texture_bytes(),"framebufferBytes":w*h*24+w.div_ceil(4)*h.div_ceil(4)*16+1024*256*8,"atlas":[scene.atlas[0].width(),scene.atlas[0].height()],"load":loads,"results":rows,
            "method":"Monotonic wall clock. Each completed frame includes independent phased animation, batch preparation, buffer upload, GPU submission, and blocking GPU completion. No v-sync; no frame screenshots in timed samples. This is not an isolated GPU timestamp. Fit uses shrinking sprites, overlap uses 64px-tall sprites. Texture upload and pipeline compilation excluded from steady-state cases; archive loading measured separately. All sprites on screen; full material shader; Six main draw calls including backdrop/foreground, two HDR bloom passes and display conversion. Rain/depth add one draw capturing four animated front/back views shared by the scene. Captures and uploads are timed. Crowd depth uses shared environment cards, not a separate opponent for each sprite; the single-sprite wet floor reflection is disabled in crowds."});
        if battle {
            json["method"] = json!(
                "Monotonic wall clock from animation preparation through completed GPU work; 12 warm-up frames, no v-sync and no screenshots in timed samples. Two fully material-shaded rigs, first asset Front on the far side and second asset Back near the camera. Shared animation time, four animated front/back color/emission captures, one-bounce reflections, the selected environment and HDR bloom are included. This is one face-to-face battle, not a crowd throughput benchmark. Pipeline creation and texture upload are excluded from steady-state timing; archive loading is measured separately."
            );
        }
        if world {
            json["method"] = json!(
                "Completed frame wall time includes animation of all objects, depth ordering, material preparation, strongest luminous part per object, spatial light-list building, uploads, full-scene color/depth capture, shading, one-bounce screen-space reflection ray marching, bloom and blocking GPU completion. Fixed-size canvases (worldSize in settings), all objects on screen, plus 48px terrain tiles. Overlap grows with population; sprites are never shrunk or culled. Up to the requested number of evenly distributed emissive objects become local lights (actual count reported), eight strongest local lights per 64px tile. Reflection steps and requested light count are recorded. Roughness >=0.85 skips coherent reflections; rain wetness lowers the floor roughness so the wet-floor test traces those pixels too. Reflections use visible color/depth only; no offscreen, hidden-side, recursive or shadow rays. Capture retains current-view color and emission and stays active for transmission even when reflection steps are zero. GPU/animation rendering workload only: excludes game logic, collision, networking, audio, asset streaming and pipeline/texture creation. No v-sync or screenshots in timed cases."
            );
        }
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&output, serde_json::to_vec_pretty(&json)?)?;
        println!("Saved {}", output.display());
        Ok(())
    }
}
