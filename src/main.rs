use std::process;

fn main() {
    process::exit(av::run(
        std::env::args_os(),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    ));
}
