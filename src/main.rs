mod gl;
mod bvh;
mod camera;
mod material;
mod primitives;
// mod scene;
// mod textures;

use crate::{gl::Gl, bvh::*, camera::*, material::*, primitives::*};
use glutin::{
    event_loop::{EventLoop, ControlFlow},
    window::{WindowBuilder},
    event::{Event, WindowEvent, DeviceEvent, KeyboardInput, VirtualKeyCode},
    ContextBuilder,
};
use glam::{vec3, Vec3};
use rand::prelude::*;
use rayon::prelude::*;
use std::{
    sync::Arc,
    path::PathBuf,
};
use enum_dispatch::enum_dispatch;

static MAX_BOUNCES: u32 = 128;
static WIDTH: u32 = 1920;
static HEIGHT: u32 = 1080;
static SAMPLES: usize = 128;
static COLOR_CHANNELS: u32 = 3;
static GAMMA: f32 = 2.2;

type DefaultRng = rand_xoshiro::Xoshiro256Plus;

#[derive(Clone, Copy, Debug, Default)]
pub struct Ray {
    origin: Vec3,
    direction: Vec3,
    inv_direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let inv_direction = vec3(
            1.0 / direction.x(),
            1.0 / direction.y(),
            1.0 / direction.z(),
        );

        Self { origin, direction, inv_direction }
    }

    pub fn point_at_parameter(&self, t: f32) -> Vec3 {
        self.origin + t * self.direction
    }
}

// Computes whether a ray intersects the implementor
#[enum_dispatch]
pub trait Intersect: Send {
    fn intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<Hit>;
    fn has_intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> bool;
    fn bounds(&self) -> Option<AABB>;
}

// Contains data to be used in the generation of a new ray as a result of an intersection.
#[derive(Clone, Debug)]
pub struct Hit {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: Option<Arc<dyn Material + Send + Sync>>,
}

// Computes the color of a pixel/sample based on a ray
// Returns color and raycount
fn color(
    ray: Ray,
    bounces: &mut u32,
    bvh: &BVH,
    rng: &mut DefaultRng,
) -> Vec3 {
    // Max bounces
    if *bounces > MAX_BOUNCES {
        Vec3::zero()
    }
    // If the ray trace hits something
    else if let Some(hit) = bvh.intersection(ray, 0.0001, 10_000_000.0) {
        // The material of the object we hit decides how the ray scatters
        if let Some(scatter) = hit
            .material
            .clone()
            .and_then(|material| material.scatter(ray, hit, rng))
        {
            *bounces += 1;
            scatter.attenuation * color(scatter.scattered, bounces, bvh, rng)
        } else {
            // If we somehow hit something but dont scatter
            Vec3::zero()
        }
    // Else draw the background/skybox
    } else {
        let dir = ray.direction.normalize();
        let t = 0.5 * (dir.y() + 1.0);
        (1.0 - t) * vec3(1.0, 1.0, 1.0) + t * vec3(0.5, 0.7, 1.0)
    }
}

// Generate a semi random scene
fn random() -> Vec<Instance> {
    let mut rng = rand::thread_rng();
    let mut instances = Vec::new();

    let transform = Transform::default();

    // The big sphere
    let material = Arc::new(Lambertian::new(vec3(0.5, 0.5, 0.5)));
    let primitive = Arc::new(Sphere::new(vec3(0.0, -1000.0, 0.0), 1000.0).into());
    instances.push(Instance::reciver(primitive, material, transform));

    let primitive: Arc<Primitives> = Arc::new(Sphere::new(Vec3::zero(), 0.2).into());
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
                let transform = Transform { translation: center, scale: Vec3::one(), };
                instances.push(Instance::reciver(primitive.clone(), material, transform));
            }
        }
    }

    let material = Arc::new(Lambertian::new(vec3(0.6, 0.2, 0.9)));
    let primitive = Arc::new(Sphere::new(vec3(-4.0, 1.0, 0.0), 1.0).into());
    instances.push(Instance::reciver(primitive, material, transform));

    let material = Arc::new(Dielectric::new(1.5));
    let primitive = Arc::new(Sphere::new(vec3(0.0, 1.0, 0.0), 1.0).into());
    instances.push(Instance::reciver(primitive, material, transform));

    let material = Arc::new(Metal::new(vec3(0.7, 0.6, 0.5), 0.0));
    let primitive = Arc::new(Sphere::new(vec3(4.0, 1.0, 0.0), 1.0).into());
    instances.push(Instance::reciver(primitive, material, transform));

    instances
}

