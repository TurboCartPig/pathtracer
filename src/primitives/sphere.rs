use crate::{primitives::AABB, Hit, Intersect, Ray};
use glam::{vec3, Vec3};

#[derive(Clone, Debug)]
pub struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
}

impl Intersect for Sphere {
    fn intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        let oc = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let b = oc.dot(ray.direction);
        let c = oc.dot(oc) - self.radius * self.radius;
        let discriminant = b * b - a * c;

        if discriminant > 0.0 {
            let t_1 = (-b - f32::sqrt(b * b - a * c)) / a;
            let t_2 = (-b + f32::sqrt(b * b - a * c)) / a;

            for &t in &[t_1, t_2] {
                if t_min < t && t < t_max {
                    let point = ray.point_at_parameter(t);

                    return Some(Hit {
                        t,
                        point,
                        normal: (point - self.center) / self.radius,
                        material: None,
                    });
                }
            }
        }

        None
    }

    fn has_intersection(&self, ray: Ray, _t_min: f32, _t_max: f32) -> bool {
        let oc = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let b = oc.dot(ray.direction);
        let c = oc.dot(oc) - self.radius * self.radius;
        let discriminant = b * b - a * c;

        discriminant > 0.0
    }

    fn bounds(&self) -> Option<AABB> {
        Some(AABB::new(
            self.center - vec3(self.radius, self.radius, self.radius),
            self.center + vec3(self.radius, self.radius, self.radius),
        ))
    }
}
