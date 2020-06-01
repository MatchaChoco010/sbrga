use std::path::Path;

use anyhow::Result;
use image::{self, GenericImageView};
use na::{Vector2, Vector3};
use nalgebra as na;

use crate::individual::Individual;
use crate::renderer;
use crate::resources::Resources;

pub fn create_individual(
    color_map: &str,
    dir_map: &str,
    importance_map: &str,
    output_path: &str,
    stroke_num: u32,
    stroke_thickness: f32,
) -> Result<()> {
    let color_map = image::open(color_map).unwrap();
    let dir_map = image::open(dir_map).unwrap();
    let importance_map = image::open(importance_map).unwrap();

    if color_map.dimensions() != dir_map.dimensions()
        || dir_map.dimensions() != importance_map.dimensions()
    {
        panic!("The maps are different sizes.")
    }

    let (width, height) = color_map.dimensions();
    let (width, height) = (width as i32, height as i32);
    let aspect = width as f64 / height as f64;

    let colors = color_map
        .pixels()
        .map(|(_, _, p)| {
            Vector3::new(
                p[0] as f32 / 255.0,
                p[1] as f32 / 255.0,
                p[2] as f32 / 255.0,
            )
        })
        .collect::<Vec<_>>();

    let directions = dir_map
        .pixels()
        .map(|(_, _, p)| {
            Vector2::new(
                p[0] as f32 / 255.0 * 2.0 - 1.0,
                p[1] as f32 / 255.0 * 2.0 - 1.0,
            )
            .normalize()
        })
        .collect::<Vec<_>>();

    let importance = importance_map
        .pixels()
        .map(|(_, _, p)| p[0] as f32 / 255.0)
        .collect::<Vec<_>>();

    let individual = Individual::new(
        &colors,
        &directions,
        &importance,
        width,
        height,
        stroke_num,
        stroke_thickness,
    );

    let res = Resources::from_relative_exe_path(Path::new("assets"))?;

    let sdl = sdl2::init().unwrap();
    let video_subsystem = sdl.video().unwrap();

    {
        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(4, 6);
        let (major, minor) = gl_attr.context_version();
        println!("OK: init OpenGL: version={}.{}", major, minor);
        gl_attr.set_multisample_samples(4);
    }

    let mut window = video_subsystem
        .window("Create an individual painting", width as u32, height as u32)
        .opengl()
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let _gl_context = window.gl_create_context().unwrap();
    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    let mut renderer = renderer::Renderer::new(width, height, &res)?;

    renderer.render_to_file(&individual, output_path)?;

    let mut event_pump = sdl.event_pump().unwrap();
    'main: loop {
        for event in event_pump.poll_iter() {
            use sdl2::event::Event;
            use sdl2::event::WindowEvent;
            use sdl2::keyboard::Keycode;
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                Event::Window {
                    win_event: WindowEvent::Resized(width, _),
                    ..
                } => {
                    let width = width as i32;
                    let height = (width as f64 / aspect) as i32;
                    renderer.update_viewport_size(width, height);
                    window.set_size(width as u32, height as u32).unwrap();
                }
                _ => {}
            }
        }

        renderer.show(&individual);

        window.gl_swap_window();
    }

    Ok(())
}
