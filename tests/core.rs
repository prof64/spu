use sprite_lab::{
    format::{Ease, Key, Package},
    runtime::{Affine, Scene, Settings, sample},
};

fn demo() -> Package {
    Package::read(include_bytes!("../web/examples/copperbot.sprite")).unwrap()
}

#[test]
fn unknown_metadata_and_resources_survive_edit_and_save() {
    let mut p = demo();
    p.document.extra.insert(
        "futureAuthoring".into(),
        serde_json::json!({"anchor":"left-hand","strength":0.4}),
    );
    p.document.views[0].parts[0]
        .extra
        .insert("customPhysics".into(), serde_json::json!({"magnetism":0.8}));
    p.entries
        .insert("extensions/notes.bin".into(), vec![0, 8, 10, 255]);
    p.document.materials[0].roughness = 0.2;
    let loaded = Package::read(&p.write().unwrap()).unwrap();
    assert_eq!(p.document.extra, loaded.document.extra);
    assert_eq!(
        p.document.views[0].parts[0].extra,
        loaded.document.views[0].parts[0].extra
    );
    assert_eq!(
        p.entries["extensions/notes.bin"],
        loaded.entries["extensions/notes.bin"]
    );
    assert_eq!(loaded.document.materials[0].roughness, 0.2);
}

#[test]
fn unsupported_required_features_and_versions_are_rejected() {
    let mut p = demo();
    p.document.required.push("future-volumetric-shader".into());
    assert!(
        Package::read(&p.write().unwrap())
            .unwrap_err_string()
            .contains("Unsupported required capability")
    );
    p.document.required = vec![];
    p.document.version = 2;
    assert!(Package::read(&p.write().unwrap()).is_err());
}

trait ErrorString {
    fn unwrap_err_string(self) -> String;
}
impl<T> ErrorString for anyhow::Result<T> {
    fn unwrap_err_string(self) -> String {
        match self {
            Ok(_) => panic!("expected error"),
            Err(e) => e.to_string(),
        }
    }
}

#[test]
fn invalid_references_cycles_and_unsorted_keys_fail_validation() {
    let mut p = demo();
    p.document.views[0].parts[0].parent = Some("orb".into());
    assert!(p.document.validate().is_err());
    let mut p = demo();
    p.document.views[0].images[0].material = "missing".into();
    assert!(p.document.validate().is_err());
    let mut p = demo();
    let t = &mut p.document.views[0].animations[0].tracks[0];
    t.x = vec![
        Key {
            tick: 10.0,
            value: 0.0,
            ease: Ease::Linear,
        },
        Key {
            tick: 0.0,
            value: 1.0,
            ease: Ease::Linear,
        },
    ];
    assert!(p.document.validate().is_err());
}

#[test]
fn missing_or_misaligned_texture_is_an_error() {
    let mut p = demo();
    p.entries.remove("body-color.png");
    assert!(Scene::load(vec![p]).is_err());
    let mut p = demo();
    p.entries.insert("body-normal.png".into(), vec![1, 2, 3]);
    assert!(Scene::load(vec![p]).is_err());
}

#[test]
fn interpolation_holds_steps_and_honors_easing() {
    let keys = vec![
        Key {
            tick: 0.0,
            value: 0.0,
            ease: Ease::EaseIn,
        },
        Key {
            tick: 10.0,
            value: 20.0,
            ease: Ease::Step,
        },
    ];
    assert_eq!(sample(&keys, 5.0, 9.0), 5.0);
    assert_eq!(sample(&keys, 50.0, 9.0), 20.0);
    let mut stepped = keys;
    stepped[0].ease = Ease::Step;
    assert_eq!(sample(&stepped, 9.0, 9.0), 0.0);
    assert_eq!(sample(&[], 5.0, 7.0), 7.0);
}

#[test]
fn hierarchy_preserves_mirroring_and_nonuniform_scale() {
    let parent = Affine::trs(10.0, 20.0, 90.0, -2.0, 3.0);
    let child = Affine::trs(4.0, 5.0, 30.0, 1.0, 2.0);
    let composed = parent.compose(child);
    let p = child.point(7.0, -3.0);
    let expected = parent.point(p[0], p[1]);
    let actual = composed.point(7.0, -3.0);
    assert!((expected[0] - actual[0]).abs() < 0.0001);
    assert!((expected[1] - actual[1]).abs() < 0.0001);
}

