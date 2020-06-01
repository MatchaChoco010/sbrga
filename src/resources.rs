use std::ffi;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("file contains nil")]
    FileContainsNil,
    #[error("failed to get exe path")]
    FailedToGetExePath,
}

pub struct Resources {
    root_path: PathBuf,
}

impl Resources {
    pub fn from_relative_exe_path(rel_path: &Path) -> Result<Resources, ResError> {
        let exe_file_name = ::std::env::current_exe().map_err(|_| ResError::FailedToGetExePath)?;
        let exe_path = exe_file_name.parent().ok_or(ResError::FailedToGetExePath)?;
        Ok(Resources {
            root_path: exe_path.join(rel_path),
        })
    }

    pub fn load_cstring(&self, resource_name: &str) -> Result<ffi::CString, ResError> {
        let mut file = fs::File::open(resource_name_to_path(&self.root_path, resource_name))?;

        // allocate buffer of the same size as file
        let mut buffer: Vec<u8> = Vec::with_capacity(file.metadata()?.len() as usize + 1);
        file.read_to_end(&mut buffer)?;

        // check for nul byte
        if buffer.iter().find(|i| **i == 0).is_some() {
            return Err(ResError::FileContainsNil);
        }

        Ok(unsafe { ffi::CString::from_vec_unchecked(buffer) })
    }
}

fn resource_name_to_path(root_dir: &Path, location: &str) -> PathBuf {
    let mut path: PathBuf = root_dir.into();
    for part in location.split("/") {
        path = path.join(part);
    }
    path
}
