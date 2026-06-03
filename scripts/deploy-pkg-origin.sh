#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

ATLAS_SSH_TARGET="${ATLAS_SSH_TARGET:-ec2-user@16.58.147.215}"
DEPLOY_PORT="${DEPLOY_PORT:-22}"
SSH_IDENTITY_FILE="${SSH_IDENTITY_FILE:-${HOME}/.ssh/smbh-api-ec2-us-east-2.pem}"
AV_WEB_SERVICE="${AV_WEB_SERVICE:-automic-vault-web}"
AV_WEB_REMOTE_ROOT="${AV_WEB_REMOTE_ROOT:-/apps/automic-vault-web}"
AV_WEB_DATA_DIR="${AV_WEB_DATA_DIR:-/var/lib/automic-vault-web}"
AV_WEB_PORT="${AV_WEB_PORT:-3004}"
AV_WEB_BIND_ADDR="${AV_WEB_BIND_ADDR:-127.0.0.1:${AV_WEB_PORT}}"
AV_WEB_ORIGIN_DOMAIN="${AV_WEB_ORIGIN_DOMAIN:-av-origin.automicvault.com}"
AV_WEB_ORIGIN_HEADER="${AV_WEB_ORIGIN_HEADER:-X-Automic-Vault-Origin}"
AV_WEB_ORIGIN_SECRET="${AV_WEB_ORIGIN_SECRET:-}"
AV_WEB_CERTBOT_EMAIL="${AV_WEB_CERTBOT_EMAIL:-}"
AV_WEB_TARGET="${AV_WEB_TARGET:-aarch64-unknown-linux-gnu}"
AV_WEB_SQLITE_PATH="${AV_WEB_SQLITE_PATH:-${repo_root}/cache/pkg.sqlite}"
AV_WEB_BINARY_PATH="${AV_WEB_BINARY_PATH:-}"

skip_refresh=false
skip_sqlite=false
skip_build=false

usage() {
  cat <<EOF
Usage: scripts/deploy-pkg-origin.sh [--skip-refresh] [--skip-sqlite] [--skip-build]

Build and deploy the Atlas Rust package origin.

Environment:
  ATLAS_SSH_TARGET       SSH target. Default: ${ATLAS_SSH_TARGET}
  SSH_IDENTITY_FILE      SSH key. Default: ${SSH_IDENTITY_FILE}
  SSH_EXTRA_OPTS         Extra ssh/scp options.
  AV_WEB_ORIGIN_DOMAIN   Atlas origin hostname. Default: ${AV_WEB_ORIGIN_DOMAIN}
  AV_WEB_ORIGIN_HEADER   CloudFront custom origin header. Default: ${AV_WEB_ORIGIN_HEADER}
  AV_WEB_ORIGIN_SECRET   Required shared secret for the custom origin header.
  AV_WEB_CERTBOT_EMAIL   Email for first-run certbot issuance if TLS cert is missing.
  AV_WEB_TARGET          Rust target. Default: ${AV_WEB_TARGET}
  AV_WEB_BINARY_PATH     Existing av-web binary path when using --skip-build.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-refresh)
      skip_refresh=true
      shift
      ;;
    --skip-sqlite)
      skip_sqlite=true
      shift
      ;;
    --skip-build)
      skip_build=true
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

log() {
  printf '==> %s\n' "$*" >&2
}

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

shell_quote() {
  printf '%q' "$1"
}

