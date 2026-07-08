use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::process::Command;

const MARKER: &str = "AUTOMIC_VAULT_BREW_STUB_V1";
const TARGET: &str = "/opt/homebrew/bin/brew";

fn main() {
    if std::env::args().any(|arg| arg == "--automic-vault-brew-stub-marker") {
        println!("{MARKER}");
        return;
    }

    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let err = Command::new(TARGET)
        .args(args)
        .env_clear()
        .envs(stub_env(std::env::vars_os()))
        .exec();
    eprintln!("av-brew-stub: failed to exec {TARGET}: {err}");
    std::process::exit(127);
}

fn stub_env<I>(source: I) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    let mut env = vec![
        ("HOME".into(), "/opt/homebrew/var/automic".into()),
        ("USER".into(), "automic".into()),
        ("LOGNAME".into(), "automic".into()),
        (
            "PATH".into(),
            "/opt/homebrew/bin:/opt/homebrew/sbin:/usr/bin:/bin:/usr/sbin:/sbin".into(),
        ),
        ("TMPDIR".into(), "/opt/homebrew/var/automic/tmp".into()),
        (
            "HOMEBREW_CACHE".into(),
            "/opt/homebrew/var/automic/cache".into(),
        ),
        ("AUTOMIC_VAULT_BREW_STUB".into(), MARKER.into()),
    ];

    for (key, value) in source {
        let Some(key_str) = key.to_str() else {
            continue;
        };
        if key_str == "TERM"
            || key_str == "LANG"
            || key_str == "NO_COLOR"
            || key_str.starts_with("LC_")
        {
            env.push((key, value));
        }
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_env_keeps_only_safe_user_env() {
        let env = stub_env([
            ("TERM".into(), "xterm-256color".into()),
            ("LANG".into(), "en_US.UTF-8".into()),
            ("LC_ALL".into(), "C".into()),
            ("NO_COLOR".into(), "1".into()),
            ("HOMEBREW_PREFIX".into(), "/tmp/bad".into()),
            ("PATH".into(), "/tmp/bad".into()),
        ]);

        assert!(env.contains(&("HOME".into(), "/opt/homebrew/var/automic".into())));
        assert!(env.contains(&("USER".into(), "automic".into())));
        assert!(env.contains(&("LOGNAME".into(), "automic".into())));
        assert!(env.contains(&("TERM".into(), "xterm-256color".into())));
        assert!(env.contains(&("LC_ALL".into(), "C".into())));
        assert!(!env.contains(&("HOMEBREW_PREFIX".into(), "/tmp/bad".into())));
        assert!(!env.contains(&("PATH".into(), "/tmp/bad".into())));
    }
}
