#![windows_subsystem = "windows"]

mod classify;
mod conf;
mod folders;
mod classic;
mod commands;
mod model;
mod jobs;
mod partial;
mod names;
mod net;
mod parse;
mod paths;
mod popup;
mod readout;
mod run;
mod store;
mod tools;
mod ui;

fn main() {
    std::process::exit(ui::run());
}