fn trace() -> Vec<u8> {
    let eye = vec3(13.0, 2.0, 3.0);
    let target = vec3(4.0, 1.0, 0.0);
    let up = vec3(0.0, 1.0, 0.0);
    let fov = 20.0;
    let aperture = 0.1;
    let camera = Camera::new(eye, target, up, fov, WIDTH as f32 / HEIGHT as f32, aperture);

    let mut buffer = vec![0u8; (WIDTH * HEIGHT * COLOR_CHANNELS) as usize];
    let primitives = random();
    let bvh = BVH::new(primitives);

    let global_ray_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let start = std::time::Instant::now();

    buffer
        .par_chunks_mut((WIDTH * COLOR_CHANNELS) as usize)
        .rev()
        .enumerate()
        .for_each(|(y, row)| {
            let mut rng = DefaultRng::from_entropy();
            row.chunks_mut(COLOR_CHANNELS as usize)
                .enumerate()
                .for_each(|(i, pixel)| {
                    let mut out = Vec3::zero();
                    let mut ray_count = 0;

                    // Antialiasing via multisampling
                    for _ in 0..SAMPLES {
                        let u = (rng.gen::<f32>() + i as f32) / WIDTH as f32;
                        let v = (rng.gen::<f32>() + y as f32) / HEIGHT as f32;

                        let ray = camera.ray(u, v, &mut rng);

                        let mut instance_ray_count = 1;
                        out += color(ray, &mut instance_ray_count, &bvh, &mut rng);
                        ray_count += instance_ray_count;
                    }

                    out /= SAMPLES as f32;

                    // Gamma correct
                    out = Vec3::new(
                        out.x().powf(1.0 / GAMMA),
                        out.y().powf(1.0 / GAMMA),
                        out.z().powf(1.0 / GAMMA),
                    );

                    // Convert from [0, 1] to [0, 256]
                    let r = (255.99 * out.x()) as u8;
                    let g = (255.99 * out.y()) as u8;
                    let b = (255.99 * out.z()) as u8;

                    // Write output color to buffer
                    pixel[0] = r;
                    pixel[1] = g;
                    pixel[2] = b;

                    global_ray_count.fetch_add(ray_count, std::sync::atomic::Ordering::Relaxed);
                })
        });

    let finished = std::time::Instant::now();
    let duration = finished.duration_since(start);
    let global_ray_count =
        f64::from(global_ray_count.load(std::sync::atomic::Ordering::Relaxed)) / 1_000_000.0;
    let rays_per_second = global_ray_count
        / (duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) / 1_000_000_000.0);
    println!(
        "Time elapsed: {:.2?}\nTotal Rays: {:.2}M\nRays per second: {:.2}M",
        duration, global_ray_count, rays_per_second
    );

    image::save_buffer("output.png", &buffer, WIDTH, HEIGHT, image::RGB(8)).unwrap();

    buffer
}

fn save_image(image: &[u8]) {
    image::save_buffer("output.png", image, WIDTH, HEIGHT, image::ColorType::RGB(8)).unwrap();
}

/// The user event given to glutin
pub enum PathtracerEvent {
    /// Signal that the pathtracer has made progress and that the window should be redrawn
    Redraw,
}

