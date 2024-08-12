#![windows_subsystem = "windows"]

use pcvisualizer::run;

fn main() {
    pollster::block_on(run());
}
