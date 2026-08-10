mod policy;
mod transform;

fn main() {
    eprintln!("av-proxy-helper: must be launched by Automic Vault");
    std::process::exit(1);
}
