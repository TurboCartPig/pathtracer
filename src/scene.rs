use crate::{
    bvh::BVH, camera::Camera, color, material::Material, primitives::Instance, DefaultRng,
    SettingsConfig, COLOR_CHANNELS,
};
use glam::{vec3, Vec3};
use image::{save_buffer, ColorType};
use rand::prelude::*;
use rayon::prelude::*;
use std::{collections::HashMap, sync::Arc};

/// Traced image
pub struct Image {
    pub dimensions: (u32, u32),
    pub buffer: Vec<u8>,
}

impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            dimensions: (width, height),
            buffer: vec![0u8; (width * height * COLOR_CHANNELS) as usize],
        }
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        save_buffer(
            path,
            &self.buffer,
            self.dimensions.0,
            self.dimensions.1,
            ColorType::Rgb8,
        )?;

        Ok(())
    }
}

/// A material cache that stores all the materials in the scene
pub struct Materials {
    inner: HashMap<String, Arc<dyn Material + Send + Sync>>,
}

impl Materials {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

/// A Scene containing tracable objects and their materials.
pub struct Scene {
    settings: SettingsConfig,
    camera: Camera,
    bvh: BVH,
    materials: Materials,
}

impl Scene {
    pub fn new(settings: SettingsConfig, primitives: Vec<Instance>) -> Self {
        let camera = Camera::new(
            vec3(13.0, 2.0, 3.0),
            vec3(4.0, 1.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            20.0,
            settings.width() as f32 / settings.height() as f32,
            0.1,
        );
        let bvh = BVH::new(primitives);
        let materials = Materials::new();

        Scene {
            settings,
            camera,
            bvh,
            materials,
        }
    }

    pub fn trace(&self) -> Image {
        let global_ray_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let start = std::time::Instant::now();
        let mut image = Image::new(self.settings.width(), self.settings.height());

        image
            .buffer
            .par_chunks_mut((self.settings.width() * COLOR_CHANNELS) as usize)
            .rev()
            .enumerate()
            .for_each(|(y, row)| {
                let mut rng = DefaultRng::from_entropy();
                row.chunks_mut(COLOR_CHANNELS as usize)
                    .enumerate()
                    .for_each(|(i, pixel)| {
                        let mut out = Vec3::zero();
                        let mut ray_count = 0;

                        // Antialiasing via multisampling
                        for _ in 0..self.settings.samples {
                            let u = (rng.gen::<f32>() + i as f32) / self.settings.width() as f32;
                            let v = (rng.gen::<f32>() + y as f32) / self.settings.height() as f32;

                            let ray = self.camera.ray(u, v, &mut rng);

                            let mut instance_ray_count = 1;
                            out += color(
                                ray,
                                &mut instance_ray_count,
                                &self.bvh,
                                &mut rng,
                                self.settings.max_bounces,
                            );
                            ray_count += instance_ray_count;
                        }

                        out /= self.settings.samples as f32;

                        // Gamma correct
                        out = Vec3::new(
                            out.x().powf(1.0 / self.settings.gamma),
                            out.y().powf(1.0 / self.settings.gamma),
                            out.z().powf(1.0 / self.settings.gamma),
                        );

                        // Convert from [0, 1] to [0, 256]
                        let r = (255.99 * out.x()) as u8;
                        let g = (255.99 * out.y()) as u8;
                        let b = (255.99 * out.z()) as u8;

                        // Write output color to buffer
                        pixel[0] = r;
                        pixel[1] = g;
                        pixel[2] = b;

                        global_ray_count.fetch_add(ray_count, std::sync::atomic::Ordering::Relaxed);
                    })
            });

        let finished = std::time::Instant::now();
        let duration = finished.duration_since(start);
        let global_ray_count =
            f64::from(global_ray_count.load(std::sync::atomic::Ordering::Relaxed)) / 1_000_000.0;
        let rays_per_second = global_ray_count
            / (duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) / 1_000_000_000.0);
        println!(
            "Time elapsed: {:.2?}\nTotal Rays: {:.2}M\nRays per second: {:.2}M",
            duration, global_ray_count, rays_per_second
        );

        image
    }
}
