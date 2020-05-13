use crate::material::Material;
use glam::{vec3, Vec3};
use std::sync::Arc;

/// The ray data type
#[derive(Clone, Copy, Debug, Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inv_direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let inv_direction = vec3(
            1.0 / direction.x(),
            1.0 / direction.y(),
            1.0 / direction.z(),
        );

        Self {
            origin,
            direction,
            inv_direction,
        }
    }

    pub fn point_at_parameter(&self, t: f32) -> Vec3 {
        self.origin + t * self.direction
    }
}

/// Contains data to be used in the generation of a new ray as a result of an intersection.
#[derive(Clone, Debug)]
pub struct Hit {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: Option<Arc<dyn Material>>,
}
