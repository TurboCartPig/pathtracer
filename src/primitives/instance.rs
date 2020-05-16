use crate::{material::Material, primitives::AABB, Hit, Intersect, Ray};
use glam::{Quat, Vec3};
use std::sync::Arc;

#[derive(Clone, Copy, Default)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Clone)]
pub enum Instance {
    Reciver {
        primitive: Arc<dyn Intersect>,
        material: Arc<dyn Material>,
        transform: Transform,
    },
    // Emitter {},
}

impl Instance {
    pub fn reciver(
        primitive: Arc<dyn Intersect>,
        material: Arc<dyn Material>,
        transform: Transform,
    ) -> Self {
        Instance::Reciver {
            primitive,
            material,
            transform,
        }
    }
}

// FIXME: Only does translation now
impl Intersect for Instance {
    fn intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        match self {
            Instance::Reciver {
                primitive,
                material,
                transform,
            } => {
                let ray = Ray::new(
                    ray.origin - transform.translation,
                    transform.rotation * ray.direction,
                );
                primitive.intersection(ray, t_min, t_max).map(|mut hit| {
                    hit.material = Some(material.clone());
                    hit.point += transform.translation;
                    // hit.point = transform.rotation * hit.point;
                    hit
                })
            }
        }
    }

    fn has_intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> bool {
        match self {
            Instance::Reciver {
                primitive,
                transform,
                ..
            } => {
                let ray = Ray::new(
                    ray.origin - transform.translation,
                    // transform.rotation * ray.direction,
                    ray.direction,
                );
                primitive.has_intersection(ray, t_min, t_max)
            }
        }
    }

    fn bounds(&self) -> Option<AABB> {
        match self {
            Instance::Reciver {
                primitive,
                transform,
                ..
            } => primitive.bounds().map(|mut b| {
                b.min += transform.translation;
                b.max += transform.translation;

                b
            }),
        }
    }
}
