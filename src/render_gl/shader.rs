use std::ffi::{CStr, CString};

use gl;
use thiserror::Error;

use crate::resources::{self, Resources};

#[derive(Error, Debug)]
pub enum ShaderError {
    #[error("resource load error: {name}")]
    ResourceLoad {
        name: String,
        source: resources::ResError,
    },
    #[error("can not determine shader type for resource: {name}")]
    CanNotDetermineShaderTypeForResource { name: String },
    #[error("shader compile error: {name}\nmessage: {message}")]
    CompileError { name: String, message: String },
    #[error("shader link error: {name}\nmessage: {message}")]
    LinkError { name: String, message: String },
}

pub struct Shader {
    id: gl::types::GLuint,
}

impl Shader {
    pub fn from_source(
        source: &CStr,
        name: &str,
        kind: gl::types::GLenum,
    ) -> Result<Shader, ShaderError> {
        let id = shader_from_source(source, name, kind)?;
        Ok(Shader { id })
    }

    pub fn from_vert_source(source: &CStr, name: &str) -> Result<Shader, ShaderError> {
        Shader::from_source(source, name, gl::VERTEX_SHADER)
    }

    pub fn from_frag_source(source: &CStr, name: &str) -> Result<Shader, ShaderError> {
        Shader::from_source(source, name, gl::FRAGMENT_SHADER)
    }

    pub fn from_res(res: &Resources, name: &str) -> Result<Shader, ShaderError> {
        const POSSIBLE_EXT: [(&str, gl::types::GLenum); 2] =
            [(".vert", gl::VERTEX_SHADER), (".frag", gl::FRAGMENT_SHADER)];

        let shader_kind = POSSIBLE_EXT
            .iter()
            .find(|&&(file_extension, _)| name.ends_with(file_extension))
            .map(|&(_, kind)| kind)
            .ok_or_else(|| ShaderError::CanNotDetermineShaderTypeForResource {
                name: name.into(),
            })?;

        let source = res
            .load_cstring(name)
            .map_err(|e| ShaderError::ResourceLoad {
                name: name.into(),
                source: e,
            })?;

        Self::from_source(&source, name, shader_kind)
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

pub struct Program {
    id: gl::types::GLuint,
}

impl Program {
    pub fn from_shaders(name: &str, shaders: &[Shader]) -> Result<Program, ShaderError> {
        let program_id = unsafe { gl::CreateProgram() };
        for shader in shaders {
            unsafe {
                gl::AttachShader(program_id, shader.id());
            }
        }
        unsafe {
            gl::LinkProgram(program_id);
        }
        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }
        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe {
                gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            }
            let error = create_whitespace_cstring_with_len(len as usize);
            unsafe {
                gl::GetProgramInfoLog(
                    program_id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar,
                );
            }
            return Err(ShaderError::LinkError {
                name: name.to_string(),
                message: error.to_string_lossy().into_owned(),
            });
        }
        for shader in shaders {
            unsafe {
                gl::DetachShader(program_id, shader.id());
            }
        }
        Ok(Program { id: program_id })
    }

    pub fn from_res(res: &Resources, name: &str) -> Result<Program, ShaderError> {
        const POSSIBLE_EXT: [&str; 2] = [".vert", ".frag"];

        let shaders = POSSIBLE_EXT
            .iter()
            .map(|file_extension| Shader::from_res(res, &format!("{}{}", name, file_extension)))
            .collect::<Result<Vec<Shader>, ShaderError>>()?;

        Program::from_shaders(name, &shaders[..])
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }

    pub fn set_used(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

fn shader_from_source(
    source: &CStr,
    name: &str,
    kind: gl::types::GLenum,
) -> Result<gl::types::GLuint, ShaderError> {
    let id = unsafe { gl::CreateShader(kind) };
    unsafe {
        gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(id);
    }
    let mut success: gl::types::GLint = 1;
    unsafe {
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }
    if success == 0 {
        let mut len: gl::types::GLint = 0;
        unsafe {
            gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
        }
        let error = create_whitespace_cstring_with_len(len as usize);
        unsafe {
            gl::GetShaderInfoLog(
                id,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar,
            );
        }
        return Err(ShaderError::CompileError {
            name: name.to_string(),
            message: error.to_string_lossy().into_owned(),
        });
    }
    Ok(id)
}

fn create_whitespace_cstring_with_len(len: usize) -> CString {
    // allocate buffer of correct size
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    // fill it with len spaces
    buffer.extend([b' '].iter().cycle().take(len));
    // convert buffer to CString
    unsafe { CString::from_vec_unchecked(buffer) }
}
