use crate::{material::Material, primitives::AABB, Hit, Intersect, Ray};
use glam::{Mat4, Vec3};
use std::sync::Arc;
use super::Primitives;

#[derive(Clone, Copy, Default)]
pub struct Transform {
    pub translation: Vec3,
    // pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        // FIXME: This panics
        // Mat4::from_scale_rotation_translation(
        //     self.scale,
        //     self.rotation,
        //     self.translation,
        // )

        Mat4::from_translation(self.translation)
    }

    // FIXME: This does not work
    pub fn inv_mul_vec(&self, v: Vec3) -> Vec3 {
        let inv = self.matrix().inverse();
        inv.transform_vector3(v)
    }

    // FIXME: This does not work
    pub fn inv_mul_ray(&self, ray: Ray) -> Ray {
        let origin = self.inv_mul_vec(ray.origin);
        let direction = self.inv_mul_vec(ray.direction).normalize();

        Ray::new(origin, direction)
    }
}

#[derive(Clone)]
pub enum Instance {
    Reciver {
        primitive: Arc<Primitives>,
        material: Arc<dyn Material + Send + Sync>,
        transform: Transform,
    },
    // Emitter {}
}

impl Instance {
    pub fn reciver(
        primitive: Arc<Primitives>,
        material: Arc<dyn Material + Send + Sync>,
        transform: Transform,
    ) -> Self {
        Instance::Reciver {
            primitive,
            material,
            transform,
        }
    }
}

impl Intersect for Instance {
    fn intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        match self {
            Instance::Reciver {
                primitive,
                material,
                transform,
            } => {
                // FIXME: This does not work
                // This is supposed to transform the ray to simulate that the intersection is
                // happening somwhere else
                let ray = Ray::new(ray.origin - transform.translation, ray.direction);
                // let mat = transform.matrix();
                primitive.intersection(ray, t_min, t_max).map(|mut hit| {
                    hit.material = Some(material.clone());
                    // hit.point = mat.transform_vector3(hit.point);
                    // hit.normal = mat.transform_vector3(hit.normal).normalize();
                    hit
                })
            }
        }
    }

    fn has_intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> bool {
        match self {
            Instance::Reciver { primitive, .. } => primitive.has_intersection(ray, t_min, t_max),
        }
    }

    fn bounds(&self) -> Option<AABB> {
        match self {
            Instance::Reciver { primitive, .. } => primitive.bounds(),
        }
    }
}
