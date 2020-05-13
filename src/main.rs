mod bvh;
mod camera;
mod gl;
mod material;
mod primitives;
mod ray;
mod scene;
// mod textures;

use crate::{bvh::*, material::*, primitives::*, ray::*, scene::*};
use glam::{vec3, Vec3};
// use glutin::{
//     event::{DeviceEvent, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
//     event_loop::{ControlFlow, EventLoop},
//     window::WindowBuilder,
//     ContextBuilder,
// };
use rand::prelude::*;
use serde::Deserialize;
use std::io::Read;
use std::sync::Arc;

/// Number of color channels to be used per pixel
static COLOR_CHANNELS: u32 = 3;

/// Default random number generator to be used
type DefaultRng = rand_xoshiro::Xoshiro256Plus;

/// Specifies settings used in the pathtracing
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct SettingsConfig {
    /// Resolution of the output image (width, height)
    resolution: [u32; 2],
    /// Number of samples per pixel
    samples: u32,
    /// Max bounces of a single primary ray
    max_bounces: u32,
    /// Gamma
    gamma: f32,
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            resolution: [1280, 720],
            samples: 12,
            max_bounces: 8,
            gamma: 2.2,
        }
    }
}

impl SettingsConfig {
    pub fn width(&self) -> u32 {
        self.resolution[0]
    }

    pub fn height(&self) -> u32 {
        self.resolution[1]
    }
}

/// Computes the color of a pixel/sample based on a ray
/// Returns color and raycount
fn color(ray: Ray, bounces: &mut u32, bvh: &BVH, rng: &mut DefaultRng, max_bounces: u32) -> Vec3 {
    // Max bounces
    if *bounces >= max_bounces {
        Vec3::zero()
    }
    // If the ray trace hits something
    else if let Some(hit) = bvh.intersection(ray, 0.0001, 10_000_000.0) {
        // The material of the object we hit decides how the ray scatters
        hit.material
            .clone()
            .and_then(|material| material.scatter(ray, hit, rng))
            .map(|scatter| {
                *bounces += 1;
                scatter.attenuation * color(scatter.scattered, bounces, bvh, rng, max_bounces)
            })
            .unwrap_or(Vec3::zero())
    } else {
        // Else draw the background/skybox
        let dir = ray.direction.normalize();
        let t = 0.5 * (dir.y() + 1.0);
        (1.0 - t) * vec3(1.0, 1.0, 1.0) + t * vec3(0.5, 0.7, 1.0)
    }
}

/// Generate a semi random scene
// TODO: Move to scene
fn random() -> Vec<Instance> {
    let mut rng = rand::thread_rng();
    let mut instances = Vec::new();

    let transform = Transform::default();

    // The big sphere
    let material = Arc::new(Lambertian::new(vec3(0.5, 0.5, 0.5)));
    let primitive = Arc::new(Sphere::new(vec3(0.0, -1000.0, 0.0), 1000.0));
    instances.push(Instance::reciver(primitive, material, transform));

    let primitive = Arc::new(Sphere::new(Vec3::zero(), 0.2));
    for a in -12..12 {
        for b in -12..12 {
            let material = rng.gen::<f32>();
            let center = vec3(
                a as f32 + 0.9 * rng.gen::<f32>(),
                0.2,
                b as f32 + 0.9 * rng.gen::<f32>(),
            );

            if (center - vec3(4.0, 0.2, 0.0)).length() > 0.9 {
                let r = vec3(
                    rng.gen::<f32>() * rng.gen::<f32>(),
                    rng.gen::<f32>() * rng.gen::<f32>(),
                    rng.gen::<f32>() * rng.gen::<f32>(),
                );

                // Lambertian
                let material: Arc<dyn Material + Send + Sync> = if material < 0.5 {
                    Arc::new(Lambertian::new(r))
                // Metal
                } else if material < 0.75 {
                    Arc::new(Metal::new(r, rng.gen::<f32>()))
                // Dielectric
                } else {
                    Arc::new(Dielectric::new(1.5))
                };
                let transform = Transform {
                    translation: center,
                    ..Default::default()
                };
                instances.push(Instance::reciver(primitive.clone(), material, transform));
            }
        }
    }

    let material = Arc::new(Lambertian::new(vec3(0.6, 0.2, 0.9)));
    let primitive = Arc::new(Sphere::new(vec3(-4.0, 1.0, 0.0), 1.0));
    instances.push(Instance::reciver(primitive, material, transform));

    let material = Arc::new(Dielectric::new(1.5));
    let primitive = Arc::new(Sphere::new(vec3(0.0, 1.0, 0.0), 1.0));
    instances.push(Instance::reciver(primitive, material, transform));

    let material = Arc::new(Metal::new(vec3(0.7, 0.6, 0.5), 0.0));
    let primitive = Arc::new(Sphere::new(vec3(4.0, 1.0, 0.0), 1.0));
    instances.push(Instance::reciver(primitive, material, transform));

    instances
}

