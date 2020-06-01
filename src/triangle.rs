use gl;
use na::Point3;
use nalgebra as na;

use crate::render_gl::buffer;

pub struct Triangle {
    _vbo: buffer::ArrayBuffer,
    vao: buffer::VertexArray,
}

impl Triangle {
    pub fn new() -> Self {
        let vertices: Vec<Point3<f32>> = vec![
            Point3::new(-0.5, -1.0, 0.0),
            Point3::new(0.5, -1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];

        let vbo = buffer::ArrayBuffer::new();
        vbo.bind();
        vbo.static_draw_data(&vertices);
        vbo.unbind();

        let vao = buffer::VertexArray::new();
        vao.bind();
        vbo.bind();
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
        }
        vbo.unbind();
        vao.unbind();

        Self { _vbo: vbo, vao }
    }

    pub fn render(&self) {
        self.vao.bind();
        unsafe {
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }
    }
}
