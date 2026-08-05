use std::process;

fn main() {
    process::exit(av::run_scanner_terminal(std::env::args_os()));
}
