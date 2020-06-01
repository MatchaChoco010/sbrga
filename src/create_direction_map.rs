use image::{GenericImageView, RgbImage};
use na::{Vector2, Vector3};
use nalgebra as na;
use rayon::prelude::*;

pub fn create_direction_map(input: &str, output: &str) {
    let normal_map = image::open(input).unwrap();
    let (width, height) = normal_map.dimensions();
    let mut output_image = RgbImage::new(width, height);
    output_image
        .enumerate_pixels_mut()
        .collect::<Vec<_>>()
        .par_iter_mut()
        .for_each(|(x, y, pixel)| {
            let normal = normal_map.get_pixel(*x, *y);
            let normal = Vector3::new(
                normal.0[0] as f64 / 255.0,
                normal.0[1] as f64 / 255.0,
                normal.0[2] as f64 / 255.0,
            );
            let normal = 2.0 * normal - Vector3::new(1.0, 1.0, 1.0);
            let view_dir = Vector3::<f64>::new(0.0, 0.0, 1.0);
            let dir = normal.normalize().cross(&view_dir);
            let dir = Vector2::new(dir.x, dir.y).normalize();
            let dir = (dir + Vector2::<f64>::new(1.0, 1.0)) * 0.5;
            pixel[0] = (255.0 * dir.x) as u8;
            pixel[1] = (255.0 * dir.y) as u8;
            pixel[2] = 0;
        });
    output_image.save(output).unwrap();
}
