use delta_e::DE2000;
use lerp::Lerp;
use na::{Point2, Rotation2, Vector2, Vector3, Vector4};
use nalgebra as na;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rand_distr::Normal;

#[derive(Clone)]
pub struct Stroke {
    pub pos: Vector2<i32>,
    pub color: Vector4<f32>,
    pub hopping_point: Vec<Point2<f32>>,
    pub thickness: f32,
    importance: f32,
}

impl Stroke {
    pub fn new(
        index: usize,
        colors: &Vec<Vector3<f32>>,
        directions: &Vec<Vector2<f32>>,
        importance: &Vec<f32>,
        width: i32,
        #[allow(unused_variables)] height: i32,
        stroke_thickness: f32,
    ) -> Self {
        const THICKNESS_MIN_MEAN: f32 = 4.0;
        const THICKNESS_MIN_VARIANCE: f32 = 2.0;
        const THICKNESS_MAX_MEAN: f32 = 50.0;
        const THICKNESS_MAX_VARIANCE: f32 = 40.0;
        const THICKNESS_MIN: f32 = 0.3;
        const THICKNESS_T_POW: f32 = 1.0 / 1.8;

        const HOP_LENGTH_SCALING_FACTOR_MEAN: f32 = 2.0;
        const HOP_LENGTH_SCALING_FACTOR_VARIANCE: f32 = 0.5;
        const HOP_LENGTH_MIN_SCALING_FACTOR: f32 = 0.5;
        #[allow(non_snake_case)]
        let HOP_ANGLE_VARIANCE: f32 = 10.0_f32.to_radians();
        #[allow(non_snake_case)]
        let HOP_ANGLE_MAX: f32 = 45.0_f32.to_radians();

        const HOP_END_COLOR_DISTANCE_MEAN: f32 = 5.0;
        const HOP_END_COLOR_DISTANCE_VARIANCE: f32 = 10.0;
        const HOP_END_COLOR_DISTANCE_MIN: f32 = 2.0;

        let mut rng = thread_rng();

        let pos = {
            let y = index as i32 / width;
            let x = index as i32 % width;
            Vector2::new(x, y)
        };

        let color = {
            let color = colors[index];
            Vector4::new(color.x, color.y, color.z, 1.0)
        };

        let thickness = {
            let t = (importance[index] as f32).powf(THICKNESS_T_POW);
            let thickness_mean = THICKNESS_MAX_MEAN.lerp(THICKNESS_MIN_MEAN, t);
            let thickness_variance = THICKNESS_MAX_VARIANCE.lerp(THICKNESS_MIN_VARIANCE, t);
            let normal = Normal::new(thickness_mean, thickness_variance).unwrap();
            let thickness = normal.sample(&mut rng) * stroke_thickness;
            thickness.max(THICKNESS_MIN) as f32
        };

        let length = {
            let normal = Normal::new(8.0, 4.0).unwrap();
            let length = thickness * normal.sample(&mut rng);
            length.max(thickness)
        };

        // Hopping
        let hopping_point: Vec<_> = {
            let hop_length_normal = Normal::new(
                HOP_LENGTH_SCALING_FACTOR_MEAN,
                HOP_LENGTH_SCALING_FACTOR_VARIANCE,
            )
            .unwrap();
            let angle_normal = Normal::new(0.0, HOP_ANGLE_VARIANCE).unwrap();
            let hop_end_distance_normal =
                Normal::new(HOP_END_COLOR_DISTANCE_MEAN, HOP_END_COLOR_DISTANCE_VARIANCE).unwrap();

            let pos = Point2::new(pos.x as f32, pos.y as f32);
            let y = Vector2::<f32>::y();
            let mut s0 = vec![pos];
            let mut s1 = vec![pos];
            {
                let dir = directions[index];
                let angle0 = Rotation2::rotation_between(&y, &dir).angle();
                let angle1 = angle0 + 180.0_f32.to_radians();
                let angle0 = angle0
                    + (angle_normal.sample(&mut rng) / 2.0)
                        .min(HOP_ANGLE_MAX / 2.0)
                        .max(-HOP_ANGLE_MAX / 2.0);
                let angle1 = angle1
                    + (angle_normal.sample(&mut rng) / 2.0)
                        .min(HOP_ANGLE_MAX / 2.0)
                        .max(-HOP_ANGLE_MAX / 2.0);
                s0.push(
                    pos + (Rotation2::new(angle0) * y)
                        * hop_length_normal
                            .sample(&mut rng)
                            .max(HOP_LENGTH_MIN_SCALING_FACTOR),
                );
                s1.push(
                    pos + (Rotation2::new(angle1) * y)
                        * hop_length_normal
                            .sample(&mut rng)
                            .max(HOP_LENGTH_MIN_SCALING_FACTOR),
                );
            }

            fn calc_length(s0: &Vec<Point2<f32>>, s1: &Vec<Point2<f32>>) -> f32 {
                let length0: f32 = s0
                    .iter()
                    .zip(s0.iter().skip(1))
                    .map(|(a, b)| na::distance(a, b))
                    .sum();
                let length1: f32 = s1
                    .iter()
                    .zip(s1.iter().skip(1))
                    .map(|(a, b)| na::distance(a, b))
                    .sum();
                length0 + length1
            }

            let mut next_hop = |s: &mut Vec<Point2<f32>>| {
                let s_last = s[s.len() - 1].clone();
                let s_last_2 = s[s.len() - 2].clone();
                let index = s_last.y.round() as i32 * width + s_last.x.round() as i32;
                let dir = if 0 <= index && index < directions.len() as i32 {
                    let dir = directions[index as usize];
                    Vector2::new(dir.x, -dir.y).normalize()
                } else {
                    (s_last.coords - s_last_2.coords).normalize()
                };
                let angle = Rotation2::rotation_between(&y, &dir).angle();
                let angle_prev =
                    Rotation2::rotation_between(&y, &(s_last.coords - s_last_2.coords)).angle();
                let theta = angle - angle_prev;
                let theta = if theta <= -90.0_f32.to_radians() {
                    theta + 180.0_f32.to_radians()
                } else if theta >= 90.0_f32.to_radians() {
                    theta - 180.0_f32.to_radians()
                } else {
                    theta
                };
                let theta = (theta + angle_normal.sample(&mut rng))
                    .min(HOP_ANGLE_MAX)
                    .max(-HOP_ANGLE_MAX);

                let hop_length = hop_length_normal
                    .sample(&mut rng)
                    .max(HOP_LENGTH_MIN_SCALING_FACTOR)
                    * thickness;
                let hop_point = s_last + (Rotation2::new(angle_prev + theta) * y) * hop_length;

                let hop_index = (hop_point.coords.y.round() as i32 * width
                    + hop_point.coords.x.round() as i32) as usize;
                let hop_color = if hop_index < colors.len() - 1 {
                    let hop_color = colors[hop_index];
                    [
                        (hop_color.x * 255.0) as u8,
                        (hop_color.y * 255.0) as u8,
                        (hop_color.z * 255.0) as u8,
                    ]
                } else {
                    [
                        (color.x * 255.0) as u8,
                        (color.y * 255.0) as u8,
                        (color.z * 255.0) as u8,
                    ]
                };
                let stroke_color = [
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8,
                    (color.z * 255.0) as u8,
                ];

                if DE2000::from_rgb(&stroke_color, &hop_color)
                    > hop_end_distance_normal
                        .sample(&mut rng)
                        .max(HOP_END_COLOR_DISTANCE_MIN)
                {
                    true
                } else {
                    s.push(hop_point);
                    false
                }
            };

            while calc_length(&s0, &s1) < length {
                let is_end0 = next_hop(&mut s0);
                let is_end1 = next_hop(&mut s1);
                if is_end0 && is_end1 {
                    break;
                }
            }

            s1.into_iter().skip(1).rev().chain(s0).collect()
        };

        let importance = importance[index];

        Self {
            pos,
            color,
            hopping_point,
            thickness,
            importance,
        }
    }
}

