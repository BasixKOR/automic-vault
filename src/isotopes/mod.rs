use std::path::Path;

use crate::Finding;

mod acli;
mod akamai;
mod algolia;
mod aliyun_cli;
mod ansible;
mod argocd;
mod ast_cli;
mod astra;
mod atuin;
pub(crate) mod aws;
mod aws_cli;
mod aws_sso_cli;
mod aws_vault;
mod azure_cli;
mod bash;
mod bitwarden_cli;
mod buf;
mod cariddi;
mod censys;
mod certbot;
mod checkov;
mod circleci;
mod civo;
mod cloudflare_wrangler;
mod cloudflared;
mod cloudsmith_cli;
mod composer;
mod curl;
mod databricks;
mod dcos_cli;
mod docker;
mod docker_credential_helper;
mod docker_machine;
mod doctl;
mod dropbox_uploader;
mod envchain;
mod fastlane;
mod fastly;
mod fauna_shell;
mod firebase_cli;
mod flyctl;
mod gallery_dl;
mod gcli;
pub(crate) mod git;
mod git_credential_oauth;
mod glab;
mod goat;
mod gotify;
mod gptcommit;
mod grafanactl;
mod graphite;
mod hcloud;
mod helm;
mod heroku;
mod httpie;
mod huggingface_cli;
mod imap_backup;
mod jfrog_cli;
mod k6;
mod kubernetes_cli;
mod luarocks;
mod maestro;
mod mariadb;
mod maven;
mod mcp_remote;
mod mercurial;
mod midnight_commander;
mod minio_mc;
mod mkcert;
mod mongodb_atlas_cli;
mod mycli;
mod mysql;
mod mysql_8_0;
mod mysql_8_4;
mod mysql_client;
mod netlify_cli;
mod node;
mod node_18;
mod nuget;
mod oauth2l;
mod oci_cli;
mod opencode;
mod openhue_cli;
mod openssh;
mod openssl_3;
mod openstackclient;
mod opentofu;
mod openvpn;
mod ordercli;
mod ossutil;
mod oxide_cli;
mod perl;
mod phylum_cli;
mod pianobar;
mod plumber;
mod pnpm;
mod podman;
mod poetry;
mod pulumi;
mod qwen_code;
mod radioisotope;
mod railway;
mod rclone;
mod rsync;
mod ruby;
mod runpodctl;
mod rust;
mod s3cmd;
mod sbt;
mod secretlint;
mod sentry_cli;
mod shodan;
mod skopeo;
mod snowflake_cli;
mod snyk;
mod soracom_cli;
mod sqlcmd;
mod sshpass;
mod sslmate;
mod stripe_cli;
mod tailscale;
mod talosctl;
mod terraform;
mod terraform_core;
mod todoist_cli;
mod transifex_cli;
mod travis;
mod twine;
mod uaa_cli;
mod uv;
mod vagrant;
mod vault;
mod vercel_cli;
mod virustotal_cli;
mod vultr;
mod wakatime_cli;
mod wget;
mod wget2;
mod wsk;
mod yt_dlp;
mod zsh;

const DETECTORS: &[fn(&Path) -> Vec<Finding>] = &[
    git::findings,
    aws::findings,
    acli::findings,
    akamai::findings,
    algolia::findings,
    aliyun_cli::findings,
    ansible::findings,
    argocd::findings,
    ast_cli::findings,
    astra::findings,
    atuin::findings,
    aws_cli::findings,
    aws_sso_cli::findings,
    aws_vault::findings,
    azure_cli::findings,
    bash::findings,
    bitwarden_cli::findings,
    buf::findings,
    cariddi::findings,
    censys::findings,
    certbot::findings,
    checkov::findings,
    circleci::findings,
    civo::findings,
    cloudflare_wrangler::findings,
    cloudflared::findings,
    cloudsmith_cli::findings,
    composer::findings,
    curl::findings,
    databricks::findings,
    dcos_cli::findings,
    docker::findings,
    docker_credential_helper::findings,
    docker_machine::findings,
    doctl::findings,
    dropbox_uploader::findings,
    envchain::findings,
    fastlane::findings,
    fastly::findings,
    fauna_shell::findings,
    firebase_cli::findings,
    flyctl::findings,
    gallery_dl::findings,
    gcli::findings,
    git_credential_oauth::findings,
    glab::findings,
    goat::findings,
    gotify::findings,
    gptcommit::findings,
    grafanactl::findings,
    graphite::findings,
    hcloud::findings,
    helm::findings,
    heroku::findings,
    httpie::findings,
    huggingface_cli::findings,
    imap_backup::findings,
    jfrog_cli::findings,
    k6::findings,
    kubernetes_cli::findings,
    luarocks::findings,
    maestro::findings,
    mariadb::findings,
    maven::findings,
    mcp_remote::findings,
    mercurial::findings,
    midnight_commander::findings,
    minio_mc::findings,
    mkcert::findings,
    mongodb_atlas_cli::findings,
    mycli::findings,
    mysql::findings,
    mysql_client::findings,
    mysql_8_0::findings,
    mysql_8_4::findings,
    netlify_cli::findings,
    node::findings,
    node_18::findings,
    nuget::findings,
    oauth2l::findings,
    oci_cli::findings,
    opencode::findings,
    openhue_cli::findings,
    openssh::findings,
    openssl_3::findings,
    openstackclient::findings,
    opentofu::findings,
    openvpn::findings,
    ordercli::findings,
    ossutil::findings,
    oxide_cli::findings,
    perl::findings,
    phylum_cli::findings,
    pianobar::findings,
    plumber::findings,
    pnpm::findings,
    podman::findings,
    poetry::findings,
    pulumi::findings,
    qwen_code::findings,
    railway::findings,
    rclone::findings,
    rsync::findings,
    ruby::findings,
    runpodctl::findings,
    rust::findings,
    s3cmd::findings,
    sbt::findings,
    secretlint::findings,
    sentry_cli::findings,
    shodan::findings,
    skopeo::findings,
    snowflake_cli::findings,
    snyk::findings,
    soracom_cli::findings,
    sqlcmd::findings,
    sshpass::findings,
    sslmate::findings,
    stripe_cli::findings,
    tailscale::findings,
    talosctl::findings,
    terraform::findings,
    terraform_core::findings,
    todoist_cli::findings,
    transifex_cli::findings,
    travis::findings,
    twine::findings,
    uaa_cli::findings,
    uv::findings,
    vagrant::findings,
    vault::findings,
    vercel_cli::findings,
    virustotal_cli::findings,
    vultr::findings,
    wakatime_cli::findings,
    wget::findings,
    wget2::findings,
    wsk::findings,
    yt_dlp::findings,
    zsh::findings,
];

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for detector in DETECTORS {
        findings.extend(detector(home));
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_runs_every_registered_isotope() {
        assert_eq!(DETECTORS.len(), 140);
        assert_eq!(aws::NAME, "aws");
        assert_eq!(git::NAME, "git");
    }
}