fn redraw(gl: &Gl) {
    unsafe {
        gl.ClearColor(1.0, 1.0, 1.0, 1.0);
        gl.DrawArrays(gl::TRIANGLES, 0, 4);
    }
}

fn main() {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let event_loop = EventLoop::with_user_event();
    let context = ContextBuilder::new()
        .with_gl(glutin::GlRequest::Latest)
        .with_gl_profile(glutin::GlProfile::Core)
        .build_windowed(
            WindowBuilder::new()
                .with_title("Pathtracer")
                .with_inner_size(glutin::dpi::LogicalSize::new(1920.0, 1080.0)),
            &event_loop
        )
        .unwrap();

    let context = unsafe { context.make_current().unwrap() };
    let gl = Gl::load_with(|name| context.get_proc_address(name) as *const _);

    let image = trace();

    unsafe {
        let mut tex = 0;
        gl.GenTextures(1, &mut tex);
        gl.BindTexture(gl::TEXTURE_2D, tex);
        gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_BORDER as i32);
        gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_BORDER as i32);
        gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        // gl.TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32, WIDTH as i32, HEIGHT as i32, 0, gl::RGB, gl::FLOAT, image.as_ptr() as _);
        // FIXME^: STATUS_ACCESS_VIOLATION

        let vertices = vec![
            -1.0,  1.0,
             1.0,  1.0,
             1.0, -1.0,
            -1.0, -1.0,
        ];

        let mut vert_buffer = 0;
        gl.GenBuffers(1, &mut vert_buffer);
        gl.BindBuffer(gl::ARRAY_BUFFER, vert_buffer);
        gl.BufferData(gl::ARRAY_BUFFER, (std::mem::size_of::<f32>() * vertices.len()) as isize, vertices.as_ptr() as _, gl::STATIC_DRAW);

        let load = |name: std::path::PathBuf| {
            use std::io::Read;

            let file = std::fs::File::open(name.clone()).expect(&format!("Failed to open file: {:?}", name));
            let mut reader = std::io::BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read(&mut buffer).unwrap();

            std::ffi::CString::new(buffer).unwrap()
        };

        let vert_shader = gl.CreateShader(gl::VERTEX_SHADER);
        let vert_source = load(root_dir.join("resources\\shaders\\present.vert"));
        gl.ShaderSource(vert_shader, 1, &vert_source.as_ptr(), std::ptr::null());
        gl.CompileShader(vert_shader);

        let frag_shader = gl.CreateShader(gl::FRAGMENT_SHADER);
        let frag_source = load(root_dir.join("resources\\shaders\\present.frag"));
        gl.ShaderSource(frag_shader, 1, &frag_source.as_ptr(), std::ptr::null());
        gl.CompileShader(frag_shader);

        let shader_program = gl.CreateProgram();
        gl.AttachShader(shader_program, vert_shader);
        gl.AttachShader(shader_program, frag_shader);

        gl.BindFragDataLocation(shader_program, 0, "color".as_ptr() as _);

        gl.LinkProgram(shader_program);
        gl.UseProgram(shader_program);

        let pos_attrib = gl.GetAttribLocation(shader_program, "position".as_ptr() as _) as u32;
        gl.VertexAttribPointer(pos_attrib, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
        gl.EnableVertexAttribArray(pos_attrib);

        redraw(&gl);
    }

    // Main event loop
    event_loop.run(move |event: Event<PathtracerEvent>, _, control_flow| {
        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(_size) => {
                    redraw(&gl);
                },
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            Event::DeviceEvent { ref event, .. } => match event {
                DeviceEvent::Key(KeyboardInput { virtual_keycode: Some(VirtualKeyCode::S), .. }) => {
                    println!("Saving rendered image...");
                    save_image(&image);
                },
                _ => (),
            },
            Event::UserEvent(ref event) => match event {
                PathtracerEvent::Redraw => {
                    redraw(&gl);
                },
            },
            _ => (),
        }
    });
}
