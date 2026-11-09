//! Run explicitly on a machine with a GPU: cargo test --test renderer -- --ignored
use sprite_lab::{
    format::{Ease, Key, Package},
    render::{Renderer, Uniforms},
    runtime::{Scene, Settings},
};

#[test]
#[ignore = "requires an actual graphics adapter"]
fn shared_gpu_renderer_responds_to_animation_and_materials() {
    let original = Package::read(include_bytes!("../web/examples/copperbot.sprite")).unwrap();
    // A public, synthetic direction fixture: red front, blue back, same animated rig.
    let mut directional =
        Package::read(include_bytes!("../web/examples/copperbot.sprite")).unwrap();
    let mut views = Vec::new();
    for (side, color) in [("Front", [255, 12, 6]), ("Back", [6, 12, 255])] {
        let mut view = directional.document.views[0].clone();
        view.id = side.into();
        view.name = side.into();
        for drawing in &mut view.images {
            let mut image = image::load_from_memory(&directional.entries[&drawing.color])
                .unwrap()
                .to_rgba8();
            for pixel in image.pixels_mut() {
                pixel.0[..3].copy_from_slice(&color);
            }
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(image)
                .write_to(&mut bytes, image::ImageOutputFormat::Png)
                .unwrap();
            drawing.color = format!("{side}/{}", drawing.color);
            drawing.emission = None;
            directional
                .entries
                .insert(drawing.color.clone(), bytes.into_inner());
        }
        views.push(view);
    }
    directional.document.views = views;
    let mut hidden = Package::read(include_bytes!("../web/examples/copperbot.sprite")).unwrap();
    for view in &mut hidden.document.views {
        for clip in &mut view.animations {
            for track in &mut clip.tracks {
                track.visible = vec![Key {
                    tick: 0.0,
                    value: false,
                    ease: Ease::Step,
                }];
            }
        }
    }
    let mut scene = Scene::load(vec![
        original,
        directional,
        hidden,
        Package::read(include_bytes!("../web/examples/moss-stone.sprite")).unwrap(),
        Package::read(include_bytes!("../web/examples/lantern.sprite")).unwrap(),
        Package::read(include_bytes!("../web/examples/moss-scout.sprite")).unwrap(),
        Package::read(include_bytes!("../web/examples/tin-beetle.sprite")).unwrap(),
        Package::read(include_bytes!("../web/examples/glass-vial.sprite")).unwrap(),
    ])
    .unwrap();
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let (adapter, device, queue) = pollster::block_on(Renderer::request(&instance, None)).unwrap();
    let mut renderer = Renderer::new(
        device,
        queue,
        adapter.get_info(),
        wgpu::TextureFormat::Rgba8Unorm,
        &scene,
    );
    let size = wgpu::Extent3d {
        width: 256,
        height: 192,
        depth_or_array_layers: 1,
    };
    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 192 * 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut instances = Vec::new();
    let mut render = |s: Settings, time: f32| {
        scene.prepare(&s, time, 256, 192, &mut instances);
        if s.world {
            renderer.upload_world_lights(&scene.emitters, 256, 192);
        }
        renderer.upload_reflections(&scene.reflection_instances);
        renderer.upload(
            &instances,
            &Uniforms::new(&s, time, 256, 192).with_emitters(&scene.emitters),
        );
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        renderer.encode(&mut encoder, &target.create_view(&Default::default()));
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(1024),
                    rows_per_image: Some(192),
                },
            },
            size,
        );
        renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        renderer
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let pixels = readback.slice(..).get_mapped_range().to_vec();
        readback.unmap();
        pixels
    };
    let baseline = render(Settings::default(), 0.0);
    let mut changed = vec![
        Settings {
            light: [0.85, 0.8, 0.3],
            ..Default::default()
        },
        Settings {
            emission: 0.0,
            ..Default::default()
        },
        Settings {
            transmission: 0.0,
            ..Default::default()
        },
        Settings {
            roughness: 0.1,
            ..Default::default()
        },
        Settings {
            opacity: 0.3,
            ..Default::default()
        },
        Settings {
            normals: 0.0,
            ..Default::default()
        },
        Settings {
            reflectivity: 0.0,
            ..Default::default()
        },
        Settings {
            probe_front: true,
            ..Default::default()
        },
    ];
    for s in changed.drain(..) {
        let pixels = render(s.clone(), 0.0);
        let different = baseline.iter().zip(&pixels).filter(|(a, b)| a != b).count();
        assert!(
            different > 40,
            "Control did not affect enough pixels: {s:?}, {different}"
        );
    }
    assert_ne!(baseline, render(Settings::default(), 0.6));
    let cave = Settings {
        background: 4,
        ambient: 0.07,
        light_power: 0.0,
        emission: 3.0,
        probe_enabled: false,
        bloom: 0.0,
        spill: 3.0,
        ..Default::default()
    };
    let illuminated = render(cave.clone(), 0.0);
    let no_spill = render(
        Settings {
            spill: 0.0,
            ..cave.clone()
        },
        0.0,
    );
    // Cave illumination must reach pixels outside the sprite, not merely brighten its texture.
    let outside_changes = illuminated
        .chunks_exact(4)
        .zip(no_spill.chunks_exact(4))
        .enumerate()
        .filter(|(i, (a, b))| {
            let (x, y) = (i % 256, i / 256);
            (x < 64 || x > 192 || y > 170)
                && a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 2)
        })
        .count();
    assert!(
        outside_changes > 100,
        "Emission did not light surrounding cave: {outside_changes}"
    );
    assert_eq!(
        render(
            Settings {
                emission: 0.0,
                ..cave.clone()
            },
            0.0
        ),
        render(
            Settings {
                emission: 0.0,
                spill: 0.0,
                ..cave.clone()
            },
            0.0
        )
    );
    assert_ne!(
        illuminated,
        render(
            Settings {
                bloom: 1.0,
                ..cave.clone()
            },
            0.0
        )
    );
    let reflection = Settings {
        background: 5,
        emission: 0.0,
        transmission: 0.0,
        ambient: 0.2,
        light_power: 0.0,
        bloom: 0.0,
        reflectivity: 2.0,
        probe: [0.5, 0.5],
        ..Default::default()
    };
    let reflected = render(reflection.clone(), 0.0);
    let matte = render(
        Settings {
            reflectivity: 0.0,
            ..reflection
        },
        0.0,
    );
    assert!(reflected.iter().zip(&matte).filter(|(a, b)| a != b).count() > 100);
    assert_eq!(&reflected[..256 * 20 * 4], &matte[..256 * 20 * 4]);
    let depth = Settings {
        background: 7,
        partner: 1,
        mode: 5,
        normals: 0.0,
        roughness: 0.1,
        source_depth: 0.9,
        trainers: false,
        reflection_focus: true,
        bloom: 0.0,
        ..Default::default()
    };
    let count_red = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 140 && p[0] > p[2].saturating_mul(2))
            .count()
    };
    let count_blue = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .filter(|p| p[2] > 140 && p[2] > p[0].saturating_mul(2))
            .count()
    };
    let near = render(depth.clone(), 0.0);
    assert!(
        count_red(&near) > 100,
        "The reflecting surface should see the red FRONT"
    );
    assert_eq!(
        count_blue(&near),
        0,
        "The hidden source's blue back must not become its reflection"
    );
    let far = render(
        Settings {
            source_depth: -0.9,
            ..depth.clone()
        },
        0.0,
    );
    assert_eq!(
        count_red(&far),
        0,
        "An object beyond the surface cannot reflect in its flat camera-facing side"
    );
    let battle = render(
        Settings {
            reflection_focus: false,
            ..depth
        },
        0.0,
    );
    assert!(
        count_blue(&battle) > 100,
        "The direct battle camera should see the source's blue BACK"
    );
    let empty = render(
        Settings {
            background: 7,
            asset: 2,
            partner: 2,
            trainers: false,
            reflection_focus: false,
            mode: 5,
            bloom: 0.0,
            ..Default::default()
        },
        0.0,
    );
    assert_eq!(
        count_blue(&empty) + count_red(&empty),
        0,
        "An empty capture must not retain the previous front/back images"
    );
    let rainy = Settings {
        background: 6,
        probe_enabled: false,
        ..Default::default()
    };
    let wet = render(rainy.clone(), 0.4);
    assert_ne!(
        wet,
        render(
            Settings {
                rain: 0.0,
                ..rainy.clone()
            },
            0.4
        )
    );
    assert_ne!(
        wet,
        render(
            Settings {
                wetness: 0.0,
                ..rainy.clone()
            },
            0.4
        )
    );
    assert_ne!(
        wet,
        render(
            Settings {
                wind: -0.8,
                ..rainy
            },
            0.4
        )
    );
    // A full battle shades two real rigs: far FRONT, near BACK, with shared XYZ captures.
    let face_to_face = Settings {
        battle: true,
        asset: 1,
        partner: 1,
        mode: 1,
        background: 7,
        bloom: 0.0,
        trainers: false,
        probe_enabled: false,
        ..Default::default()
    };
    let pair = render(face_to_face.clone(), 0.0);
    let centroid = |red: bool| {
        let points: Vec<_> = pair
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, p)| {
                if red {
                    p[0] > 140 && p[0] > p[2].saturating_mul(2)
                } else {
                    p[2] > 140 && p[2] > p[0].saturating_mul(2)
                }
            })
            .map(|(i, _)| [(i % 256) as f32, (i / 256) as f32])
            .collect();
        assert!(
            points.len() > 100,
            "Both directional views must be visible in battle"
        );
        [
            points.iter().map(|p| p[0]).sum::<f32>() / points.len() as f32,
            points.iter().map(|p| p[1]).sum::<f32>() / points.len() as f32,
        ]
    };
    let (front, back) = (centroid(true), centroid(false));
    assert!(
        front[0] > back[0] + 50.0 && front[1] < back[1] - 30.0,
        "Opponent front belongs at upper right; player back at lower left"
    );
    let reflected_pair = render(
        Settings {
            mode: 5,
            normals: 0.0,
            roughness: 0.1,
            ..face_to_face.clone()
        },
        0.0,
    );
    assert!(
        count_red(&reflected_pair) > 30,
        "Opponent must reflect the player's red FRONT"
    );
    assert_eq!(
        count_blue(&reflected_pair),
        0,
        "Reflection must not reuse the camera's blue BACK"
    );
    let dark_pair = Settings {
        asset: 0,
        partner: 0,
        mode: 0,
        background: 4,
        emission: 3.0,
        ambient: 0.07,
        light_power: 0.0,
        spill: 2.0,
        ..face_to_face
    };
    assert_ne!(
        render(dark_pair.clone(), 0.0),
        render(
            Settings {
                emission: 0.0,
                ..dark_pair
            },
            0.0
        )
    );
    let world = Settings {
        world: true,
        count: 32,
        world_size: 32.0,
        background: 6,
        ambient: 0.2,
        light_power: 0.1,
        lights: 16,
        reflection_steps: 16,
        reflectivity: 3.0,
        roughness: 0.4,
        rain: 0.0,
        bloom: 0.0,
        probe_enabled: false,
        ..Default::default()
    };
    let full = render(world.clone(), 0.4);
    let no_lights = render(
        Settings {
            lights: 0,
            ..world.clone()
        },
        0.4,
    );
    let no_reflections = render(
        Settings {
            reflection_steps: 0,
            ..world.clone()
        },
        0.4,
    );
    let differences =
        |a: &[u8], b: &[u8]| a.iter().zip(b).filter(|(a, b)| a.abs_diff(**b) > 2).count();
    assert!(
        differences(&full, &no_lights) > 300,
        "World local lights must illuminate the scene"
    );
    assert!(
        differences(&full, &no_reflections) > 100,
        "World depth reflections must contribute visible color"
    );
    assert_ne!(
        full,
        render(world.clone(), 0.9),
        "World rigs and walking must animate"
    );
    assert_eq!(
        render(
            Settings {
                emission: 0.0,
                ..world.clone()
            },
            0.4
        ),
        render(
            Settings {
                emission: 0.0,
                lights: 0,
                ..world.clone()
            },
            0.4
        ),
        "Switching off emitters must remove their local light too"
    );
    // Curated scenes must move the actual captured subject and preserve material controls.
    for showcase in 1..=3 {
        let composed = Settings {
            showcase,
            world: true,
            showcase_motion: 0.3,
            ambient: 0.3,
            light_power: 0.3,
            reflectivity: 2.0,
            bloom: 0.0,
            probe_enabled: false,
            ..Default::default()
        };
        let first = render(composed.clone(), 0.4);
        assert!(
            differences(
                &first,
                &render(
                    Settings {
                        showcase_motion: 0.7,
                        ..composed.clone()
                    },
                    0.4
                )
            ) > 100,
            "Showcase {showcase} must move a real object"
        );
        if showcase == 3 {
            assert!(
                differences(
                    &first,
                    &render(
                        Settings {
                            transmission: 0.0,
                            ..composed
                        },
                        0.4
                    )
                ) > 100,
                "Glass must reveal a visibly different source layer"
            );
        }
    }
    let mirror_only = Settings {
        world: true,
        showcase: 2,
        mode: 5,
        showcase_motion: 0.25,
        roughness: 0.2,
        probe_enabled: false,
        bloom: 0.0,
        ..Default::default()
    };
    let mirror_a = render(mirror_only.clone(), 0.4);
    let mirror_b = render(
        Settings {
            showcase_motion: 0.75,
            ..mirror_only
        },
        0.4,
    );
    let reflected_movement = mirror_a
        .chunks_exact(4)
        .zip(mirror_b.chunks_exact(4))
        .enumerate()
        .filter(|(i, (a, b))| {
            i % 256 > 166 && a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 2)
        })
        .count();
    assert!(
        reflected_movement > 10,
        "Moving the captured source must alter the distant mirror, got {reflected_movement}"
    );
    // Unlit source art must be independent of normal/light settings.
    assert_eq!(
        render(
            Settings {
                mode: 1,
                ..Default::default()
            },
            0.0
        ),
        render(
            Settings {
                mode: 1,
                light: [0.9, 0.9, 1.2],
                normals: 0.0,
                emission: 0.0,
                ..Default::default()
            },
            0.0
        )
    );

    // A true comparison reference must ignore ALL viewer material/display changes,
    // not merely the lighting terms in the sprite fragment shader.
    let reference = Settings {
        reference: true,
        battle: true,
        asset: 1,
        partner: 1,
        ..Default::default()
    };
    let reference_pixels = render(reference.clone(), 0.4);
    assert_eq!(
        reference_pixels,
        render(
            Settings {
                mode: 5,
                background: 6,
                rain: 2.0,
                wind: -1.0,
                wetness: 1.0,
                light: [0.8, 0.9, 0.1],
                light_power: 4.0,
                ambient: 0.0,
                emission: 4.0,
                opacity: 0.05,
                transmission: 2.0,
                reflectivity: 4.0,
                normals: 0.0,
                bloom: 2.0,
                exposure: 5.0,
                probe_front: true,
                probe: [0.3, 0.5],
                ..reference.clone()
            },
            0.4,
        ),
        "Original reference must ignore material, weather, probe and display controls"
    );
    for rgb in [[255u8, 12, 6], [6u8, 12, 255]] {
        let exact = reference_pixels
            .chunks_exact(4)
            .filter(|p| p[..3].iter().zip(rgb).all(|(&a, b)| a.abs_diff(b) <= 1))
            .count();
        assert!(
            exact > 100,
            "Reference must retain both front/back source RGB values, got {exact}"
        );
    }
    assert_ne!(
        reference_pixels,
        render(reference, 1.2),
        "Reference keeps the native animation clock instead of freezing the original"
    );
}
