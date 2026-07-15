use std::io::Write;
use std::process::Command;

pub(super) fn run<W: Write>(stderr: &mut W) -> i32 {
    match Command::new("/usr/bin/open")
        .args(["-a", "Automic Vault", "--args", "--open-main-window"])
        .output()
    {
        Ok(output) if output.status.success() => 0,
        Ok(output) => {
            let _ = stderr.write_all(&output.stderr);
            output.status.code().unwrap_or(1)
        }
        Err(err) => {
            let _ = writeln!(stderr, "av open: {err}");
            1
        }
    }
}
