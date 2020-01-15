//! This module is full of primitives that all impl Intersection

mod aabb;
mod sphere;
mod instance;

pub use aabb::*;
pub use sphere::*;
pub use instance::*;


use enum_dispatch::enum_dispatch;

#[enum_dispatch(Intersect)]
#[derive(Clone)]
pub enum Primitives {
    AABB,
    Sphere,
}
