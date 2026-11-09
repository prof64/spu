use crate::{
    format::Package,
    render::{Renderer, Uniforms},
    runtime::{Instance, Scene, Settings},
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

fn error(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
fn now() -> f64 {
    web_sys::window().unwrap().performance().unwrap().now()
}

#[wasm_bindgen]
pub struct Lab {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    scene: Scene,
    settings: Settings,
    instances: Vec<Instance>,
}
#[wasm_bindgen]
impl Lab {
    pub async fn create(
        canvas: HtmlCanvasElement,
        first: Vec<u8>,
        second: Vec<u8>,
    ) -> Result<Lab, JsValue> {
        console_error_panic_hook::set_once();
        let mut packages = vec![Package::read(&first).map_err(error)?];
        if !second.is_empty() {
            packages.push(Package::read(&second).map_err(error)?);
        }
        let scene = Scene::load(packages).map_err(error)?;
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(error)?;
        let (adapter, device, queue) = Renderer::request(&instance, Some(&surface))
            .await
            .map_err(error)?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .ok_or_else(|| error("No display format available"))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: canvas.width().max(1),
            height: canvas.height().max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let renderer = Renderer::new(device, queue, adapter.get_info(), format, &scene);
        if let Some(validation) = renderer.device.pop_error_scope().await {
            return Err(error(format!(
                "Could not initialize the GPU renderer: {validation}"
            )));
        }
        Ok(Self {
            surface,
            config,
            renderer,
            scene,
            settings: Settings::default(),
            instances: Vec::with_capacity(256),
        })
    }
    pub fn metadata(&self) -> String {
        let mut value = self.scene.metadata();
        value["adapter"] = serde_json::json!({"name":self.renderer.adapter.name,"backend":format!("{:?}",self.renderer.adapter.backend)});
        value.to_string()
    }
    pub fn configure(&mut self, json: &str) -> Result<(), JsValue> {
        let mut settings: Settings = serde_json::from_str(json).map_err(error)?;
        settings.count = settings.count.clamp(1, 10000);
        settings.zoom = settings.zoom.clamp(0.1, 3.0);
        self.settings = settings;
        Ok(())
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.clamp(1, 4096);
        self.config.height = height.clamp(1, 4096);
        self.surface.configure(&self.renderer.device, &self.config);
    }
    /// CPU preparation/submission timings only; completion is measured by native benchmarks.
    pub fn frame(&mut self, seconds: f32) -> Result<Vec<f64>, JsValue> {
        let start = now();
        self.scene.prepare(
            &self.settings,
            seconds,
            self.config.width,
            self.config.height,
            &mut self.instances,
        );
        let prepared = now();
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.renderer.device, &self.config);
                self.surface.get_current_texture().map_err(error)?
            }
            Err(e) => return Err(error(e)),
        };
        if self.settings.world {
            self.renderer.upload_world_lights(
                &self.scene.emitters,
                self.config.width,
                self.config.height,
            );
        }
        self.renderer
            .upload_reflections(&self.scene.reflection_instances);
        self.renderer.upload(
            &self.instances,
            &Uniforms::new(
                &self.settings,
                seconds,
                self.config.width,
                self.config.height,
            )
            .with_emitters(&self.scene.emitters),
        );
        self.renderer
            .draw(&frame.texture.create_view(&Default::default()));
        frame.present();
        Ok(vec![
            prepared - start,
            now() - prepared,
            self.instances.len() as f64,
            self.scene.emitters.len() as f64,
            self.scene.terrain_tiles as f64,
        ])
    }
    pub fn load(&mut self, bytes: Vec<u8>) -> Result<usize, JsValue> {
        let package = Package::read(&bytes).map_err(error)?;
        let mut packages: Vec<_> = self
            .scene
            .assets
            .iter()
            .take(31)
            .map(|a| a.package.clone())
            .collect();
        packages.push(package);
        let index = packages.len() - 1;
        let scene = Scene::load(packages).map_err(error)?;
        let renderer = Renderer::new(
            self.renderer.device.clone(),
            self.renderer.queue.clone(),
            self.renderer.adapter.clone(),
            self.config.format,
            &scene,
        );
        self.scene = scene;
        self.renderer = renderer;
        Ok(index)
    }
    pub fn export(&self, index: usize) -> Result<Vec<u8>, JsValue> {
        let mut p = self
            .scene
            .assets
            .get(index)
            .ok_or_else(|| error("Unknown asset"))?
            .package
            .clone();
        for m in &mut p.document.materials {
            m.roughness = (m.roughness * self.settings.roughness).clamp(0.04, 1.0);
            m.transmission = (m.transmission * self.settings.transmission).clamp(0.0, 1.0);
            m.emission *= self.settings.emission;
            m.opacity *= self.settings.opacity;
            m.normal_strength *= self.settings.normals;
            m.reflectivity *= self.settings.reflectivity;
        }
        p.document.extra.insert(
            "previewEnvironment".into(),
            serde_json::to_value(&self.settings).map_err(error)?,
        );
        p.document.validate().map_err(error)?;
        p.write().map_err(error)
    }
}
