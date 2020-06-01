use std::time::Instant;

use anyhow::Result;
use c_str_macro::c_str;
use delta_e::DE2000;
use image;
use na::{Matrix4, Point3, Vector2, Vector3, Vector4};
use nalgebra as na;
use rayon::prelude::*;

use crate::individual::Individual;
use crate::render_gl;
use crate::resources::Resources;

pub struct Renderer {
    width: i32,
    height: i32,
    v_width: i32,
    v_height: i32,
    viewport: render_gl::Viewport,
    color_buffer: render_gl::ColorBuffer,
    _shader_program: render_gl::Program,
    render_texture: gl::types::GLuint,
    frame_buffer: gl::types::GLuint,
}

impl Renderer {
    pub fn new(width: i32, height: i32, res: &Resources) -> Result<Self> {
        unsafe {
            gl::Enable(gl::MULTISAMPLE);
        }

        let viewport = render_gl::Viewport::for_window(width as i32, height as i32);
        viewport.set_used();

        let color_buffer = render_gl::ColorBuffer::from_color(Vector3::zeros());
        color_buffer.set_used();

        let shader_program = render_gl::Program::from_res(&res, "shaders/stroke")?;
        let view_projection_loc;
        unsafe {
            view_projection_loc =
                gl::GetUniformLocation(shader_program.id(), c_str!("ViewProjection").as_ptr());
        }

        let view_matrix = Matrix4::look_at_rh(
            &Point3::new(0.0, 0.0, 1.0),
            &Point3::new(0.0, 0.0, 0.0),
            &Vector3::y(),
        );
        let projection_matrix = Matrix4::new_orthographic(
            -(width as f32 / 2.0),
            width as f32 / 2.0,
            -(height as f32 / 2.0),
            height as f32 / 2.0,
            -10.0,
            10.0,
        );
        let view_projection_matrix = projection_matrix * view_matrix;

        shader_program.set_used();
        unsafe {
            gl::UniformMatrix4fv(
                view_projection_loc,
                1 as gl::types::GLsizei,
                false as gl::types::GLboolean,
                view_projection_matrix.as_ptr(),
            );
        }

        let mut render_texture: gl::types::GLuint = 0;
        let mut frame_buffer: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut render_texture);
            gl::BindTexture(gl::TEXTURE_2D, render_texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::FLOAT,
                std::ptr::null() as *const gl::types::GLvoid,
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);

            gl::GenFramebuffers(1, &mut frame_buffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, frame_buffer);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                render_texture,
                0,
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Ok(Self {
            width,
            height,
            v_width: width,
            v_height: height,
            viewport,
            color_buffer,
            _shader_program: shader_program,
            render_texture,
            frame_buffer,
        })
    }

    fn render(&self, individual: &Individual) {
        self.color_buffer.clear();

        let mut vertices = vec![];
        let mut colors = vec![];
        individual.strokes.iter().for_each(|stroke| {
            for v in stroke.vertices() {
                vertices.push(Vector2::new(
                    v.x - self.width as f32 / 2.0,
                    (self.height as f32 - 1.0 - v.y) - self.height as f32 / 2.0,
                ));
                colors.push(stroke.color.clone());
            }
        });

        let vertices_vbo = render_gl::buffer::ArrayBuffer::new();
        vertices_vbo.bind();
        vertices_vbo.static_draw_data(&vertices);
        vertices_vbo.unbind();

        let colors_vbo = render_gl::buffer::ArrayBuffer::new();
        colors_vbo.bind();
        colors_vbo.static_draw_data(&colors);
        colors_vbo.unbind();

        let vao = render_gl::buffer::VertexArray::new();
        vao.bind();
        vertices_vbo.bind();
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
        }
        vertices_vbo.unbind();
        colors_vbo.bind();
        unsafe {
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 4, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
        }
        colors_vbo.unbind();

        // let start = Instant::now();

        unsafe {
            gl::DrawArrays(gl::TRIANGLES, 0, vertices.len() as i32);
        }

        // let end = start.elapsed();
        // println!("Draw {}.{:03}s", end.as_secs(), end.as_millis());
    }

    pub fn render_to_vec(&mut self, individual: &Individual) -> Vec<Vector4<f32>> {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.frame_buffer);
        }

        self.viewport.update_size(self.width, self.height);
        self.viewport.set_used();

        self.render(individual);

        let mut data = vec![Vector4::zeros(); (self.width * self.height) as usize];
        unsafe {
            gl::ReadPixels(
                0,
                0,
                self.width as i32,
                self.height as i32,
                gl::RGBA,
                gl::FLOAT,
                data.as_mut_ptr() as *mut gl::types::GLvoid,
            );
        }
        data
    }

    pub fn render_to_file(&mut self, individual: &Individual, output_path: &str) -> Result<()> {
        let data = self.render_to_vec(individual);

        let mut imgbuf = image::ImageBuffer::new(self.width as u32, self.height as u32);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let p = data[((self.height - 1 - y as i32) * self.width + x as i32) as usize];
            *pixel = image::Rgba([
                (255.0 * p.x) as u8,
                (255.0 * p.y) as u8,
                (255.0 * p.z) as u8,
                (255.0 * p.w) as u8,
            ]);
        }
        imgbuf.save(output_path)?;

        Ok(())
    }

    pub fn show(&mut self, individual: &Individual) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
        self.viewport.update_size(self.v_width, self.v_height);
        self.viewport.set_used();
        self.render(individual);
    }

    pub fn score(
        &mut self,
        individual: &Individual,
        colors: &Vec<Vector3<f32>>,
        importance: &Vec<f32>,
    ) -> f32 {
        let data = self.render_to_vec(individual);

        // let start = Instant::now();

        let score = data
            .par_iter()
            .enumerate()
            .map(|(i, v)| {
                let v0 = v;
                let v1 = colors[i];
                let w = importance[i];

                let c0 = [
                    (v0.x * 255.0) as u8,
                    (v0.y * 255.0) as u8,
                    (v0.z * 255.0) as u8,
                ];
                let c1 = [
                    (v1.x * 255.0) as u8,
                    (v1.y * 255.0) as u8,
                    (v1.z * 255.0) as u8,
                ];
                let color_loss = DE2000::from_rgb(&c0, &c1);

                let a0 = v0.w;
                let a1 = 1.0;
                let alpha_loss = (a0 - a1).powf(2.0) * 500.0;

                (color_loss + alpha_loss) * w
            })
            .sum();

        // let end = start.elapsed();
        // println!("Score {}.{:03}s", end.as_secs(), end.as_millis());

        score
    }

    pub fn update_viewport_size(&mut self, width: i32, height: i32) {
        self.v_width = width;
        self.v_height = height;
        self.viewport.update_size(width, height);
        self.viewport.set_used();
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &mut self.frame_buffer);
            gl::DeleteTextures(1, &mut self.render_texture);
        }
    }
}
