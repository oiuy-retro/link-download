#![windows_subsystem = "windows"]

mod classify;
mod classic;
mod commands;
mod model;
mod names;
mod net;
mod parse;
mod paths;
mod readout;
mod run;
mod store;
mod tools;
mod ui;

fn main() {
    std::process::exit(ui::run());
}
