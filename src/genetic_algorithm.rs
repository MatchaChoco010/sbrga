use std::path::Path;

use anyhow::Result;
use chrono::Local;
use image::{self, GenericImageView};
use na::{Vector2, Vector3};
use nalgebra as na;
use rand::distributions::WeightedIndex;
use rand::prelude::*;

use crate::individual::Individual;
use crate::renderer;
use crate::resources::Resources;

pub fn genetic_algorithm(
    color_map: &str,
    dir_map: &str,
    importance_map: &str,
    output_path: &str,
    stroke_num: u32,
    stroke_thickness: f32,
    population_size: u32,
    generation: usize,
    save_generation: Vec<usize>,
    save_generation_step: usize,
) -> Result<()> {
    const PROBABILITY_OF_MUTATION: f64 = 0.01;
    const PROBABILITY_BIAS: f32 = 10.0;

    println!("[{}] Start GA...", Local::now());

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

    let window_height = 1080_u32;
    let window_width = (window_height as f64 * aspect) as u32;

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

    let save_generation = save_generation
        .into_iter()
        .chain(
            (0..)
                .map(|i| i * save_generation_step as usize)
                .take_while(|&x| x <= generation as usize),
        )
        .collect::<Vec<_>>();

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
        .window("Create an individual painting", window_width, window_height)
        .opengl()
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let _gl_context = window.gl_create_context().unwrap();
    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    let mut event_pump = sdl.event_pump().unwrap();

    let mut renderer = renderer::Renderer::new(width, height, &res)?;
    renderer.update_viewport_size(window_width as i32, window_height as i32);

    println!("[{}] Generate initial population...", Local::now());

    let mut population = (0..population_size)
        .map(|_| {
            Individual::new(
                &colors,
                &directions,
                &importance,
                width,
                height,
                stroke_num,
                stroke_thickness,
            )
        })
        .collect::<Vec<_>>();

    let mut rng = thread_rng();
    let dist05 = WeightedIndex::new(vec![1.0, 1.0]).unwrap();
    let dist_mutation =
        WeightedIndex::new(vec![1.0 - PROBABILITY_OF_MUTATION, PROBABILITY_OF_MUTATION]).unwrap();

    for gen in 1..=generation {
        println!("[{}] generation {:>5}", Local::now(), { gen });
        let mut population_scores = population
            .iter()
            .map(|i| (i, renderer.score(i, &colors, &importance)))
            .collect::<Vec<_>>();
        population_scores
            .sort_unstable_by(|(_, score_a), (_, score_b)| score_a.partial_cmp(score_b).unwrap());

        let (top_individual, top_score) = population_scores[0];

        println!("[{}] top score: {}", Local::now(), top_score);
        for _ in event_pump.poll_iter() {}
        renderer.show(&top_individual);
        window.gl_swap_window();

        // 保存指定されていたジェネレーションならば保存する。
        if save_generation.contains(&gen) {
            let output_path = output_path.to_string() + ".gen-" + &gen.to_string() + ".png";
            renderer.render_to_file(&top_individual, &output_path)?;
            println!("[{}] save file: {}", Local::now(), output_path);
        }

        println!("[{}] create populations...", Local::now());

        population = population_scores
            .into_iter()
            .map(|(i, _)| i.clone())
            .collect();
        let mut population_clone = population.clone();

        let top_index = (population.len() as f64 * 0.1) as usize;
        let bottom_index = (population.len() as f64 * 0.9) as usize;
        let _ = population.split_off(top_index);
        let _ = population_clone.split_off(bottom_index);

        let dist = WeightedIndex::new(
            (1..=population_clone.len()).map(|i| 1.0 / (i as f32 + PROBABILITY_BIAS)),
        )
        .unwrap();

        while population.len() != population_size as usize {
            let mut i0 = population_clone[dist.sample(&mut rng)].clone();
            let mut i1 = population_clone[dist.sample(&mut rng)].clone();
            let i_other = Individual::new(
                &colors,
                &directions,
                &importance,
                width,
                height,
                stroke_num,
                stroke_thickness,
            );

            for i in 0..(i0.strokes.len()) {
                let flag = dist05.sample(&mut rng);
                if flag == 1 {
                    let s0 = i0.strokes[i].clone();
                    let s1 = i1.strokes[i].clone();
                    i0.strokes[i] = s1;
                    i1.strokes[i] = s0;
                }
            }

            for i in 0..(i0.strokes.len()) {
                let flag = dist_mutation.sample(&mut rng);
                if flag == 1 {
                    let s_other = i_other.strokes[i].clone();
                    i0.strokes[i] = s_other;
                }
            }
            for i in 0..(i1.strokes.len()) {
                let flag = dist_mutation.sample(&mut rng);
                if flag == 1 {
                    let s_other = i_other.strokes[i].clone();
                    i1.strokes[i] = s_other;
                }
            }

            population.push(i0);
            population.push(i1);

            if population.len() >= population_size as usize {
                population.pop();
            }
        }
    }

    println!("[{}] final generation", Local::now());
    let mut population_scores = population
        .iter()
        .map(|i| (i, renderer.score(i, &colors, &importance)))
        .collect::<Vec<_>>();
    population_scores
        .sort_unstable_by(|(_, score_a), (_, score_b)| score_a.partial_cmp(score_b).unwrap());

    let (top_individual, top_score) = population_scores[0];
    println!("[{}] final score: {}", Local::now(), top_score);

    renderer.render_to_file(&top_individual, output_path)?;

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

        renderer.show(&top_individual);
        window.gl_swap_window();
    }

    Ok(())
}
