//! This module is full of primitives that all impl Intersection

mod aabb;
mod instance;
mod sphere;

pub use aabb::*;
pub use instance::*;
pub use sphere::*;

use crate::ray::{Hit, Ray};

/// Computes whether a ray intersects a primitive
pub trait Intersect: Send + Sync {
    /// Computes the intersection between the ray and the primitive
    fn intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<Hit>;

    /// Computes whether there is an intersection between the ray and the primitive.
    /// Could be cheaper than "intersection".
    fn has_intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> bool;

    /// Generate a bounds for the primitive
    fn bounds(&self) -> Option<AABB>;
}
