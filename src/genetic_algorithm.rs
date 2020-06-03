use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

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
    save_width: i32,
    save_height: i32,
) -> Result<()> {
    const PROBABILITY_OF_MUTATION: f64 = 0.35;
    const PROBABILITY_CROSSOVER_BIAS: f64 = 50.0;
    // const DISTANCE_RATE: f64 = 0.5;

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

    let mut renderer = renderer::Renderer::new(width, height, save_width, save_height, &res)?;
    renderer.update_viewport_size(window_width as i32, window_height as i32);

    println!("[{}] Generate initial population...", Local::now());

    let mut population = (0..population_size)
        .map(|_| {
            Rc::new(RefCell::new(Individual::new(
                &colors,
                &directions,
                &importance,
                width,
                height,
                stroke_num,
                stroke_thickness,
            )))
        })
        .collect::<Vec<_>>();

    println!("[{}] calc initial population scores", Local::now());
    let mut population_scores = population
        .iter()
        .cloned()
        .map(|i| (i.clone(), renderer.score(&i.borrow(), &colors, &importance)))
        .collect::<Vec<_>>();
    population_scores
        .sort_unstable_by(|(_, score_a), (_, score_b)| score_a.partial_cmp(score_b).unwrap());

    let (top_individual, top_score) = population_scores[0].clone();

    // let mut new_population: Vec<Individual> = vec![];

    println!("[{}] top score: {:.10}", Local::now(), top_score);
    for _ in event_pump.poll_iter() {}
    renderer.show(&top_individual.borrow());
    window.gl_swap_window();

    let mut rng = thread_rng();

    let mut crossover_pos = (0..(top_individual.borrow().strokes.len() / 2))
        .map(|_| true)
        .chain(
            (0..(top_individual.borrow().strokes.len()
                - top_individual.borrow().strokes.len() / 2))
                .map(|_| false),
        )
        .collect::<Vec<_>>();
    // let select_from_all_dist = WeightedIndex::new((0..population_size).map(|_| 1.0)).unwrap();
    // let select_from_distance_rate_dist =
    //     WeightedIndex::new((0..((population_size as f64 * DISTANCE_RATE) as usize)).map(|_| 1.0))
    //         .unwrap();
    let select_from_all_dist = WeightedIndex::new(
        (0..population_size).map(|i| 1.0 / (i as f64 + PROBABILITY_CROSSOVER_BIAS)),
    )
    .unwrap();

    // let dist05 = WeightedIndex::new(vec![1.0, 1.0]).unwrap();
    let dist_mutation =
        WeightedIndex::new(vec![1.0 - PROBABILITY_OF_MUTATION, PROBABILITY_OF_MUTATION]).unwrap();

    // let mut d = (top_individual.borrow().strokes.len() / 4) as i32;
    let mut d = 50;
    let mut last_top_individual = top_individual;

    for gen in 1..=generation {
        println!("[{}] generation {:>5}", Local::now(), { gen });

        println!("[{}] generate new population", Local::now());
        // generate new population
        let mut new_population = vec![];
        while new_population.len() < population_size as usize {
            let p0 = population[select_from_all_dist.sample(&mut rng)].clone();
            // let mut population_clone = population.clone();
            // population_clone.sort_unstable_by_key(|i| i.borrow().distance(&p0.borrow()));
            // let p1 = population_clone[select_from_distance_rate_dist.sample(&mut rng)].clone();
            let p1 = population[select_from_all_dist.sample(&mut rng)].clone();
            crossover_pos.shuffle(&mut rng);
            for (i, &flag) in crossover_pos.iter().enumerate() {
                if flag {
                    let s0 = p0.borrow().strokes[i].clone();
                    let s1 = p1.borrow().strokes[i].clone();
                    p0.borrow_mut().strokes[i] = s1;
                    p1.borrow_mut().strokes[i] = s0;
                }
            }
            new_population.push(p0);
            if new_population.len() < population.len() {
                new_population.push(p1);
            }
        }

        println!("[{}] calc new generation scores", Local::now());
        let mut new_population_scores = new_population
            .iter()
            .cloned()
            .map(|i| (i.clone(), renderer.score(&i.borrow(), &colors, &importance)))
            .collect::<Vec<_>>();

        // println!("[{}] append generation and new generation", Local::now());
        // selecting from two generation
        population_scores.append(&mut new_population_scores);

        // println!("[{}] sort generation and new generation", Local::now());
        population_scores.sort_by(|(_, s0), (_, s1)| s0.partial_cmp(s1).unwrap());

        // println!("[{}] split off generation and new generation", Local::now());
        let _ = population_scores.split_off(population_size as usize);

        // println!("[{}] get population", Local::now());
        population = population_scores
            .iter()
            .cloned()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        // println!("[{}] show top individual", Local::now());
        // show top individual
        let (top_individual, top_score) = population_scores[0].clone();
        println!("[{}] top score: {:.10}", Local::now(), top_score);
        for _ in event_pump.poll_iter() {}
        renderer.show(&top_individual.borrow());
        window.gl_swap_window();

        // 保存指定されていたジェネレーションならば保存する。
        if save_generation.contains(&gen) {
            println!("[{}] save top individual render image", Local::now());
            let output_path = output_path.to_string() + ".gen-" + &gen.to_string() + ".png";
            renderer.render_to_file(&top_individual.borrow(), &output_path)?;
            println!("[{}] save file: {}", Local::now(), output_path);
        }

        // println!("[{}] crossover", Local::now());
        if top_individual
            .borrow()
            .distance(&last_top_individual.borrow())
            == 0
        {
            d -= 1;

            if d < 0 {
                println!("[{}] mutation...", Local::now());
                let _ = population_scores.split_off(1);
                while population.len() < population_size as usize {
                    let i_other = Individual::new(
                        &colors,
                        &directions,
                        &importance,
                        width,
                        height,
                        stroke_num,
                        stroke_thickness,
                    );
                    let i = top_individual.clone();
                    for index in 0..i.borrow().strokes.len() {
                        if dist_mutation.sample(&mut rng) == 1 {
                            i.borrow_mut().strokes[index] = i_other.strokes[index].clone();
                        }
                    }
                    population_scores
                        .push((i.clone(), renderer.score(&i.borrow(), &colors, &importance)));
                }

                // d = (top_individual.borrow().strokes.len() as f32 * 0.35 * (1.0 - 0.35)) as i32;
                d = 25;
            }
        }

        last_top_individual = top_individual;
    }

    println!("[{}] final generation", Local::now());
    let mut population_scores = population
        .iter()
        .map(|i| (i, renderer.score(&i.borrow(), &colors, &importance)))
        .collect::<Vec<_>>();
    population_scores
        .sort_unstable_by(|(_, score_a), (_, score_b)| score_a.partial_cmp(score_b).unwrap());

    let (top_individual, top_score) = population_scores[0];
    println!("[{}] final score: {}", Local::now(), top_score);

    renderer.render_to_file(&top_individual.borrow(), output_path)?;

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

        renderer.show(&top_individual.borrow());
        window.gl_swap_window();
    }

    Ok(())
}
