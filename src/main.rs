#![windows_subsystem = "windows"]

use std::{env, path::PathBuf};

use pcvisualizer::run;

fn main() {
    let args: Vec<String> = env::args().collect();

    let pcd_path = if args.len() > 1 {
        Some(PathBuf::from(args[1].clone()))
    } else {
        None
    };

    run(pcd_path);
}
