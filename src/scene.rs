use crate::{
    bvh::BVH, camera::Camera, color, material::Material, primitives::Instance, DefaultRng,
    SettingsConfig,
};
use glam::{vec3, Vec3};
use image::{save_buffer, ColorType};
use itertools::iproduct;
use rand::prelude::*;
use rayon::prelude::*;
use smallvec::*;
use std::{collections::HashMap, sync::Arc};

/// Traced image
pub struct Image {
    pub dimensions: (u32, u32),
    pub buffer: Vec<u8>,
}

impl Image {
    // pub fn new(width: u32, height: u32) -> Self {
    //     Self {
    //         dimensions: (width, height),
    //         buffer: vec![0u8; (width * height * 3) as usize],
    //     }
    // }

    pub fn from(buffer: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            dimensions: (width, height),
            buffer,
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
        let start = std::time::Instant::now();

        // Cartesian product
        let pixels: Vec<_> =
            iproduct!(0..self.settings.width(), 0..self.settings.height()).collect();

        // Main pathtracing
        let (global_ray_count, mut pixels): (Vec<u32>, Vec<_>) = pixels
            .into_par_iter()
            .map_with(DefaultRng::from_entropy(), |mut rng, (x, y)| {
                let mut pixel = Vec3::zero();
                let mut ray_count = 0;

                // Antialiasing via multisampling
                for _ in 0..self.settings.samples {
                    let u = (rng.gen::<f32>() + x as f32) / self.settings.width() as f32;
                    let v = (rng.gen::<f32>() + y as f32) / self.settings.height() as f32;

                    let ray = self.camera.ray(u, v, &mut rng);

                    let mut instance_ray_count = 0;
                    pixel += color(
                        ray,
                        &mut instance_ray_count,
                        &self.bvh,
                        &mut rng,
                        self.settings.max_bounces,
                    );
                    ray_count += instance_ray_count;
                }

                // Normalize over samples
                pixel /= self.settings.samples as f32;

                // Gamma correct
                pixel = Vec3::new(
                    pixel.x().powf(1.0 / self.settings.gamma),
                    pixel.y().powf(1.0 / self.settings.gamma),
                    pixel.z().powf(1.0 / self.settings.gamma),
                );

                // Convert from [0, 1] to [0, 255]
                let pixel = 254.99 * pixel;

                (ray_count, ((x, y), pixel))
            })
            .unzip();

        // Add up all the ray counts
        let global_ray_count: u32 = global_ray_count.into_iter().sum();

        // Sort the pixels
        pixels.sort_unstable_by(|((x1, y1), _), ((x2, y2), _)| {
            let a = (self.settings.height() - y1) * self.settings.width() + x1;
            let b = (self.settings.height() - y2) * self.settings.width() + x2;

            Ord::cmp(&a, &b)
        });

        // Reinterperate the pixels into expected image format
        let pixels: Vec<_> = pixels
            .into_iter()
            .flat_map(|(_, pixel)| {
                let p: SmallVec<[u8; 3]> =
                    smallvec![pixel.x() as u8, pixel.y() as u8, pixel.z() as u8];
                p
            })
            .collect();

        let image = Image::from(pixels, self.settings.width(), self.settings.height());

        let finished = std::time::Instant::now();
        let duration = finished.duration_since(start);

        let global_ray_count = global_ray_count / 1_000_000;
        let rays_per_second = global_ray_count as f64
            / (duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) / 1_000_000_000.0);
        println!(
            "Time elapsed: {:.2?}\nTotal Rays: {:.2}M\nRays per second: {:.2}M",
            duration, global_ray_count, rays_per_second
        );

        let min_estimated_total_rays =
            self.settings.width() * self.settings.height() * self.settings.samples;
        let max_estimated_total_rays = min_estimated_total_rays * self.settings.max_bounces;
        println!(
            "Minimum estimated total rays: {:.2}M\nMaximum estimated total rays: {:.2}M",
            min_estimated_total_rays / 1_000_000,
            max_estimated_total_rays / 1_000_000
        );

        image
    }
}