expand_path() {
  case "$1" in
    \~) printf '%s\n' "${HOME}" ;;
    \~/*) printf '%s/%s\n' "${HOME}" "${1#\~/}" ;;
    *) printf '%s\n' "$1" ;;
  esac
}

build_ssh_args() {
  ssh_args=(-p "${DEPLOY_PORT}")
  scp_args=(-P "${DEPLOY_PORT}")

  if [[ -n "${SSH_IDENTITY_FILE}" ]]; then
    SSH_IDENTITY_FILE="$(expand_path "${SSH_IDENTITY_FILE}")"
    ssh_args+=(-i "${SSH_IDENTITY_FILE}")
    scp_args+=(-i "${SSH_IDENTITY_FILE}")
  fi

  if [[ -n "${SSH_EXTRA_OPTS:-}" ]]; then
    # shellcheck disable=SC2206
    extra_ssh_args=(${SSH_EXTRA_OPTS})
    ssh_args+=("${extra_ssh_args[@]}")
    scp_args+=("${extra_ssh_args[@]}")
  fi
}

run_refresh_steps() {
  log "Refreshing package-origin source artifacts"
  python3 "${script_dir}/generate-pkg-page-enrichment.py" --refresh
  python3 "${script_dir}/generate-pkg-version-freshness.py"
  python3 "${script_dir}/generate-pkg-manager-indexes.py"
  python3 "${script_dir}/generate-pkg-cross-ecosystem.py"
  python3 "${script_dir}/generate-pkg-graph.py"
  python3 "${script_dir}/generate-pkg-graph-curation.py"
  python3 "${script_dir}/generate-pkg-graph.py"
}

generate_sqlite() {
  log "Generating package-origin SQLite artifact"
  python3 "${script_dir}/generate-pkg-sqlite.py" --output "${AV_WEB_SQLITE_PATH}"
  verify_sqlite
}

verify_sqlite() {
  local integrity
  [[ -f "${AV_WEB_SQLITE_PATH}" ]] || fail "missing SQLite artifact: ${AV_WEB_SQLITE_PATH}"
  integrity="$(sqlite3 "${AV_WEB_SQLITE_PATH}" 'PRAGMA integrity_check;')"
  [[ "${integrity}" == "ok" ]] || fail "sqlite integrity_check failed: ${integrity}"
}

rust_host_target() {
  rustc -vV | awk '/^host:/ { print $2; exit }'
}

rust_target_installed() {
  if command -v rustup >/dev/null 2>&1; then
    rustup target list --installed | grep -qx "${AV_WEB_TARGET}"
    return
  fi
  rustc --print target-libdir --target "${AV_WEB_TARGET}" >/dev/null 2>&1
}

ensure_target_buildable_with_cargo() {
  if rust_target_installed; then
    return 0
  fi

  fail "Rust target ${AV_WEB_TARGET} is not installed. Atlas is ARM64, so this target is correct; install cargo-zigbuild/zig or cross, or run: rustup target add ${AV_WEB_TARGET}"
}

build_binary() {
  if [[ "${skip_build}" == "true" ]]; then
    if [[ -z "${AV_WEB_BINARY_PATH}" ]]; then
      AV_WEB_BINARY_PATH="${repo_root}/target/${AV_WEB_TARGET}/release/av-web"
    fi
    log "Skipping av-web build; using ${AV_WEB_BINARY_PATH}"
    return 0
  fi

  local host_target
  host_target="$(rust_host_target)"
  log "Building av-web for ${AV_WEB_TARGET}"
  if [[ "${AV_WEB_TARGET}" == "${host_target}" ]]; then
    cargo build --release --bin av-web
    AV_WEB_BINARY_PATH="${repo_root}/target/release/av-web"
  elif cargo zigbuild --version >/dev/null 2>&1; then
    cargo zigbuild --release --target "${AV_WEB_TARGET}" --bin av-web
    AV_WEB_BINARY_PATH="${repo_root}/target/${AV_WEB_TARGET}/release/av-web"
  elif command -v cross >/dev/null 2>&1; then
    cross build --release --target "${AV_WEB_TARGET}" --bin av-web
    AV_WEB_BINARY_PATH="${repo_root}/target/${AV_WEB_TARGET}/release/av-web"
  else
    ensure_target_buildable_with_cargo
    cargo build --release --target "${AV_WEB_TARGET}" --bin av-web
    AV_WEB_BINARY_PATH="${repo_root}/target/${AV_WEB_TARGET}/release/av-web"
  fi
}

write_local_config_files() {
  local staging_dir="$1"
  local unit_file="${staging_dir}/automic-vault-web.service"
  local env_file="${staging_dir}/automic-vault-web.env"
  local nginx_http_file="${staging_dir}/automic-vault-web-http.conf"
  local nginx_tls_file="${staging_dir}/automic-vault-web.conf"

  umask 077
  cat >"${env_file}" <<EOF
AV_WEB_BIND_ADDR=${AV_WEB_BIND_ADDR}
AV_WEB_DB_PATH=${AV_WEB_DATA_DIR}/pkg.sqlite
AV_WEB_ORIGIN_HEADER=${AV_WEB_ORIGIN_HEADER}
AV_WEB_ORIGIN_SECRET=${AV_WEB_ORIGIN_SECRET}
EOF
  umask 022

  cat >"${unit_file}" <<EOF
[Unit]
Description=Automic Vault package web origin
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${AV_WEB_SERVICE}
Group=${AV_WEB_SERVICE}
WorkingDirectory=${AV_WEB_REMOTE_ROOT}/current
EnvironmentFile=/etc/automic-vault-web.env
ExecStart=${AV_WEB_REMOTE_ROOT}/current/av-web
Restart=always
RestartSec=3
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true
ReadWritePaths=${AV_WEB_DATA_DIR}

[Install]
WantedBy=multi-user.target
EOF

  cat >"${nginx_http_file}" <<EOF
server {
    listen 80;
    server_name ${AV_WEB_ORIGIN_DOMAIN};

    location /.well-known/acme-challenge/ {
        root /var/www/html;
    }

    location / {
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_pass http://${AV_WEB_BIND_ADDR};
    }
}
EOF

  cat >"${nginx_tls_file}" <<EOF
server {
    listen 80;
    server_name ${AV_WEB_ORIGIN_DOMAIN};

    location /.well-known/acme-challenge/ {
        root /var/www/html;
    }

    location / {
        return 301 https://\$host\$request_uri;
    }
}

server {
    listen 443 ssl http2;
    server_name ${AV_WEB_ORIGIN_DOMAIN};

    ssl_certificate /etc/letsencrypt/live/${AV_WEB_ORIGIN_DOMAIN}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${AV_WEB_ORIGIN_DOMAIN}/privkey.pem;

    location / {
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto https;
        proxy_pass http://${AV_WEB_BIND_ADDR};
    }
}
EOF
}

deploy_remote() {
  local stamp staging_dir remote_tmp

  [[ -n "${AV_WEB_BINARY_PATH}" ]] || fail "missing av-web binary path"
  [[ -x "${AV_WEB_BINARY_PATH}" ]] || fail "missing built binary: ${AV_WEB_BINARY_PATH}"
  [[ -f "${AV_WEB_SQLITE_PATH}" ]] || fail "missing SQLite artifact: ${AV_WEB_SQLITE_PATH}"

  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  staging_dir="$(mktemp -d)"
  trap 'rm -rf "${staging_dir}"' RETURN
  write_local_config_files "${staging_dir}"

  remote_tmp="/tmp/av-web-deploy-${stamp}.$$"
  log "Copying package origin release to Atlas"
  ssh "${ssh_args[@]}" "${ATLAS_SSH_TARGET}" "mkdir -p $(shell_quote "${remote_tmp}")"
  scp "${scp_args[@]}" \
    "${AV_WEB_BINARY_PATH}" \
    "${AV_WEB_SQLITE_PATH}" \
    "${staging_dir}/automic-vault-web.service" \
    "${staging_dir}/automic-vault-web.env" \
    "${staging_dir}/automic-vault-web-http.conf" \
    "${staging_dir}/automic-vault-web.conf" \
    "${ATLAS_SSH_TARGET}:${remote_tmp}/"

  log "Installing package origin on Atlas"
  # shellcheck disable=SC2029
  ssh "${ssh_args[@]}" "${ATLAS_SSH_TARGET}" \
    "bash -s -- $(shell_quote "${remote_tmp}") $(shell_quote "${stamp}") $(shell_quote "${AV_WEB_SERVICE}") $(shell_quote "${AV_WEB_REMOTE_ROOT}") $(shell_quote "${AV_WEB_DATA_DIR}") $(shell_quote "${AV_WEB_ORIGIN_DOMAIN}") $(shell_quote "${AV_WEB_CERTBOT_EMAIL}")" <<'REMOTE'
set -euo pipefail

remote_tmp="$1"
stamp="$2"
service="$3"
remote_root="$4"
data_dir="$5"
origin_domain="$6"
certbot_email="$7"
release_dir="${remote_root}/releases/${stamp}"

sudo useradd --system --home "${remote_root}" --shell /sbin/nologin "${service}" >/dev/null 2>&1 || true
sudo mkdir -p "${release_dir}" "${data_dir}" /var/www/html
sudo install -o root -g root -m 0755 "${remote_tmp}/av-web" "${release_dir}/av-web"
sudo install -o "${service}" -g "${service}" -m 0640 "${remote_tmp}/pkg.sqlite" "${data_dir}/pkg.sqlite.new"
sudo mv "${data_dir}/pkg.sqlite.new" "${data_dir}/pkg.sqlite"
sudo chown "${service}:${service}" "${data_dir}/pkg.sqlite"
sudo ln -sfn "${release_dir}" "${remote_root}/current"
sudo install -o root -g root -m 0644 "${remote_tmp}/automic-vault-web.service" "/etc/systemd/system/automic-vault-web.service"
sudo install -o root -g root -m 0600 "${remote_tmp}/automic-vault-web.env" "/etc/automic-vault-web.env"

if [[ ! -f "/etc/letsencrypt/live/${origin_domain}/fullchain.pem" ]]; then
  sudo install -o root -g root -m 0644 "${remote_tmp}/automic-vault-web-http.conf" "/etc/nginx/conf.d/automic-vault-web.conf"
  sudo nginx -t
  sudo systemctl reload nginx
  if [[ -z "${certbot_email}" ]]; then
    printf 'missing TLS certificate for %s and AV_WEB_CERTBOT_EMAIL was not set\n' "${origin_domain}" >&2
    exit 2
  fi
  sudo certbot --nginx --non-interactive --agree-tos -m "${certbot_email}" -d "${origin_domain}"
fi

sudo install -o root -g root -m 0644 "${remote_tmp}/automic-vault-web.conf" "/etc/nginx/conf.d/automic-vault-web.conf"
sudo systemctl daemon-reload
sudo systemctl enable automic-vault-web.service >/dev/null
sudo systemctl restart automic-vault-web.service
sudo nginx -t
sudo systemctl reload nginx
curl -fsS "http://127.0.0.1:3004/healthz" >/dev/null
rm -rf "${remote_tmp}"
REMOTE
}

if [[ -z "${AV_WEB_ORIGIN_SECRET}" ]]; then
  fail "set AV_WEB_ORIGIN_SECRET to the CloudFront custom origin header value"
fi

require_cmd cargo
require_cmd rustc
require_cmd python3
require_cmd sqlite3
require_cmd ssh
require_cmd scp
build_ssh_args

if [[ "${skip_refresh}" != "true" ]]; then
  run_refresh_steps
fi
if [[ "${skip_sqlite}" == "true" ]]; then
  verify_sqlite
else
  generate_sqlite
fi
build_binary
deploy_remote
log "Atlas package origin deployed: ${AV_WEB_ORIGIN_DOMAIN}"
