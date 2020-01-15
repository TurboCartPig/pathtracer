use crate::{DefaultRng, Hit, Ray};
use glam::{vec3, Vec3};
use rand::prelude::*;
use rand_distr::{Distribution, UnitSphere};

// Samples a random point in a unit sphere from the thread rng
pub fn sample_unit_sphere(rng: &mut DefaultRng) -> Vec3 {
    Vec3::from(UnitSphere.sample(rng))
}

// Reflect vector v around normal n
pub fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(n) * n
}

// Refract vector v around normal n and return only if successfull
pub fn refract(v: Vec3, n: Vec3, ni_over_nt: f32) -> Option<Vec3> {
    let uv = v.normalize();
    let dt = uv.dot(n);
    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);

    if discriminant > 0.0 {
        let refracted = ni_over_nt * (uv - n * dt) - n * f32::sqrt(discriminant);
        Some(refracted)
    } else {
        None
    }
}

// An approximation for reflectivity
pub fn schlick(cosine: f32, reflection_index: f32) -> f32 {
    let r_0 = (1.0 - reflection_index) / (1.0 + reflection_index);
    let r_0 = r_0 * r_0;

    r_0 + (1.0 - r_0) * f32::powf(1.0 - cosine, 5.0)
}

pub struct ScatterResult {
    pub scattered: Ray,
    pub attenuation: Vec3,
}

use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub enum Materials {
    Lambertian,
    Metal,
    Dielectric,
}

#[enum_dispatch]
pub trait Material: std::fmt::Debug {
    fn scatter(&self, ray: Ray, hit: Hit, rng: &mut DefaultRng) -> Option<ScatterResult>;
}

#[derive(Debug)]
pub struct Lambertian {
    pub albedo: Vec3,
}

impl Lambertian {
    pub fn new(albedo: Vec3) -> Self {
        Self { albedo }
    }
}

impl Material for Lambertian {
    fn scatter(&self, _ray: Ray, hit: Hit, rng: &mut DefaultRng) -> Option<ScatterResult> {
        let target = hit.point + hit.normal + sample_unit_sphere(rng);

        Some(ScatterResult {
            scattered: Ray::new(hit.point, target - hit.point),
            attenuation: self.albedo,
        })
    }
}

#[derive(Debug)]
pub struct Metal {
    pub albedo: Vec3,
    pub fuzz: f32,
}

impl Metal {
    pub fn new(albedo: Vec3, fuzz: f32) -> Self {
        Self { albedo, fuzz }
    }
}

impl Material for Metal {
    fn scatter(&self, ray: Ray, hit: Hit, rng: &mut DefaultRng) -> Option<ScatterResult> {
        let reflected = reflect(ray.direction.normalize(), hit.normal);
        let scattered = Ray::new(hit.point, reflected + self.fuzz * sample_unit_sphere(rng));

        if scattered.direction.dot(hit.normal) > 0.0 {
            Some(ScatterResult {
                scattered,
                attenuation: self.albedo,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Dielectric {
    reflection_index: f32,
}

impl Dielectric {
    pub fn new(reflection_index: f32) -> Self {
        Self { reflection_index }
    }
}

impl Material for Dielectric {
    fn scatter(&self, ray: Ray, hit: Hit, rng: &mut DefaultRng) -> Option<ScatterResult> {
        let outward_normal;
        let ni_over_nt;
        let cosine;

        if ray.direction.dot(hit.normal) > 0.0 {
            outward_normal = -hit.normal;
            ni_over_nt = self.reflection_index;
            cosine = self.reflection_index * ray.direction.dot(hit.normal) / ray.direction.length();
        } else {
            outward_normal = hit.normal;
            ni_over_nt = 1.0 / self.reflection_index;
            cosine = -ray.direction.dot(hit.normal) / ray.direction.length();
        }

        let reflected = reflect(ray.direction, hit.normal);
        let refracted = refract(ray.direction, outward_normal, ni_over_nt);

        // Probability decides if we reflect or refract
        let reflect_prob = if refracted.is_some() {
            schlick(cosine, self.reflection_index)
        } else {
            1.0
        };

        // Reflect or refract based on probability
        let scattered = if rng.gen_bool(reflect_prob.into()) {
            Ray::new(hit.point, reflected)
        } else {
            Ray::new(hit.point, refracted.unwrap())
        };

        let attenuation = vec3(0.9, 0.9, 0.9);

        Some(ScatterResult {
            scattered,
            attenuation,
        })
    }
}
