mod bvh;
mod camera;
mod material;
mod primitives;
mod ray;
mod scene;
// mod textures;

use crate::{bvh::*, material::*, primitives::*, ray::*, scene::*};
use glam::{vec3, Vec3};
use rand::prelude::*;
use serde::Deserialize;
use std::io::Read;
use std::sync::Arc;

/// Default random number generator to be used
type DefaultRng = rand_xoshiro::Xoshiro256PlusPlus;

/// Specifies settings used in the pathtracing
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct SettingsConfig {
    /// Resolution of the output image (width, height)
    resolution: [u32; 2],
    /// Number of samples per pixel
    samples: u32,
    /// Max bounces of a single primary ray
    max_bounces: u32,
    /// Gamma
    gamma: f32,
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            resolution: [1280, 720],
            samples: 12,
            max_bounces: 8,
            gamma: 2.2,
        }
    }
}

impl SettingsConfig {
    pub fn width(&self) -> u32 {
        self.resolution[0]
    }

    pub fn height(&self) -> u32 {
        self.resolution[1]
    }
}

/// Computes the color of a pixel/sample based on a ray
/// Returns color and raycount
fn color(ray: Ray, bounces: &mut u32, bvh: &BVH, rng: &mut DefaultRng, max_bounces: u32) -> Vec3 {
    // Max bounces
    if *bounces >= max_bounces {
        Vec3::zero()
    }
    // If the ray trace hits something
    else if let Some(hit) = bvh.intersection(ray, 0.0001, 10_000_000.0) {
        // The material of the object we hit decides how the ray scatters
        hit.material
            .clone()
            .and_then(|material| material.scatter(ray, hit, rng))
            .map(|scatter| {
                *bounces += 1;
                scatter.attenuation * color(scatter.scattered, bounces, bvh, rng, max_bounces)
            })
            .unwrap_or_else(Vec3::zero)
    }
    // Else draw the background/skybox
    else {
        let dir = ray.direction.normalize();
        let t = 0.5 * (dir.y() + 1.0);
        (1.0 - t) * vec3(1.0, 1.0, 1.0) + t * vec3(0.5, 0.7, 1.0)
    }
}

/// Generate a semi random scene
// TODO: Move to scene
fn random() -> Vec<Instance> {
    let mut rng = rand::thread_rng();
    let mut instances = Vec::new();

    // let transform = Transform::default();
    // let transform = Transform {
    //     rotation: glam::Quat::from_rotation_x(3.0),
    //     ..Default::default()
    // };
    let transform = Default::default();

    // The big sphere
    let material = Arc::new(Lambertian::new(vec3(0.5, 0.5, 0.5)));
    let primitive = Arc::new(Sphere::new(vec3(0.0, -1000.0, 0.0), 1000.0));
    instances.push(Instance::reciver(primitive, material, transform));

    let primitive = Arc::new(Sphere::new(Vec3::zero(), 0.2));
    for a in -12..12 {
        for b in -12..12 {
            let material = rng.gen::<f32>();
            let center = vec3(
                a as f32 + 0.9 * rng.gen::<f32>(),
                0.2,
                b as f32 + 0.9 * rng.gen::<f32>(),
            );

            if (center - vec3(4.0, 0.2, 0.0)).length() > 0.9 {
                let r = vec3(
                    rng.gen::<f32>() * rng.gen::<f32>(),
                    rng.gen::<f32>() * rng.gen::<f32>(),
                    rng.gen::<f32>() * rng.gen::<f32>(),
                );

                // Lambertian
                let material: Arc<dyn Material + Send + Sync> = if material < 0.5 {
                    Arc::new(Lambertian::new(r))
                // Metal
                } else if material < 0.75 {
                    Arc::new(Metal::new(r, rng.gen::<f32>()))
                // Dielectric
                } else {
                    Arc::new(Dielectric::new(1.5))
                };
                let transform = Transform {
                    translation: center,
                    ..Default::default()
                };
                instances.push(Instance::reciver(primitive.clone(), material, transform));
            }
        }
    }

    let material = Arc::new(Lambertian::new(vec3(0.6, 0.2, 0.9)));
    let primitive = Arc::new(Sphere::new(vec3(-4.0, 1.0, 0.0), 1.0));
    instances.push(Instance::reciver(primitive, material, transform));

    let material = Arc::new(Dielectric::new(1.5));
    let primitive = Arc::new(Sphere::new(vec3(0.0, 1.0, 0.0), 1.0));
    instances.push(Instance::reciver(primitive, material, transform));

    let material = Arc::new(Metal::new(vec3(0.7, 0.6, 0.5), 0.0));
    let primitive = Arc::new(Sphere::new(vec3(4.0, 1.0, 0.0), 1.0));
    instances.push(Instance::reciver(primitive, material, transform));

    instances
}

/// Load pathtracer settings from a settings file
fn load_settings() -> anyhow::Result<SettingsConfig> {
    let mut settings = std::fs::File::open("settings.toml")?;
    let mut buffer = String::new();
    settings.read_to_string(&mut buffer)?;
    let settings: SettingsConfig = toml::from_str(&buffer)?;

    Ok(settings)
}

fn main() {
    // Load in settings
    let settings: SettingsConfig = load_settings().unwrap_or_default();

    let scene = Scene::new(settings, random());
    let image = scene.trace();
    image
        .save("output.png")
        .expect("Failed to save output image");
}
