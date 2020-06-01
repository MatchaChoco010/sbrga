use std::path::Path;

use anyhow::{anyhow, Context, Result};
use c_str_macro::c_str;
use gl;
use image::GenericImageView;
use na::{Matrix4, Point3, Rotation, Vector3};
use nalgebra as na;
use sdl2;

use crate::render_gl;
use crate::resources::Resources;
use crate::triangle::Triangle;

pub fn visualize_direction_map(input: &str, x: i32, y: i32) -> Result<()> {
    let dir_map = image::open(input).unwrap();
    let (width, height) = dir_map.dimensions();
    let aspect = width as f64 / height as f64;

    let res =
        Resources::from_relative_exe_path(Path::new("assets")).context("resource path error")?;

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
        .window("Visualize Direction Map", width, height)
        .opengl()
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let _gl_context = window.gl_create_context().unwrap();
    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    unsafe {
        gl::Enable(gl::MULTISAMPLE);
    }

    let mut viewport = render_gl::Viewport::for_window(width as i32, height as i32);
    viewport.set_used();

    let color_buffer = render_gl::ColorBuffer::from_color(Vector3::zeros());
    color_buffer.set_used();

    let shader_program =
        render_gl::Program::from_res(&res, "shaders/triangle").context("shader load error")?;

    let model_loc;
    let view_projection_loc;
    unsafe {
        model_loc = gl::GetUniformLocation(shader_program.id(), c_str!("Model").as_ptr());
        view_projection_loc =
            gl::GetUniformLocation(shader_program.id(), c_str!("ViewProjection").as_ptr());
    }

    let mut model_matrices = vec![];
    {
        let wx = width as f32 / x as f32;
        let offset_x = -(width as f32 / 2.0) + wx / 2.0;
        let hy = height as f32 / y as f32;
        let offset_y = -(height as f32 / 2.0) + hy / 2.0;
        for x in 0..x {
            for y in 0..y {
                let x = wx * x as f32 + offset_x;
                let y = hy * y as f32 + offset_y;

                let scale = Matrix4::new_scaling(5.0);

                let dir = dir_map.get_pixel(
                    (x + (width as f32 / 2.0)).round() as u32,
                    (-y + (height as f32 / 2.0)).round() as u32,
                );
                let dir = Vector3::new(
                    (dir.0[0] as f32 / 255.0) * 2.0 - 1.0,
                    (dir.0[1] as f32 / 255.0) * 2.0 - 1.0,
                    0.0,
                );
                let rotation = Rotation::<_, na::U3>::rotation_between(&Vector3::y(), &dir)
                    .ok_or(anyhow!("calc rotation error"))?
                    .to_homogeneous();

                let translate = Matrix4::new_translation(&Vector3::new(x, y, 0.0));

                model_matrices.push(translate * rotation * scale);
            }
        }
    }

    let view_matrix = Matrix4::look_at_rh(
        &Point3::new(0.0, 0.0, 1.0),
        &Point3::new(0.0, 0.0, 0.0),
        &Vector3::y(),
    );
    let perspective_matrix = Matrix4::new_orthographic(
        -(width as f32 / 2.0),
        width as f32 / 2.0,
        -(height as f32 / 2.0),
        height as f32 / 2.0,
        -10.0,
        10.0,
    );
    let view_projection_matrix = perspective_matrix * view_matrix;

    let triangle_mesh = Triangle::new();

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
                    let width = width as u32;
                    let height = (width as f64 / aspect) as u32;
                    window.set_size(width, height).unwrap();
                    viewport.update_size(width as i32, height as i32);
                    viewport.set_used();
                }
                _ => {}
            }
        }

        color_buffer.clear();

        shader_program.set_used();

        unsafe {
            gl::UniformMatrix4fv(
                view_projection_loc,
                1 as gl::types::GLsizei,
                false as gl::types::GLboolean,
                view_projection_matrix.as_ptr(),
            );
        }

        for model_matrix in &model_matrices {
            unsafe {
                gl::UniformMatrix4fv(
                    model_loc,
                    1 as gl::types::GLsizei,
                    false as gl::types::GLboolean,
                    model_matrix.as_slice().as_ptr(),
                );
            }

            triangle_mesh.render();
        }

        window.gl_swap_window();
    }

    Ok(())
}
