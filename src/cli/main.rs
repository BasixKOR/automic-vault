use std::process;

fn main() {
    process::exit(av::run_terminal(std::env::args_os()));
}