/// The user event given to glutin
// pub enum PathtracerEvent {
//     /// Signal that the pathtracer has made progress and that the window should be redrawn
//     Redraw,
// }

// fn redraw(gl: &Gl) {
//     unsafe {
//         gl.ClearColor(1.0, 0.0, 1.0, 1.0);
//         gl.Clear(gl::COLOR_BUFFER_BIT);
//         gl.DrawArrays(gl::TRIANGLES, 0, 4);
//     }
// }

/// Load pathtracer settings from a settings file
fn load_settings() -> anyhow::Result<SettingsConfig> {
    let mut settings = std::fs::File::open("settings.toml")?;
    let mut buffer = String::new();
    settings.read_to_string(&mut buffer)?;
    let settings: SettingsConfig = toml::from_str(&buffer)?;

    Ok(settings)
}

fn main() {
    // Load in settings
    let settings: SettingsConfig = load_settings().unwrap_or(Default::default());

    let scene = Scene::new(settings, random());
    let image = scene.trace();
    image.save("output.png").unwrap();

    // TODO: Fix opengl previewer
    // *******************************************************************
    // let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // let event_loop = EventLoop::with_user_event();
    // let context = ContextBuilder::new()
    //     .with_gl(glutin::GlRequest::Latest)
    //     .with_gl_profile(glutin::GlProfile::Core)
    //     .build_windowed(
    //         WindowBuilder::new()
    //             .with_title("Pathtracer")
    //             .with_inner_size(glutin::dpi::LogicalSize::new(1920.0, 1080.0)),
    //         &event_loop,
    //     )
    //     .unwrap();

    // let context = unsafe { context.make_current().unwrap() };
    // let gl = Gl::load_with(|name| context.get_proc_address(name) as *const _);

    // unsafe {
    //     let mut tex = 0;
    //     gl.GenTextures(1, &mut tex);
    //     gl.BindTexture(gl::TEXTURE_2D, tex);
    //     gl.TexImage2D(
    //         gl::TEXTURE_2D,
    //         0,
    //         gl::RGB as i32,
    //         WIDTH as i32,
    //         HEIGHT as i32,
    //         0,
    //         gl::RGB,
    //         gl::UNSIGNED_BYTE,
    //         image.as_ptr() as _,
    //     );
    //     gl.TexParameteri(
    //         gl::TEXTURE_2D,
    //         gl::TEXTURE_WRAP_S,
    //         gl::CLAMP_TO_BORDER as i32,
    //     );
    //     gl.TexParameteri(
    //         gl::TEXTURE_2D,
    //         gl::TEXTURE_WRAP_T,
    //         gl::CLAMP_TO_BORDER as i32,
    //     );
    //     gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    //     gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

    //     let mut vao = 0;
    //     gl.GenVertexArrays(1, &mut vao);
    //     gl.BindVertexArray(vao);

    //     let vertices = vec![-1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0];

    //     let mut vert_buffer = 0;
    //     gl.GenBuffers(1, &mut vert_buffer);
    //     gl.BindBuffer(gl::ARRAY_BUFFER, vert_buffer);
    //     gl.BufferData(
    //         gl::ARRAY_BUFFER,
    //         (std::mem::size_of::<f32>() * vertices.len()) as isize,
    //         vertices.as_ptr() as _,
    //         gl::STATIC_DRAW,
    //     );

    //     let load = |name: std::path::PathBuf| {
    //         use std::io::Read;

    //         let file = std::fs::File::open(name.clone())
    //             .expect(&format!("Failed to open file: {:?}", name));
    //         let mut reader = std::io::BufReader::new(file);
    //         let mut buffer = Vec::new();
    //         reader.read(&mut buffer).unwrap();

    //         std::ffi::CString::new(buffer).unwrap()
    //     };

    //     let vert_shader = gl.CreateShader(gl::VERTEX_SHADER);
    //     let vert_source = load(root_dir.join("resources\\shaders\\present.vert"));
    //     gl.ShaderSource(vert_shader, 1, &vert_source.as_ptr(), std::ptr::null());
    //     gl.CompileShader(vert_shader);

    //     let mut status = 0;
    //     gl.GetShaderiv(vert_shader, gl::COMPILE_STATUS, &mut status);
    //     println!("Shader compilation status: {}", status);

    //     let frag_shader = gl.CreateShader(gl::FRAGMENT_SHADER);
    //     let frag_source = load(root_dir.join("resources\\shaders\\present.frag"));
    //     gl.ShaderSource(frag_shader, 1, &frag_source.as_ptr(), std::ptr::null());
    //     gl.CompileShader(frag_shader);

    //     let mut status = 0;
    //     gl.GetShaderiv(frag_shader, gl::COMPILE_STATUS, &mut status);
    //     println!("Shader compilation status: {}", status);

    //     let shader_program = gl.CreateProgram();
    //     gl.AttachShader(shader_program, vert_shader);
    //     gl.AttachShader(shader_program, frag_shader);

    //     gl.BindFragDataLocation(shader_program, 0, "color".as_ptr() as _);

    //     gl.LinkProgram(shader_program);
    //     gl.UseProgram(shader_program);

    //     let pos_attrib = gl.GetAttribLocation(shader_program, b"position\0".as_ptr() as _) as u32;
    //     gl.EnableVertexAttribArray(pos_attrib);
    //     gl.VertexAttribPointer(
    //         pos_attrib,
    //         2,
    //         gl::FLOAT,
    //         gl::FALSE,
    //         2 * std::mem::size_of::<f32>() as i32,
    //         std::ptr::null(),
    //     );

    //     // let tex_coord_attrib = gl.GetAttribLocation(shader_program, "tex_coord".as_ptr() as _) as u32;
    //     // gl.VertexAttribPointer(tex_coord_attrib, 3, gl::UNSIGNED_BYTE, gl::FALSE, 4 * std::mem::size_of::<f32>() as i32, (2 * std::mem::size_of::<f32>() as i32) as _);

    //     redraw(&gl);
    // }

    // Main event loop
    // event_loop.run(move |event: Event<PathtracerEvent>, _, control_flow| {
    //     match event {
    //         Event::LoopDestroyed => return,
    //         Event::WindowEvent { ref event, .. } => match event {
    //             // WindowEvent::Resized(_size) => {
    //             //     redraw(&gl);
    //             // },
    //             WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
    //             _ => (),
    //         },
    //         Event::DeviceEvent { ref event, .. } => match event {
    //             DeviceEvent::Key(KeyboardInput {
    //                 virtual_keycode: Some(VirtualKeyCode::S),
    //                 ..
    //             }) => {
    //                 println!("Saving rendered image...");
    //                 save_image(&image);
    //             }
    //             _ => (),
    //         },
    //         Event::RedrawRequested(_) => {
    //             redraw(&gl);
    //             context.swap_buffers().unwrap();
    //             println!("Hei");
    //         }
    //         // Event::UserEvent(ref event) => match event {
    //         //     PathtracerEvent::Redraw => {
    //         //         redraw(&gl);
    //         //     },
    //         // },
    //         _ => (),
    //     }
    // });
}
