use crate::{bvh::Axis, Hit, Intersect, Ray};
use glam::{vec3, Vec3};

#[derive(Clone, Copy, Debug, Default)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    // Create a union AABB of two AABBs that surrounds both of them
    pub fn union(self, other: AABB) -> Self {
        let min = vec3(
            self.min.x().min(other.min.x()),
            self.min.y().min(other.min.y()),
            self.min.z().min(other.min.z()),
        );
        let max = vec3(
            self.max.x().max(other.max.x()),
            self.max.y().max(other.max.y()),
            self.max.z().max(other.max.z()),
        );

        AABB::new(min, max)
    }

    pub fn point_union(self, other: Vec3) -> Self {
        let min = vec3(
            self.min.x().min(other.x()),
            self.min.y().min(other.y()),
            self.min.z().min(other.z()),
        );
        let max = vec3(
            self.max.x().max(other.x()),
            self.max.y().max(other.y()),
            self.max.z().max(other.z()),
        );

        AABB::new(min, max)
    }

    // Returns the axis which has greatest extent
    pub fn max_extent(&self) -> Axis {
        let extent = self.max - self.min;

        if extent.x() > extent.y() && extent.x() > extent.z() {
            Axis::X
        } else if extent.y() > extent.z() {
            Axis::Y
        } else {
            Axis::Z
        }
    }

    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x() * d.y() + d.x() * d.z() + d.y() * d.z())
    }
}

impl Intersect for AABB {
    fn intersection(&self, _ray: Ray, _t_min: f32, _t_max: f32) -> Option<Hit> {
        unimplemented!()
    }

    // Taken from tavianator.com
    fn has_intersection(&self, ray: Ray, _t_min: f32, _t_max: f32) -> bool {
        let t1 = (self.min - ray.origin) * ray.inv_direction;
        let t2 = (self.max - ray.origin) * ray.inv_direction;

        // X
        let tmin = f32::min(t1.x(), t2.x());
        let tmax = f32::max(t2.x(), t1.x());

        // Y
        let tmin = f32::max(tmin, f32::min(t1.y(), t2.y()));
        let tmax = f32::min(tmax, f32::max(t1.y(), t2.y()));

        // Z
        let tmin = f32::max(tmin, f32::min(t1.z(), t2.z()));
        let tmax = f32::min(tmax, f32::max(t1.z(), t2.z()));

        tmax >= f32::max(tmin, 0.0)
    }

    fn bounds(&self) -> Option<AABB> {
        Some(*self)
    }
}
