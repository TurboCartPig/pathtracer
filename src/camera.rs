use crate::{material::sample_unit_sphere, DefaultRng, Ray};
use glam::Vec3;
use std::f32::consts::PI;

#[derive(Debug)]
pub struct Camera {
    origin: Vec3,
    lower_left_corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    lens_radius: f32,
}

impl Camera {
    pub fn new(origin: Vec3, target: Vec3, up: Vec3, vfov: f32, aspect: f32, apeture: f32) -> Self {
        let lens_radius = apeture / 2.0;
        let focus_dist = (origin - target).length();
        let theta = vfov * PI / 180.0;
        let half_height = f32::tan(theta / 2.0);
        let half_width = aspect * half_height;
        let w = (origin - target).normalize();
        let u = up.cross(w).normalize();
        let v = w.cross(u);
        let lower_left_corner =
            origin - half_width * focus_dist * u - half_height * focus_dist * v - focus_dist * w;
        let horizontal = 2.0 * half_width * focus_dist * u;
        let vertical = 2.0 * half_height * focus_dist * v;

        Self {
            origin,
            lower_left_corner,
            horizontal,
            vertical,
            u,
            v,
            lens_radius,
        }
    }

    pub fn ray(&self, s: f32, t: f32, rng: &mut DefaultRng) -> Ray {
        let rd = self.lens_radius * sample_unit_sphere(rng);
        let offset = self.u * rd.x() + self.v * rd.y();

        Ray::new(
            self.origin + offset,
            self.lower_left_corner + s * self.horizontal + t * self.vertical - self.origin - offset,
        )
    }
}
