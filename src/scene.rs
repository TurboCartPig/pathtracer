use crate::{bvh::BVH, camera::Camera, primitives::Sphere, material::Material};
use std::collections::HashMap;

// pub struct LoadedScene {
//     primitives: Primitives,
//     materials: Materials,
//     textures: Textures,
// }

pub struct Scene {
    camera: Camera,
    tree: BVH<Sphere>,
    materials: Materials,
    // textures: Textures,
}

pub struct Materials {
    inner: HashMap<String, Box<dyn Material + Send + Sync>>,
}
