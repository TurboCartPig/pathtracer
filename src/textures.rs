use glam::{vec3, Vec3};
use image::RgbImage;

pub trait Texture {
    fn value(&self, u: f32, v: f32) -> Vec3;
}

/// A texture with a constant uniform color
pub struct UniformTexture {
    color: Vec3,
}

impl UniformTexture {
    pub fn new(color: Vec3) -> Self {
        Self { color }
    }
}

impl Texture for UniformTexture {
    fn value(&self, _u: f32, _v: f32) -> Vec3 {
        self.color
    }
}

pub struct ImageTexture {
    image: RgbImage,
}

impl ImageTexture {
    pub fn new(image: RgbImage) -> Self {
        Self { image }
    }
}

impl Texture for ImageTexture {
    fn value(&self, u: f32, v: f32) -> Vec3 {
        let u = u * self.image.width() as f32;
        let v = v * self.image.height() as f32;

        let [r, g, b] = self.image.get_pixel(u as u32, v as u32).0;

        let r = r as f32 / 255.99;
        let g = g as f32 / 255.99;
        let b = b as f32 / 255.99;

        vec3(r, g, b)
    }
}