#[derive(Clone)]
pub struct Individual {
    pub strokes: Vec<Stroke>,
}

impl Individual {
    pub fn new(
        colors: &Vec<Vector3<f32>>,
        directions: &Vec<Vector2<f32>>,
        importance: &Vec<f32>,
        width: i32,
        height: i32,
        stroke_num: u32,
        stroke_thickness: f32,
    ) -> Self {
        let mut strokes = vec![];

        let weighted_random_dist = WeightedIndex::new(importance).unwrap();
        let uniform_random_dist =
            WeightedIndex::new(importance.iter().map(|_| 1.0).collect::<Vec<_>>()).unwrap();
        let mut rng = thread_rng();

        let weighted_random_stroke_num = (stroke_num as f64 * 0.95) as i32;
        let uniform_random_stroke_num = stroke_num as i32 - weighted_random_stroke_num;
        for _ in 0..weighted_random_stroke_num {
            let index = weighted_random_dist.sample(&mut rng);
            let stroke = Stroke::new(
                index,
                &colors,
                &directions,
                &importance,
                width,
                height,
                stroke_thickness,
            );
            strokes.push(stroke);
        }
        for _ in 0..uniform_random_stroke_num {
            let index = uniform_random_dist.sample(&mut rng);
            let stroke = Stroke::new(
                index,
                &colors,
                &directions,
                &importance,
                width,
                height,
                stroke_thickness,
            );
            strokes.push(stroke);
        }

        strokes.sort_unstable_by(|a, b| a.importance.partial_cmp(&b.importance).unwrap());

        Self { strokes }
    }
}