#[test]
fn independent_clocks_animate_and_texture_storage_is_shared() {
    let mut scene = Scene::load(vec![demo()]).unwrap();
    let bytes = scene.texture_bytes();
    let mut a = Vec::new();
    scene.prepare(&Settings::default(), 0.0, 640, 480, &mut a);
    let original = a.clone();
    scene.prepare(&Settings::default(), 1.0, 640, 480, &mut a);
    assert_ne!(
        bytemuck::cast_slice::<_, u8>(&a),
        bytemuck::cast_slice::<_, u8>(&original)
    );
    scene.prepare(
        &Settings {
            count: 1000,
            ..Default::default()
        },
        1.0,
        640,
        480,
        &mut a,
    );
    assert_eq!(a.len(), 3000);
    assert_eq!(scene.texture_bytes(), bytes);
    assert!(
        a.iter()
            .all(|i| i.basis.iter().chain(&i.placement).all(|f| f.is_finite()))
    );
}

#[test]
fn named_battle_animations_resolve_per_view_and_in_reflection_captures() {
    let mut package = demo();
    let mut front = package.document.views[0].clone();
    front.id = "front".into();
    front.name = "Front".into();
    let mut idle = front.animations[0].clone();
    idle.id = "idle".into();
    idle.name = "Idle".into();
    let mut glow = idle.clone();
    glow.id = "glow".into();
    glow.name = "Glow pulse".into();
    // Three visible parts identify Glow. Idle hides them so index/name mismatches
    // are observable without relying on a particular animation's sampled values.
    for track in &mut idle.tracks {
        track.visible = vec![Key {
            tick: 0.0,
            value: false,
            ease: Ease::Step,
        }];
    }
    front.animations = vec![idle.clone(), glow.clone()];
    let mut back = front.clone();
    back.id = "back".into();
    back.name = "Back".into();
    back.animations = vec![glow, idle];
    package.document.views = vec![front, back];
    let mut scene = Scene::load(vec![package.clone()]).unwrap();
    let mut instances = Vec::new();
    let named = Settings {
        battle: true,
        animation: 1,
        animation_name: Some("gLoW pUlSe".into()),
        ..Default::default()
    };
    scene.prepare(&named, 0.0, 640, 480, &mut instances);
    assert_eq!(instances.len(), 6, "Both facing views select Glow by name");
    assert_eq!(
        scene.reflection_instances.len(),
        12,
        "All four captured views select Glow by name"
    );

    let numeric = Settings {
        animation_name: None,
        ..named.clone()
    };
    scene.prepare(&numeric, 0.0, 640, 480, &mut instances);
    assert_eq!(instances.len(), 3, "Legacy numeric selection is unchanged");

    scene.prepare(
        &Settings {
            animation_name: Some("Missing".into()),
            ..named.clone()
        },
        0.0,
        640,
        480,
        &mut instances,
    );
    assert!(
        instances.is_empty(),
        "Missing named clip selects each view's Idle, not its numeric index"
    );
    assert!(scene.reflection_instances.is_empty());

    // Nonbattle reflection partner captures start from default Settings. They
    // still need the caller's named clip, not only its numeric animation index.
    scene.prepare(
        &Settings {
            battle: false,
            background: 7,
            ..named.clone()
        },
        0.0,
        640,
        480,
        &mut instances,
    );
    assert_eq!(instances.len(), 3);
    assert_eq!(scene.reflection_instances.len(), 12);

    for view in &mut package.document.views {
        for clip in &mut view.animations {
            if clip.name == "Idle" {
                clip.name = "Rest".into();
            }
        }
    }
    let mut no_idle = Scene::load(vec![package]).unwrap();
    no_idle.prepare(
        &Settings {
            animation_name: Some("Missing".into()),
            ..named
        },
        0.0,
        640,
        480,
        &mut instances,
    );
    assert_eq!(
        instances.len(),
        3,
        "Without the requested clip or Idle, each view uses clip zero"
    );
}
