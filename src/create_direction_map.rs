use image::{GenericImageView, RgbImage};
use na::{Vector2, Vector3};
use nalgebra as na;
use rayon::prelude::*;

pub fn create_direction_map_from_normal(input: &str, output: &str) {
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

pub fn create_direction_map_from_edge(input: &str, output: &str) {
    let edge_map = image::open(input).unwrap();
    let (width, height) = edge_map.dimensions();

    let edge_points = edge_map
        .to_luma()
        .enumerate_pixels()
        .filter(|(_, _, pixel)| pixel[0] >= 128)
        .map(|(x, y, _)| (x, y))
        .collect::<Vec<_>>();

    let distance_map = (0..(width * height))
        .collect::<Vec<_>>()
        .par_iter()
        .map(|index| {
            let (x, y) = (index % width, index / width);
            edge_points
                .iter()
                .map(|(ex, ey)| {
                    (x as f64 - *ex as f64).powf(2.0) + (y as f64 - *ey as f64).powf(2.0)
                })
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        })
        .collect::<Vec<_>>();

    let mut output_image = RgbImage::new(width, height);
    output_image
        .enumerate_pixels_mut()
        .collect::<Vec<_>>()
        .par_iter_mut()
        .for_each(|(x, y, pixel)| {
            let index = (*x + *y * width) as usize;
            let (width, height) = (width as usize, height as usize);
            let p = distance_map[index];
            let p_x_prev = if index % width == 0 {
                distance_map[index]
            } else {
                distance_map[index - 1]
            };
            let p_x_next = if index % width == width - 1 {
                distance_map[index]
            } else {
                distance_map[index + 1]
            };
            let p_y_prev = if index / width == 0 {
                distance_map[index]
            } else {
                distance_map[index - width]
            };
            let p_y_next = if index / width == height - 1 {
                distance_map[index]
            } else {
                distance_map[index + width]
            };
            let dir = Vector2::new(
                -((p_y_next + p) - (p + p_y_prev)),
                (p_x_next + p) - (p + p_x_prev),
            )
            .normalize();
            let dir = (dir + Vector2::<f64>::new(1.0, 1.0)) * 0.5;
            pixel[0] = (255.0 * dir.x) as u8;
            pixel[1] = (255.0 * dir.y) as u8;
            pixel[2] = 0;
        });
    output_image.save(output).unwrap();
}
