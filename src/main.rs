use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

pub mod render_gl;
pub mod resources;

mod create_direction_map;
mod create_individual;
mod genetic_algorithm;
mod individual;
mod renderer;
mod triangle;
mod visualize_direction_map;

use create_direction_map::{create_direction_map_from_edge, create_direction_map_from_normal};
use create_individual::create_individual;
use genetic_algorithm::genetic_algorithm;
use visualize_direction_map::visualize_direction_map;
#[derive(StructOpt, Debug)]
#[structopt(name = "sbrga", about = "A stroke based rendering tool set.")]
enum Sbrga {
    #[structopt(about = "create direction map from normal map")]
    CreateDirmapFromNormal {
        #[structopt(parse(from_os_str), about = "input normal map path")]
        input: PathBuf,
        #[structopt(parse(from_os_str), short, long, about = "output file path")]
        output: Option<PathBuf>,
    },
    #[structopt(about = "create direction map from edge map")]
    CreateDirmapFromEdge {
        #[structopt(parse(from_os_str), about = "input edge map path")]
        input: PathBuf,
        #[structopt(parse(from_os_str), short, long, about = "output file path")]
        output: Option<PathBuf>,
    },
    #[structopt(about = "visualize direction map")]
    VisualizeDirmap {
        #[structopt(parse(from_os_str), about = "input direction map path")]
        input: PathBuf,
    },
    #[structopt(about = "create an individual painting")]
    CreateIndividual {
        #[structopt(parse(from_os_str), short, long, about = "input color map")]
        color_map: PathBuf,
        #[structopt(parse(from_os_str), short, long, about = "input direction map")]
        dir_map: PathBuf,
        #[structopt(parse(from_os_str), short, long, about = "input importance map")]
        importance_map: PathBuf,
        #[structopt(parse(from_os_str), short, long, about = "output path")]
        output_path: PathBuf,
        #[structopt(default_value = "10000", short, long, about = "number of strokes")]
        stroke_num: u32,
        #[structopt(default_value = "1.0", long, about = "stroke thickness scale")]
        stroke_thickness: f32,
    },
    #[structopt(about = "genetic algorithm process")]
    GA {
        #[structopt(parse(from_os_str), short, long, about = "input color map")]
        color_map: PathBuf,
        #[structopt(parse(from_os_str), short, long, about = "input direction map")]
        dir_map: PathBuf,
        #[structopt(parse(from_os_str), short, long, about = "input importance map")]
        importance_map: PathBuf,
        #[structopt(parse(from_os_str), short, long, about = "output path")]
        output_path: PathBuf,
        #[structopt(default_value = "10000", long, about = "number of strokes")]
        stroke_num: u32,
        #[structopt(default_value = "1.0", long, about = "stroke thickness scale")]
        stroke_thickness: f32,
        #[structopt(default_value = "250", short, long, about = "population size")]
        population_size: u32,
        #[structopt(default_value = "100", short, long, about = "generation number")]
        generation: usize,
        #[structopt(short, long, about = "save generation")]
        save_generation: Vec<usize>,
        #[structopt(long, about = "save generation step")]
        save_generation_step: usize,
        #[structopt(long, about = "save file width")]
        width: i32,
        #[structopt(long, about = "save file height")]
        height: i32,
        #[structopt(default_value = "50", long, about = "D value")]
        d_value: i32,
    },
}

fn main() -> Result<()> {
    let opt = Sbrga::from_args();

    match opt {
        Sbrga::CreateDirmapFromNormal { input, output } => {
            println!("Normal map file: {}", input.to_str().unwrap());
            let output = if let Some(o) = output {
                o
            } else {
                if let Some(ext) = input.extension() {
                    input.with_extension("dir.".to_string() + ext.to_str().unwrap())
                } else {
                    input.with_extension("dir")
                }
            };
            println!(">> Output file: {}", output.to_str().unwrap());
            create_direction_map_from_normal(input.to_str().unwrap(), output.to_str().unwrap());
        }
        Sbrga::CreateDirmapFromEdge { input, output } => {
            println!("Edge map file: {}", input.to_str().unwrap());
            let output = if let Some(o) = output {
                o
            } else {
                if let Some(ext) = input.extension() {
                    input.with_extension("dir.".to_string() + ext.to_str().unwrap())
                } else {
                    input.with_extension("dir")
                }
            };
            println!(">> Output file: {}", output.to_str().unwrap());
            create_direction_map_from_edge(input.to_str().unwrap(), output.to_str().unwrap());
        }
        Sbrga::VisualizeDirmap { input } => {
            visualize_direction_map(input.to_str().unwrap(), 100, 100)?;
        }
        Sbrga::CreateIndividual {
            color_map,
            dir_map,
            importance_map,
            output_path,
            stroke_num,
            stroke_thickness,
        } => {
            create_individual(
                color_map.to_str().unwrap(),
                dir_map.to_str().unwrap(),
                importance_map.to_str().unwrap(),
                output_path.to_str().unwrap(),
                stroke_num,
                stroke_thickness,
            )?;
        }
        Sbrga::GA {
            color_map,
            dir_map,
            importance_map,
            output_path,
            stroke_num,
            stroke_thickness,
            population_size,
            generation,
            save_generation,
            save_generation_step,
            width,
            height,
            d_value,
        } => genetic_algorithm(
            color_map.to_str().unwrap(),
            dir_map.to_str().unwrap(),
            importance_map.to_str().unwrap(),
            output_path.to_str().unwrap(),
            stroke_num,
            stroke_thickness,
            population_size,
            generation,
            save_generation,
            save_generation_step,
            width,
            height,
            d_value,
        )?,
    }

    Ok(())
}
