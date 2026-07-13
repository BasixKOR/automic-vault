use std::ffi::OsString;
use std::path::Path;
use std::sync::Mutex;

use crate::{AffectedFile, Finding};

const HIGH: &str = "high";
const SOLUTION: &str =
    "Review the reported plaintext secret and move or remove it; this radioisotope is detect-only.";

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn findings(
    name: &'static str,
    reasons: fn() -> Result<Vec<String>, String>,
    home: &Path,
) -> Vec<Finding> {
    let _lock = ENV_LOCK.lock().expect("radioisotope env lock poisoned");
    let _home = HomeEnv::set(home);
    let docs_url = docs_url(name);

    let Ok(reasons) = reasons() else {
        return Vec::new();
    };

    reasons
        .into_iter()
        .map(|reason| Finding {
            source: name,
            homepage: docs_url,
            severity: HIGH,
            affected: affected(&reason),
            explanation: reason,
            solution: SOLUTION.to_string(),
            docs_url,
        })
        .collect()
}

pub(crate) fn docs_url(name: &'static str) -> &'static str {
    match name {
        "acli" => "https://github.com/automic-vault/radioisotopes/tree/main/acli",
        "akamai" => "https://github.com/automic-vault/radioisotopes/tree/main/akamai",
        "algolia" => "https://github.com/automic-vault/radioisotopes/tree/main/algolia",
        "aliyun-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/aliyun-cli",
        "ansible" => "https://github.com/automic-vault/radioisotopes/tree/main/ansible",
        "argocd" => "https://github.com/automic-vault/radioisotopes/tree/main/argocd",
        "ast-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/ast-cli",
        "astra" => "https://github.com/automic-vault/radioisotopes/tree/main/astra",
        "atuin" => "https://github.com/automic-vault/radioisotopes/tree/main/atuin",
        "aws-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/aws-cli",
        "aws-cli-credentials-file" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/aws-cli-credentials-file"
        }
        "aws-cli-legacy-plugins" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/aws-cli-legacy-plugins"
        }
        "aws-cli-login-cache" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/aws-cli-login-cache"
        }
        "aws-sso-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/aws-sso-cli",
        "aws-vault" => "https://github.com/automic-vault/radioisotopes/tree/main/aws-vault",
        "azure-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/azure-cli",
        "bash" => "https://github.com/automic-vault/radioisotopes/tree/main/bash",
        "bitwarden-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/bitwarden-cli",
        "buf" => "https://github.com/automic-vault/radioisotopes/tree/main/buf",
        "bun" => "https://github.com/automic-vault/radioisotopes/tree/main/bun",
        "cariddi" => "https://github.com/automic-vault/radioisotopes/tree/main/cariddi",
        "cariddi-persisted-output" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/cariddi-persisted-output"
        }
        "cariddi-shell-history" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/cariddi-shell-history"
        }
        "censys" => "https://github.com/automic-vault/radioisotopes/tree/main/censys",
        "certbot" => "https://github.com/automic-vault/radioisotopes/tree/main/certbot",
        "checkov" => "https://github.com/automic-vault/radioisotopes/tree/main/checkov",
        "circleci" => "https://github.com/automic-vault/radioisotopes/tree/main/circleci",
        "civo" => "https://github.com/automic-vault/radioisotopes/tree/main/civo",
        "cloudflare-wrangler" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/cloudflare-wrangler"
        }
        "cloudflared" => "https://github.com/automic-vault/radioisotopes/tree/main/cloudflared",
        "cloudsmith-cli" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/cloudsmith-cli"
        }
        "composer" => "https://github.com/automic-vault/radioisotopes/tree/main/composer",
        "curl" => "https://github.com/automic-vault/radioisotopes/tree/main/curl",
        "databricks" => "https://github.com/automic-vault/radioisotopes/tree/main/databricks",
        "dcos-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/dcos-cli",
        "docker" => "https://github.com/automic-vault/radioisotopes/tree/main/docker",
        "docker-credential-helpers" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/docker-credential-helpers"
        }
        "docker-registry-credentials" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/docker-registry-credentials"
        }
        "docker-root-access" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/docker-root-access"
        }
        "docker-credential-helper" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/docker-credential-helper"
        }
        "docker-machine" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/docker-machine"
        }
        "doctl" => "https://github.com/automic-vault/radioisotopes/tree/main/doctl",
        "dropbox-uploader" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/dropbox-uploader"
        }
        "envchain" => "https://github.com/automic-vault/radioisotopes/tree/main/envchain",
        "fastlane" => "https://github.com/automic-vault/radioisotopes/tree/main/fastlane",
        "fastly" => "https://github.com/automic-vault/radioisotopes/tree/main/fastly",
        "fauna-shell" => "https://github.com/automic-vault/radioisotopes/tree/main/fauna-shell",
        "firebase-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/firebase-cli",
        "flyctl" => "https://github.com/automic-vault/radioisotopes/tree/main/flyctl",
        "gallery-dl" => "https://github.com/automic-vault/radioisotopes/tree/main/gallery-dl",
        "gcli" => "https://github.com/automic-vault/radioisotopes/tree/main/gcli",
        "gh-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/gh-cli",
        "gh-cli-hosts-token" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/gh-cli-hosts-token"
        }
        "gh-cli-keychain-access" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/gh-cli-keychain-access"
        }
        "git-credential-oauth" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/git-credential-oauth"
        }
        "glab" => "https://github.com/automic-vault/radioisotopes/tree/main/glab",
        "goat" => "https://github.com/automic-vault/radioisotopes/tree/main/goat",
        "gotify" => "https://github.com/automic-vault/radioisotopes/tree/main/gotify",
        "gptcommit" => "https://github.com/automic-vault/radioisotopes/tree/main/gptcommit",
        "grafanactl" => "https://github.com/automic-vault/radioisotopes/tree/main/grafanactl",
        "graphite" => "https://github.com/automic-vault/radioisotopes/tree/main/graphite",
        "hcloud" => "https://github.com/automic-vault/radioisotopes/tree/main/hcloud",
        "helm" => "https://github.com/automic-vault/radioisotopes/tree/main/helm",
        "heroku" => "https://github.com/automic-vault/radioisotopes/tree/main/heroku",
        "httpie" => "https://github.com/automic-vault/radioisotopes/tree/main/httpie",
        "huggingface-cli" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/huggingface-cli"
        }
        "imap-backup" => "https://github.com/automic-vault/radioisotopes/tree/main/imap-backup",
        "jfrog-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/jfrog-cli",
        "k6" => "https://github.com/automic-vault/radioisotopes/tree/main/k6",
        "kubernetes-cli" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/kubernetes-cli"
        }
        "luarocks" => "https://github.com/automic-vault/radioisotopes/tree/main/luarocks",
        "maestro" => "https://github.com/automic-vault/radioisotopes/tree/main/maestro",
        "mariadb" => "https://github.com/automic-vault/radioisotopes/tree/main/mariadb",
        "maven" => "https://github.com/automic-vault/radioisotopes/tree/main/maven",
        "mcp-remote" => "https://github.com/automic-vault/radioisotopes/tree/main/mcp-remote",
        "mercurial" => "https://github.com/automic-vault/radioisotopes/tree/main/mercurial",
        "midnight-commander" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/midnight-commander"
        }
        "minio-mc" => "https://github.com/automic-vault/radioisotopes/tree/main/minio-mc",
        "mkcert" => "https://github.com/automic-vault/radioisotopes/tree/main/mkcert",
        "mongodb-atlas-cli" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/mongodb-atlas-cli"
        }
        "mycli" => "https://github.com/automic-vault/radioisotopes/tree/main/mycli",
        "mysql" => "https://github.com/automic-vault/radioisotopes/tree/main/mysql",
        "mysql-client" => "https://github.com/automic-vault/radioisotopes/tree/main/mysql-client",
        "mysql@8.0" => "https://github.com/automic-vault/radioisotopes/tree/main/mysql@8.0",
        "mysql@8.4" => "https://github.com/automic-vault/radioisotopes/tree/main/mysql@8.4",
        "netlify-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/netlify-cli",
        "node" => "https://github.com/automic-vault/radioisotopes/tree/main/node",
        "node@18" => "https://github.com/automic-vault/radioisotopes/tree/main/node@18",
        "npm" => "https://github.com/automic-vault/radioisotopes/tree/main/npm",
        "nuget" => "https://github.com/automic-vault/radioisotopes/tree/main/nuget",
        "oauth2l" => "https://github.com/automic-vault/radioisotopes/tree/main/oauth2l",
        "oci-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/oci-cli",
        "opencode" => "https://github.com/automic-vault/radioisotopes/tree/main/opencode",
        "openhue-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/openhue-cli",
        "openssh" => "https://github.com/automic-vault/radioisotopes/tree/main/openssh",
        "openssl@3" => "https://github.com/automic-vault/radioisotopes/tree/main/openssl@3",
        "openstackclient" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/openstackclient"
        }
        "opentofu" => "https://github.com/automic-vault/radioisotopes/tree/main/opentofu",
        "openvpn" => "https://github.com/automic-vault/radioisotopes/tree/main/openvpn",
        "ordercli" => "https://github.com/automic-vault/radioisotopes/tree/main/ordercli",
        "ossutil" => "https://github.com/automic-vault/radioisotopes/tree/main/ossutil",
        "oxide-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/oxide-cli",
        "perl" => "https://github.com/automic-vault/radioisotopes/tree/main/perl",
        "phylum-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/phylum-cli",
        "pianobar" => "https://github.com/automic-vault/radioisotopes/tree/main/pianobar",
        "plumber" => "https://github.com/automic-vault/radioisotopes/tree/main/plumber",
        "pnpm" => "https://github.com/automic-vault/radioisotopes/tree/main/pnpm",
        "pnpm-auth-token" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/pnpm-auth-token"
        }
        "pnpm-minimum-release-age" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/pnpm-minimum-release-age"
        }
        "podman" => "https://github.com/automic-vault/radioisotopes/tree/main/podman",
        "poetry" => "https://github.com/automic-vault/radioisotopes/tree/main/poetry",
        "pulumi" => "https://github.com/automic-vault/radioisotopes/tree/main/pulumi",
        "qwen-code" => "https://github.com/automic-vault/radioisotopes/tree/main/qwen-code",
        "railway" => "https://github.com/automic-vault/radioisotopes/tree/main/railway",
        "rclone" => "https://github.com/automic-vault/radioisotopes/tree/main/rclone",
        "rsync" => "https://github.com/automic-vault/radioisotopes/tree/main/rsync",
        "ruby" => "https://github.com/automic-vault/radioisotopes/tree/main/ruby",
        "runpodctl" => "https://github.com/automic-vault/radioisotopes/tree/main/runpodctl",
        "rust" => "https://github.com/automic-vault/radioisotopes/tree/main/rust",
        "s3cmd" => "https://github.com/automic-vault/radioisotopes/tree/main/s3cmd",
        "sbt" => "https://github.com/automic-vault/radioisotopes/tree/main/sbt",
        "secretlint" => "https://github.com/automic-vault/radioisotopes/tree/main/secretlint",
        "secretlint-persisted-report" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/secretlint-persisted-report"
        }
        "secretlint-shell-history" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/secretlint-shell-history"
        }
        "sentry-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/sentry-cli",
        "shodan" => "https://github.com/automic-vault/radioisotopes/tree/main/shodan",
        "skopeo" => "https://github.com/automic-vault/radioisotopes/tree/main/skopeo",
        "snowflake-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/snowflake-cli",
        "snyk" => "https://github.com/automic-vault/radioisotopes/tree/main/snyk",
        "soracom-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/soracom-cli",
        "sqlcmd" => "https://github.com/automic-vault/radioisotopes/tree/main/sqlcmd",
        "sshpass" => "https://github.com/automic-vault/radioisotopes/tree/main/sshpass",
        "sslmate" => "https://github.com/automic-vault/radioisotopes/tree/main/sslmate",
        "stripe-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/stripe-cli",
        "supabase" => "https://github.com/automic-vault/radioisotopes/tree/main/supabase",
        "tailscale" => "https://github.com/automic-vault/radioisotopes/tree/main/tailscale",
        "talosctl" => "https://github.com/automic-vault/radioisotopes/tree/main/talosctl",
        "terraform" => "https://github.com/automic-vault/radioisotopes/tree/main/terraform",
        "terraform-core" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/terraform-core"
        }
        "todoist-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/todoist-cli",
        "transifex-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/transifex-cli",
        "travis" => "https://github.com/automic-vault/radioisotopes/tree/main/travis",
        "twine" => "https://github.com/automic-vault/radioisotopes/tree/main/twine",
        "uaa-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/uaa-cli",
        "uv" => "https://github.com/automic-vault/radioisotopes/tree/main/uv",
        "vagrant" => "https://github.com/automic-vault/radioisotopes/tree/main/vagrant",
        "vault" => "https://github.com/automic-vault/radioisotopes/tree/main/vault",
        "vercel-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/vercel-cli",
        "virustotal-cli" => {
            "https://github.com/automic-vault/radioisotopes/tree/main/virustotal-cli"
        }
        "vultr" => "https://github.com/automic-vault/radioisotopes/tree/main/vultr",
        "wakatime-cli" => "https://github.com/automic-vault/radioisotopes/tree/main/wakatime-cli",
        "wget" => "https://github.com/automic-vault/radioisotopes/tree/main/wget",
        "wget2" => "https://github.com/automic-vault/radioisotopes/tree/main/wget2",
        "wsk" => "https://github.com/automic-vault/radioisotopes/tree/main/wsk",
        "yt-dlp" => "https://github.com/automic-vault/radioisotopes/tree/main/yt-dlp",
        "yarn" => "https://github.com/automic-vault/radioisotopes/tree/main/yarn",
        "zsh" => "https://github.com/automic-vault/radioisotopes/tree/main/zsh",
        _ => "https://github.com/automic-vault/radioisotopes",
    }
}

