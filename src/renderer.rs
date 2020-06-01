use anyhow::Result;
use c_str_macro::c_str;
use delta_e::DE2000;
use image;
use na::{Matrix4, Point2, Point3, Vector2, Vector3, Vector4};
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
    color_loc: gl::types::GLint,
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
        let color_loc;
        unsafe {
            view_projection_loc =
                gl::GetUniformLocation(shader_program.id(), c_str!("ViewProjection").as_ptr());
            color_loc = gl::GetUniformLocation(shader_program.id(), c_str!("Color").as_ptr());
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
            color_loc,
        })
    }

    fn render(&self, individual: &Individual) {
        const CATMULL_ROM_SUBDIVISION: i32 = 3;

        self.color_buffer.clear();

        for stroke in &individual.strokes {
            let catmull_rom_vertices = {
                let mut vertices: Vec<Point2<f32>> = vec![];
                for i in 1..stroke.hopping_point.len() {
                    for t in 0..CATMULL_ROM_SUBDIVISION {
                        let t = t as f32 / CATMULL_ROM_SUBDIVISION as f32;
                        let x = if i == 1 {
                            let p1 = &stroke.hopping_point[i - 1].coords;
                            let p2 = &stroke.hopping_point[i].coords;
                            let p3 = &stroke.hopping_point[i + 1].coords;
                            0.5 * ((p1 - 2.0 * p2 + p3) * t * t
                                + (-3.0 * p1 + 4.0 * p2 - p3) * t
                                + 2.0 * p1)
                        } else if i == stroke.hopping_point.len() - 1 {
                            let p0 = &stroke.hopping_point[i - 2].coords;
                            let p1 = &stroke.hopping_point[i - 1].coords;
                            let p2 = &stroke.hopping_point[i].coords;
                            0.5 * ((p0 - 2.0 * p1 + p2) * t * t + (-p0 + p2) * t + 2.0 * p1)
                        } else {
                            let p0 = &stroke.hopping_point[i - 2].coords;
                            let p1 = &stroke.hopping_point[i - 1].coords;
                            let p2 = &stroke.hopping_point[i].coords;
                            let p3 = &stroke.hopping_point[i + 1].coords;
                            0.5 * ((-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t
                                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
                                + (-p0 + p3) * t
                                + 2.0 * p1)
                        };
                        vertices.push(Point2::new(x.x, x.y));
                    }
                }
                vertices
            };

            let vertices = {
                let mut vertices: Vec<Point2<f32>> = vec![];
                for i in 1..catmull_rom_vertices.len() {
                    let p0 = if i == 1 {
                        &catmull_rom_vertices[0]
                    } else {
                        &catmull_rom_vertices[i - 2]
                    };
                    let p1 = &catmull_rom_vertices[i - 1];
                    let p2 = &catmull_rom_vertices[i];
                    let p3 = if i == catmull_rom_vertices.len() - 1 {
                        &catmull_rom_vertices[catmull_rom_vertices.len() - 1]
                    } else {
                        &catmull_rom_vertices[i + 1]
                    };
                    let d0 = (p2.coords - p0.coords).normalize();
                    let d0 = Vector2::new(d0.y, -d0.x).normalize();
                    let d1 = (p3.coords - p1.coords).normalize();
                    let d1 = Vector2::new(d1.y, -d1.x).normalize();
                    let v0 = p1 + d0 * stroke.thickness / 2.0;
                    let v1 = p1 - d0 * stroke.thickness / 2.0;
                    let v2 = p2 + d1 * stroke.thickness / 2.0;
                    let v3 = p2 - d1 * stroke.thickness / 2.0;
                    vertices.push(v0.clone());
                    vertices.push(v1.clone());
                    vertices.push(v2.clone());
                    vertices.push(v2.clone());
                    vertices.push(v1.clone());
                    vertices.push(v3.clone());
                }
                vertices
            };

            let vertices = vertices
                .iter()
                .map(|p| {
                    Vector3::new(
                        p.coords.x - self.width as f32 / 2.0,
                        (self.height as f32 - 1.0 - p.coords.y) - self.height as f32 / 2.0,
                        0.0,
                    )
                })
                .collect::<Vec<_>>();

            let vbo = render_gl::buffer::ArrayBuffer::new();
            vbo.bind();
            vbo.static_draw_data(&vertices);
            vbo.unbind();

            let vao = render_gl::buffer::VertexArray::new();
            vao.bind();
            vbo.bind();
            unsafe {
                gl::EnableVertexAttribArray(0);
                gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            }
            vbo.unbind();
            vao.unbind();

            unsafe {
                gl::Uniform4f(
                    self.color_loc,
                    stroke.color.x,
                    stroke.color.y,
                    stroke.color.z,
                    stroke.color.w,
                );
            }
            vao.bind();
            unsafe {
                gl::DrawArrays(gl::TRIANGLES, 0, vertices.len() as i32);
            }
        }
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
        data.par_iter()
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
                let alpha_loss = (a0 - a1).powf(2.0) * 100.0;

                (color_loss + alpha_loss) * w
            })
            .sum()
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