fn affected(reason: &str) -> Vec<AffectedFile> {
    reason
        .rsplit_once(": ")
        .map(|(_, path)| path.trim())
        .filter(|path| path.starts_with('/') || path.starts_with('~'))
        .map(|path| {
            vec![AffectedFile {
                path: path.to_string(),
                line: None,
            }]
        })
        .unwrap_or_default()
}

struct HomeEnv {
    previous: Option<OsString>,
}

impl HomeEnv {
    fn set(home: &Path) -> Self {
        let previous = std::env::var_os("HOME");
        // SAFETY: imported radioisotope detectors read HOME from process env;
        // ENV_LOCK serializes scan-time HOME changes and Drop restores it.
        unsafe { std::env::set_var("HOME", home) };
        Self { previous }
    }
}

impl Drop for HomeEnv {
    fn drop(&mut self) {
        // SAFETY: still protected by ENV_LOCK while restoring HOME.
        unsafe {
            match &self.previous {
                Some(previous) => std::env::set_var("HOME", previous),
                None => std::env::remove_var("HOME"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_reason_paths_to_affected_file() {
        assert_eq!(
            affected("Stripe CLI config contains plaintext API keys: /tmp/config.toml"),
            vec![AffectedFile {
                path: "/tmp/config.toml".to_string(),
                line: None,
            }]
        );
    }
}
