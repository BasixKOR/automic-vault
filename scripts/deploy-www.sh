#!/usr/bin/env bash

set -euo pipefail

use_color=false
if [[ -t 2 && -z "${NO_COLOR:-}" && "${TERM:-}" != "dumb" ]]; then
  use_color=true
fi

if [[ "${use_color}" == true ]]; then
  bold=$'\033[1m'
  dim=$'\033[2m'
  red=$'\033[31m'
  green=$'\033[32m'
  blue=$'\033[34m'
  yellow=$'\033[33m'
  reset=$'\033[0m'
  glyph_step="◆"
  glyph_ok="✓"
  glyph_warn="!"
  glyph_error="✗"
else
  bold=""
  dim=""
  red=""
  green=""
  blue=""
  yellow=""
  reset=""
  glyph_step=">"
  glyph_ok="OK"
  glyph_warn="WARN"
  glyph_error="ERROR"
fi

log() {
  printf '%s\n' "$*" >&2
}

log_header() {
  log "${bold}Deploying ${WWW_DOMAIN:-www}${reset}"
  if [[ "${static_only:-false}" == true ]]; then
    log "${dim}Static site -> S3${reset}"
  else
    log "${dim}Static site -> S3 -> CloudFront${reset}"
  fi
}

log_step() {
  log "${blue}${glyph_step}${reset} ${bold}$*${reset}"
}

log_ok() {
  log "  ${green}${glyph_ok}${reset} $*"
}

log_warn() {
  log "  ${yellow}${glyph_warn}${reset} $*"
}

log_error() {
  log "${red}${glyph_error}${reset} $*"
}

die() {
  log_error "$*"
  exit 1
}

on_error() {
  local line="$1"
  log_error "Deployment failed near line ${line}."
  log "${dim}Run with AWS CLI credentials loaded and required .envrc values set.${reset}"
}

trap 'on_error "$LINENO"' ERR

static_only=false

usage() {
  cat >&2 <<'EOF'
Usage: deploy-www.sh [--static-only]

Options:
  --static-only  Sync prepared website files to S3 without changing CloudFront,
                 bucket policy, package origin routing, or certificates.
  -h, --help     Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --static-only)
      static_only=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      die "Unknown option: $1"
      ;;
  esac
done

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    die "Set ${name} in .envrc."
  fi
}

required_tools=(aws node python3)
if [[ "${static_only}" != true ]]; then
  required_tools+=(jq)
fi

for tool in "${required_tools[@]}"; do
  command -v "$tool" >/dev/null 2>&1 || {
    die "Missing required tool: ${tool}."
  }
done

WWW_PKG_ORIGIN_HEADER_NAME="${WWW_PKG_ORIGIN_HEADER_NAME:-X-Automic-Vault-Origin}"
WWW_EMERGENCY_INVALIDATE="${WWW_EMERGENCY_INVALIDATE:-false}"

require_env AWS_REGION
require_env WWW_DOMAIN

export WWW_WWW_DOMAIN="${WWW_WWW_DOMAIN:-www.${WWW_DOMAIN}}"
export WWW_CANONICAL_HOST="${WWW_CANONICAL_HOST:-${WWW_WWW_DOMAIN}}"
export WWW_BUCKET="${WWW_BUCKET:-${WWW_DOMAIN}}"
export WWW_CLOUDFRONT_PRICE_CLASS="${WWW_CLOUDFRONT_PRICE_CLASS:-PriceClass_100}"
export WWW_HTML_CACHE_CONTROL="${WWW_HTML_CACHE_CONTROL:-public, max-age=60, must-revalidate}"
export WWW_ASSET_CACHE_CONTROL="${WWW_ASSET_CACHE_CONTROL:-public, max-age=31536000, immutable}"

for env_name in \
  WWW_BUCKET \
  WWW_HTML_CACHE_CONTROL \
  WWW_ASSET_CACHE_CONTROL
do
  require_env "${env_name}"
done

if [[ "${static_only}" != true ]]; then
  for env_name in \
    WWW_WWW_DOMAIN \
    WWW_CANONICAL_HOST \
    WWW_CERTIFICATE_ARN \
    WWW_CLOUDFRONT_PRICE_CLASS \
    WWW_PKG_ORIGIN_DOMAIN \
    WWW_PKG_ORIGIN_HEADER_VALUE
  do
    require_env "${env_name}"
  done
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
site_dir="${repo_root}/www"
llms_full_generator="${repo_root}/scripts/generate-llms-full.mjs"
www_i18n_generator="${repo_root}/scripts/generate-www-i18n.py"
product_version_source="${repo_root}/Cargo.toml"
db_source="${repo_root}/data/combined.json"
scan_log_source="${repo_root}/data/radioisotopes/SCAN_LOG.md"
prepared_site_dir=""

if [[ ! -d "${site_dir}" ]]; then
  die "Missing site directory: ${site_dir}"
fi

if [[ ! -f "${db_source}" ]]; then
  die "Missing database source: ${db_source}"
fi

if [[ ! -f "${product_version_source}" ]]; then
  die "Missing product version source: ${product_version_source}"
fi

origin_domain="${WWW_BUCKET}.s3.${AWS_REGION}.amazonaws.com"
pkg_origin_id="${WWW_DOMAIN}-atlas-pkg-origin"
distribution_comment="${WWW_DOMAIN} static site"
oac_name="${WWW_DOMAIN}-s3-oac"
redirect_function_name="${WWW_DOMAIN//./-}-redirect-to-canonical"
response_headers_policy_name="${WWW_DOMAIN//./-}-security-headers"
cache_policy_name="${WWW_DOMAIN//./-}-brotli-cache"
pkg_cache_policy_name="${WWW_DOMAIN//./-}-pkg-daily-cache"
pkg_search_cache_policy_name="${WWW_DOMAIN//./-}-pkg-search-daily-cache"

cleanup() {
  if [[ -n "${prepared_site_dir}" && -d "${prepared_site_dir}" ]]; then
    rm -rf "${prepared_site_dir}"
  fi
}

trap cleanup EXIT

count_scan_log_entries() {
  if [[ ! -f "${scan_log_source}" ]]; then
    die "Missing scan log: ${scan_log_source}"
  fi

  local count
  count="$(awk '/^\|[[:space:]]*[0-9]+[[:space:]]*\|/ { count++ } END { print count + 0 }' "${scan_log_source}")"

  if [[ -z "${count}" || "${count}" == "0" ]]; then
    die "Could not find scan log entries in ${scan_log_source}"
  fi

  printf '%s\n' "${count}"
}

format_count_for_display() {
  local count="$1"
  perl -e '
    my $count = shift;
    die "Invalid count: $count\n" unless defined $count && $count =~ /\A[0-9]+\z/;
    1 while $count =~ s/^([0-9]+)([0-9]{3})/$1,$2/;
    print "$count\n";
  ' "${count}"
}

read_product_version() {
  local version
  version="$(
    awk -F'"' '/^version = / { print $2; exit }' "${product_version_source}"
  )"

  if [[ -z "${version}" ]]; then
    die "Could not read product version from ${product_version_source}"
  fi

  if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
    die "Unexpected product version in ${product_version_source}: ${version}"
  fi

  printf '%s\n' "${version}"
}

prepared_product_files() {
  find "${prepared_site_dir}" \
    \( -path "${prepared_site_dir}/pkg" -o -path "${prepared_site_dir}/pagefind" \) -prune \
    -o -type f \
    \( -name '*.html' -o -name '*.txt' -o -name '*.md' -o -name '*.json' \) \
    -print0
}

assert_product_version_stamped() {
  local product_version="$1"
  local file mismatch_file
  mismatch_file="$(mktemp)"

  while IFS= read -r -d '' file; do
    PRODUCT_VERSION="${product_version}" perl -0ne '
      my $version = $ENV{"PRODUCT_VERSION"};
      if (/__AUTOMIC_VAULT_VERSION__/) {
        print "$ARGV: unresolved __AUTOMIC_VAULT_VERSION__ placeholder\n";
      }
      while (/"softwareVersion"\s*:\s*"([^"]+)"/g) {
        print "$ARGV: softwareVersion=$1 expected $version\n" if $1 ne $version;
      }
      while (/^- Current version:\s*([^\r\n]+)/mg) {
        my $current = $1;
        $current =~ s/\s+$//;
        print "$ARGV: Current version=$current expected $version\n" if $current ne $version;
      }
    ' "${file}" >>"${mismatch_file}"
  done < <(prepared_product_files)

  if [[ -s "${mismatch_file}" ]]; then
    log_error "Product version stamping left mismatches:"
    sed -n '1,40p' "${mismatch_file}" >&2
    rm -f "${mismatch_file}"
    die "Prepared site product version must match ${product_version} before deploy."
  fi

  rm -f "${mismatch_file}"
}

stamp_product_version() {
  local product_version="$1"
  local file file_count
  file_count=0

  while IFS= read -r -d '' file; do
    PRODUCT_VERSION="${product_version}" perl -0pi -e '
      my $version = $ENV{"PRODUCT_VERSION"};
      s{__AUTOMIC_VAULT_VERSION__}{$version}g;
      s{("softwareVersion"\s*:\s*")[^"]+(")}{$1 . $version . $2}ge;
      s{(- Current version:\s*)[^\r\n]+}{$1 . $version}ge;
    ' "${file}"
    file_count=$((file_count + 1))
  done < <(prepared_product_files)

  if [[ "${file_count}" == "0" ]]; then
    die "No prepared product files found for version stamping."
  fi

  assert_product_version_stamped "${product_version}"
}

prepare_site_for_upload() {
  local product_version scanned_package_count scanned_package_display_count index_path
  local stamped_scan_count
  log_step "Preparing deploy-time site content"
  product_version="$(read_product_version)"
  scanned_package_count="$(count_scan_log_entries)"
  scanned_package_display_count="$(format_count_for_display "${scanned_package_count}")"
  prepared_site_dir="$(mktemp -d)"
  rsync -a \
    --exclude '/pkg/' \
    --exclude '/*/pkg/' \
    --exclude '/pagefind/' \
    "${site_dir}/" "${prepared_site_dir}/"
  stamp_product_version "${product_version}"

  index_path="${prepared_site_dir}/index.html"
  if [[ ! -f "${index_path}" ]]; then
    die "Missing prepared index: ${index_path}"
  fi

  stamped_scan_count=false
  if perl -0ne '
    exit(
      /<([a-zA-Z][a-zA-Z0-9:-]*)\b[^>]*\bdata-secured-package-count\b[^>]*>[^<]*<\/\1>/ ||
      /<span>[0-9,]+ Homebrew packages scanned<\/span>/
        ? 0
        : 1
    );
  ' "${index_path}"; then
    SCANNED_PACKAGE_COUNT="${scanned_package_display_count}" perl -0pi -e '
      BEGIN {
        $count = $ENV{"SCANNED_PACKAGE_COUNT"};
        $matches = 0;
      }
      $matches += s{(<([a-zA-Z][a-zA-Z0-9:-]*)\b[^>]*\bdata-secured-package-count\b[^>]*>)[^<]*(</\2>)}{$1$count$3}g;
      $matches += s{(<span>)[0-9,]+ Homebrew packages scanned(</span>)}{$1$count Homebrew packages scanned$2}g;
      END {
        die "Expected exactly one scanned package count replacement, got $matches\n"
          unless $matches == 1;
      }
    ' "${index_path}"
    stamped_scan_count=true
  else
    log_warn "No homepage scanned package count marker; skipped scan-count stamp"
  fi

  node "${llms_full_generator}" "${prepared_site_dir}" "${prepared_site_dir}/llms-full.txt"
  stamp_product_version "${product_version}"

  log_ok "Stamped Automic Vault ${product_version}"
  if [[ "${stamped_scan_count}" == "true" ]]; then
    log_ok "Stamped ${scanned_package_display_count} scanned packages"
  fi
}

assert_www_i18n_current() {
  log_step "Checking localized website pages"
  if [[ ! -x "${www_i18n_generator}" && ! -f "${www_i18n_generator}" ]]; then
    die "Missing website i18n generator: ${www_i18n_generator}"
  fi
  python3 "${www_i18n_generator}" --check
}

ensure_bucket() {
  log_step "Preparing S3 bucket"
  if ! aws s3api head-bucket --bucket "${WWW_BUCKET}" >/dev/null 2>&1; then
    log "  Creating ${WWW_BUCKET}"
    if [[ "${AWS_REGION}" == "us-east-1" ]]; then
      aws s3api create-bucket --bucket "${WWW_BUCKET}"
    else
      aws s3api create-bucket \
        --bucket "${WWW_BUCKET}" \
        --create-bucket-configuration \
        "LocationConstraint=${AWS_REGION}"
    fi
  else
    log "  Bucket exists: ${WWW_BUCKET}"
  fi

  log "  Blocking public access"
  aws s3api put-public-access-block \
    --bucket "${WWW_BUCKET}" \
    --public-access-block-configuration \
    BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true

  log "  Enforcing bucket-owner object ownership"
  aws s3api put-bucket-ownership-controls \
    --bucket "${WWW_BUCKET}" \
    --ownership-controls 'Rules=[{ObjectOwnership=BucketOwnerEnforced}]'

  log "  Enabling AES256 server-side encryption"
  aws s3api put-bucket-encryption \
    --bucket "${WWW_BUCKET}" \
    --server-side-encryption-configuration \
    '{"Rules":[{"ApplyServerSideEncryptionByDefault":{"SSEAlgorithm":"AES256"}}]}'
  log_ok "S3 bucket ready"
}

ensure_oac() {
  local existing_id
  log_step "Preparing CloudFront origin access control"
  existing_id="$(
    aws cloudfront list-origin-access-controls \
      --query "OriginAccessControlList.Items[?Name==\`${oac_name}\`].Id | [0]" \
      --output text
  )"

  if [[ -n "${existing_id}" && "${existing_id}" != "None" ]]; then
    log_ok "Using existing OAC ${existing_id}"
    printf '%s\n' "${existing_id}"
    return 0
  fi

  local config_file created_id
  config_file="$(mktemp)"
  jq -n \
    --arg name "${oac_name}" \
    '{
      Name: $name,
      Description: "Origin access control for static site bucket",
      OriginAccessControlOriginType: "s3",
      SigningBehavior: "always",
      SigningProtocol: "sigv4"
    }' >"${config_file}"

  created_id="$(
    aws cloudfront create-origin-access-control \
    --origin-access-control-config "file://${config_file}" \
    --query 'OriginAccessControl.Id' \
    --output text
  )"
  log_ok "Created OAC ${created_id}"
  printf '%s\n' "${created_id}"
}

ensure_redirect_function() {
  local function_file function_etag stage
  log_step "Publishing canonical-host redirect function"
  function_file="$(mktemp)"
  cat >"${function_file}" <<EOF
function handler(event) {
  var request = event.request;
  var host = request.headers.host.value;
  var canonicalHost = "${WWW_CANONICAL_HOST}";

  function preferredContentType() {
    var header = request.headers.accept && request.headers.accept.value;
    var supported = ["text/html", "text/markdown", "text/plain", "application/json"];
    var bestType = "text/html";
    var bestQ = -1;
    var bestOrder = 999999;
    var bestSpecificity = -1;

    if (!header) {
      return bestType;
    }

    var ranges = header.split(",");
    for (var order = 0; order < ranges.length; order++) {
      var range = ranges[order].replace(/^\s+|\s+$/g, "");
      if (!range) {
        continue;
      }
      var parts = range.split(";");
      var media = parts[0].replace(/^\s+|\s+$/g, "").toLowerCase();
      var q = 1;

      for (var paramIndex = 1; paramIndex < parts.length; paramIndex++) {
        var param = parts[paramIndex].replace(/^\s+|\s+$/g, "").toLowerCase();
        if (param.slice(0, 2) === "q=") {
          var parsedQ = parseFloat(param.slice(2));
          q = isNaN(parsedQ) ? 0 : parsedQ;
        }
      }

      if (q <= 0) {
        continue;
      }

      for (var typeIndex = 0; typeIndex < supported.length; typeIndex++) {
        var candidate = supported[typeIndex];
        var specificity = -1;
        if (media === candidate) {
          specificity = 2;
        } else if (media.slice(-2) === "/*" && candidate.indexOf(media.slice(0, media.length - 1)) === 0) {
          specificity = 1;
        } else if (media === "*/*") {
          specificity = 0;
        }

        if (specificity < 0) {
          continue;
        }
        if (
          q > bestQ ||
          (q === bestQ && order < bestOrder) ||
          (q === bestQ && order === bestOrder && specificity > bestSpecificity)
        ) {
          bestType = candidate;
          bestQ = q;
          bestOrder = order;
          bestSpecificity = specificity;
        }
      }
    }

    return bestType;
  }

  function isKnownRoute(uri) {
    var routes = {
      "/": true,
      "/about": true,
      "/about/": true,
      "/ai-agent-approval-gates": true,
      "/ai-agent-approval-gates/": true,
      "/api-key-management-for-ai-agents": true,
      "/api-key-management-for-ai-agents/": true,
      "/av-trace": true,
      "/av-trace/": true,
      "/docs": true,
      "/docs/": true,
      "/download": true,
      "/download/": true,
      "/github-cli-token-security-ai-agents": true,
      "/github-cli-token-security-ai-agents/": true,
      "/hashicorp-vault-for-ai-agents": true,
      "/hashicorp-vault-for-ai-agents/": true,
      "/mcp-secrets-management": true,
      "/mcp-secrets-management/": true,
      "/privacy": true,
      "/privacy/": true,
      "/pricing": true,
      "/pricing/": true,
      "/privileged-access-management-for-ai-agents": true,
      "/privileged-access-management-for-ai-agents/": true,
      "/secret-scanner-for-ai-agents": true,
      "/secret-scanner-for-ai-agents/": true,
      "/secret-scanning-vs-agent-secret-protection": true,
      "/secret-scanning-vs-agent-secret-protection/": true,
      "/secrets-manager-for-ai-agents": true,
      "/secrets-manager-for-ai-agents/": true,
      "/secure-aws-cli-credentials-ai-agents": true,
      "/secure-aws-cli-credentials-ai-agents/": true,
      "/security": true,
      "/security/": true,
      "/security/whitepaper": true,
      "/security/whitepaper/": true,
      "/stop-ai-agents-reading-env-files": true,
      "/stop-ai-agents-reading-env-files/": true,
      "/terms": true,
      "/terms/": true
    };
    return routes[uri] === true;
  }

  function jsonNotFound() {
    return {
      statusCode: 404,
      statusDescription: "Not Found",
      headers: {
        "content-type": { value: "application/json; charset=utf-8" }
      },
      body: JSON.stringify({ error: "not_found", path: request.uri })
    };
  }

  function appendQueryString(location) {
    if (request.querystring && Object.keys(request.querystring).length > 0) {
      var parts = [];
      for (var key in request.querystring) {
        if (!Object.prototype.hasOwnProperty.call(request.querystring, key)) {
          continue;
        }
        var entry = request.querystring[key];
        if (entry.multiValue) {
          for (var i = 0; i < entry.multiValue.length; i++) {
            var item = entry.multiValue[i];
            parts.push(encodeURIComponent(key) + "=" + encodeURIComponent(item.value));
          }
        } else {
          parts.push(encodeURIComponent(key) + "=" + encodeURIComponent(entry.value));
        }
      }
      if (parts.length > 0) {
        return location + "?" + parts.join("&");
      }
    }
    return location;
  }

  function viewerProtocol() {
    var header = request.headers["cloudfront-forwarded-proto"];
    return header && header.value ? header.value.toLowerCase() : "https";
  }

  function canonicalLocation(uri) {
    return appendQueryString("https://" + canonicalHost + uri);
  }

  if (request.uri === "/av.dmg") {
    return {
      statusCode: 301,
      statusDescription: "Moved Permanently",
      headers: {
        location: { value: canonicalLocation("/Automic%20Vault.dmg") }
      }
    };
  }
  if (host !== canonicalHost || viewerProtocol() === "http") {
    return {
      statusCode: 301,
      statusDescription: "Moved Permanently",
      headers: {
        location: { value: canonicalLocation(request.uri) }
      }
    };
  }
  if (request.uri === "/install.sh" || request.uri === "/scanner.sh" || request.uri === "/scanner.gz") {
    return request;
  }

  var preferredType = preferredContentType();
  if (request.uri === "/" || request.uri === "/index.html") {
    if (preferredType === "text/markdown") {
      request.uri = "/index.md";
    } else if (preferredType === "text/plain") {
      request.uri = "/index.txt";
    } else if (preferredType === "application/json") {
      request.uri = "/index.json";
    }
    return request;
  }
  if (preferredType === "application/json" && request.uri.indexOf(".") === -1 && !isKnownRoute(request.uri)) {
    return jsonNotFound();
  }
  if (request.uri !== "/" && request.uri.slice(-1) !== "/" && request.uri.indexOf(".") === -1) {
    return {
      statusCode: 301,
      statusDescription: "Moved Permanently",
      headers: {
        location: { value: appendQueryString(request.uri + "/") }
      }
    };
  }
  if (request.uri === "/docs") {
    return {
      statusCode: 301,
      statusDescription: "Moved Permanently",
      headers: {
        location: { value: appendQueryString("/docs/") }
      }
    };
  }
  if (request.uri !== "/" && request.uri.slice(-1) === "/") {
    request.uri = request.uri + "index.html";
  }
  return request;
}

EOF


  if aws cloudfront describe-function --name "${redirect_function_name}" >/dev/null 2>&1; then
    log "  Updating ${redirect_function_name}"
    function_etag="$(
      aws cloudfront describe-function \
        --name "${redirect_function_name}" \
        --query 'ETag' \
        --output text
    )"
    aws cloudfront update-function \
      --name "${redirect_function_name}" \
      --if-match "${function_etag}" \
        --function-config Comment="Canonical host redirect and docs index routing",Runtime=cloudfront-js-2.0 \
        --function-code "fileb://${function_file}" >/dev/null
  else
    log "  Creating ${redirect_function_name}"
    aws cloudfront create-function \
      --name "${redirect_function_name}" \
      --function-config Comment="Canonical host redirect and docs index routing",Runtime=cloudfront-js-2.0 \
      --function-code "fileb://${function_file}" >/dev/null
  fi

  function_etag="$(
    aws cloudfront describe-function \
      --name "${redirect_function_name}" \
      --query 'ETag' \
      --output text
  )"
  stage="$(
    aws cloudfront describe-function \
      --name "${redirect_function_name}" \
      --query 'FunctionSummary.FunctionConfig.Stage' \
      --output text
  )"
  if [[ "${stage}" != "LIVE" ]]; then
    log "  Publishing function to LIVE"
    aws cloudfront publish-function \
      --name "${redirect_function_name}" \
      --if-match "${function_etag}" >/dev/null
  else
    log "  Function is already LIVE"
  fi
  log_ok "Redirect function ready"
}

ensure_response_headers_policy() {
  local policy_file policy_id etag response_file
  log_step "Preparing CloudFront security headers policy"
  policy_file="$(mktemp)"
  response_file="$(mktemp)"

  jq -n \
    --arg name "${response_headers_policy_name}" \
    '{
      Name: $name,
      Comment: "Security headers for Automic Vault static site",
      SecurityHeadersConfig: {
        StrictTransportSecurity: {
          Override: true,
          AccessControlMaxAgeSec: 63072000,
          IncludeSubdomains: true,
          Preload: true
        },
        ContentTypeOptions: {
          Override: true
        },
        FrameOptions: {
          Override: true,
          FrameOption: "DENY"
        },
        ReferrerPolicy: {
          Override: true,
          ReferrerPolicy: "strict-origin-when-cross-origin"
        },
        XSSProtection: {
          Override: true,
          Protection: true,
          ModeBlock: true
        },
        ContentSecurityPolicy: {
          Override: true,
          ContentSecurityPolicy: "default-src '\''self'\''; script-src '\''self'\'' '\''unsafe-inline'\'' '\''wasm-unsafe-eval'\''; style-src '\''self'\'' '\''unsafe-inline'\'' https://fonts.googleapis.com; font-src '\''self'\'' https://fonts.gstatic.com; img-src '\''self'\'' data: https://www.automicvault.com; connect-src '\''self'\''; frame-ancestors '\''none'\''; base-uri '\''self'\''; form-action '\''none'\''"
        }
      },
      CustomHeadersConfig: {
        Quantity: 1,
        Items: [
          {
            Header: "Permissions-Policy",
            Value: "camera=(), microphone=(), geolocation=(), payment=()",
            Override: true
          }
        ]
      },
      ServerTimingHeadersConfig: {
        Enabled: false
      },
      RemoveHeadersConfig: {
        Quantity: 0
      }
    }' >"${policy_file}"

  policy_id="$(
    aws cloudfront list-response-headers-policies \
      --type custom \
      --query "ResponseHeadersPolicyList.Items[?ResponseHeadersPolicy.ResponseHeadersPolicyConfig.Name == '${response_headers_policy_name}'].ResponseHeadersPolicy.Id | [0]" \
      --output text
  )"

  if [[ "${policy_id}" == "None" ]]; then
    policy_id="$(
      aws cloudfront create-response-headers-policy \
        --response-headers-policy-config "file://${policy_file}" \
        --query 'ResponseHeadersPolicy.Id' \
        --output text
    )"
    log_ok "Created response headers policy ${policy_id}"
    printf '%s\n' "${policy_id}"
    return 0
  fi

  aws cloudfront get-response-headers-policy-config \
    --id "${policy_id}" >"${response_file}"
  etag="$(jq -r '.ETag' "${response_file}")"
  aws cloudfront update-response-headers-policy \
    --id "${policy_id}" \
    --if-match "${etag}" \
    --response-headers-policy-config "file://${policy_file}" >/dev/null
  log_ok "Response headers policy ready"
  printf '%s\n' "${policy_id}"
}

ensure_cache_policy() {
  local policy_file policy_id etag response_file
  log_step "Preparing CloudFront Brotli cache policy"
  policy_file="$(mktemp)"
  response_file="$(mktemp)"

  jq -n \
    --arg name "${cache_policy_name}" \
    '{
      Name: $name,
      Comment: "Static site cache policy with Gzip, Brotli, and viewer protocol for canonical redirects",
      DefaultTTL: 86400,
      MaxTTL: 31536000,
      MinTTL: 0,
      ParametersInCacheKeyAndForwardedToOrigin: {
        EnableAcceptEncodingGzip: true,
        EnableAcceptEncodingBrotli: true,
        HeadersConfig: {
          HeaderBehavior: "whitelist",
          Headers: {
            Quantity: 1,
            Items: ["CloudFront-Forwarded-Proto"]
          }
        },
        CookiesConfig: {
          CookieBehavior: "none"
        },
        QueryStringsConfig: {
          QueryStringBehavior: "none"
        }
      }
    }' >"${policy_file}"

  policy_id="$(
    aws cloudfront list-cache-policies \
      --type custom \
      --query "CachePolicyList.Items[?CachePolicy.CachePolicyConfig.Name == '${cache_policy_name}'].CachePolicy.Id | [0]" \
      --output text
  )"

  if [[ "${policy_id}" == "None" ]]; then
    policy_id="$(
      aws cloudfront create-cache-policy \
        --cache-policy-config "file://${policy_file}" \
        --query 'CachePolicy.Id' \
        --output text
    )"
    log_ok "Created cache policy ${policy_id}"
    printf '%s\n' "${policy_id}"
    return 0
  fi

  aws cloudfront get-cache-policy-config \
    --id "${policy_id}" >"${response_file}"
  etag="$(jq -r '.ETag' "${response_file}")"
  aws cloudfront update-cache-policy \
    --id "${policy_id}" \
    --if-match "${etag}" \
    --cache-policy-config "file://${policy_file}" >/dev/null
  log_ok "Cache policy ready"
  printf '%s\n' "${policy_id}"
}

ensure_pkg_cache_policy() {
  local policy_file policy_id etag response_file
  log_step "Preparing CloudFront package-origin daily cache policy"
  policy_file="$(mktemp)"
  response_file="$(mktemp)"

  jq -n \
    --arg name "${pkg_cache_policy_name}" \
    '{
      Name: $name,
      Comment: "Package origin cache; CloudFront checks Atlas daily",
      DefaultTTL: 86400,
      MaxTTL: 86400,
      MinTTL: 0,
      ParametersInCacheKeyAndForwardedToOrigin: {
        EnableAcceptEncodingGzip: true,
        EnableAcceptEncodingBrotli: true,
        HeadersConfig: { HeaderBehavior: "none" },
        CookiesConfig: { CookieBehavior: "none" },
        QueryStringsConfig: { QueryStringBehavior: "none" }
      }
    }' >"${policy_file}"

  policy_id="$(
    aws cloudfront list-cache-policies \
      --type custom \
      --query "CachePolicyList.Items[?CachePolicy.CachePolicyConfig.Name == '${pkg_cache_policy_name}'].CachePolicy.Id | [0]" \
      --output text
  )"

  if [[ "${policy_id}" == "None" ]]; then
    policy_id="$(
      aws cloudfront create-cache-policy \
        --cache-policy-config "file://${policy_file}" \
        --query 'CachePolicy.Id' \
        --output text
    )"
    log_ok "Created package cache policy ${policy_id}"
    printf '%s\n' "${policy_id}"
    return 0
  fi

  aws cloudfront get-cache-policy-config \
    --id "${policy_id}" >"${response_file}"
  etag="$(jq -r '.ETag' "${response_file}")"
  aws cloudfront update-cache-policy \
    --id "${policy_id}" \
    --if-match "${etag}" \
    --cache-policy-config "file://${policy_file}" >/dev/null
  log_ok "Package cache policy ready"
  printf '%s\n' "${policy_id}"
}

ensure_pkg_search_cache_policy() {
  local policy_file policy_id etag response_file
  log_step "Preparing CloudFront package search cache policy"
  policy_file="$(mktemp)"
  response_file="$(mktemp)"

  jq -n \
    --arg name "${pkg_search_cache_policy_name}" \
    '{
      Name: $name,
      Comment: "Package search cache with search query parameters",
      DefaultTTL: 86400,
      MaxTTL: 86400,
      MinTTL: 0,
      ParametersInCacheKeyAndForwardedToOrigin: {
        EnableAcceptEncodingGzip: true,
        EnableAcceptEncodingBrotli: true,
        HeadersConfig: { HeaderBehavior: "none" },
        CookiesConfig: { CookieBehavior: "none" },
        QueryStringsConfig: {
          QueryStringBehavior: "whitelist",
          QueryStrings: {
            Quantity: 4,
            Items: ["q", "offset", "limit", "locale"]
          }
        }
      }
    }' >"${policy_file}"

  policy_id="$(
    aws cloudfront list-cache-policies \
      --type custom \
      --query "CachePolicyList.Items[?CachePolicy.CachePolicyConfig.Name == '${pkg_search_cache_policy_name}'].CachePolicy.Id | [0]" \
      --output text
  )"

  if [[ "${policy_id}" == "None" ]]; then
    policy_id="$(
      aws cloudfront create-cache-policy \
        --cache-policy-config "file://${policy_file}" \
        --query 'CachePolicy.Id' \
        --output text
    )"
    log_ok "Created package search cache policy ${policy_id}"
    printf '%s\n' "${policy_id}"
    return 0
  fi

  aws cloudfront get-cache-policy-config \
    --id "${policy_id}" >"${response_file}"
  etag="$(jq -r '.ETag' "${response_file}")"
  aws cloudfront update-cache-policy \
    --id "${policy_id}" \
    --if-match "${etag}" \
    --cache-policy-config "file://${policy_file}" >/dev/null
  log_ok "Package search cache policy ready"
  printf '%s\n' "${policy_id}"
}


distribution_id_for_alias() {
  local alias_csv
  alias_csv="$(
    aws cloudfront list-distributions \
      --query "DistributionList.Items[?Aliases.Items && contains(join(',', Aliases.Items), '${WWW_DOMAIN}')].Id | [0]" \
      --output text
  )"

  if [[ "${alias_csv}" == "None" ]]; then
    return 1
  fi

  printf '%s\n' "${alias_csv}"
}

build_distribution_config() {
  local oac_id="$1"
  local function_arn="$2"
  local response_headers_policy_id="$3"
  local cache_policy_id="$4"
  local pkg_cache_policy_id="$5"
  local pkg_search_cache_policy_id="$6"
  local output_file="$7"

  jq -n \
    --arg caller_reference "${WWW_DOMAIN}-$(date +%s)" \
    --arg comment "${distribution_comment}" \
    --arg origin_id "${WWW_BUCKET}-origin" \
    --arg domain_name "${origin_domain}" \
    --arg pkg_origin_id "${pkg_origin_id}" \
    --arg pkg_origin_domain "${WWW_PKG_ORIGIN_DOMAIN}" \
    --arg pkg_origin_header_name "${WWW_PKG_ORIGIN_HEADER_NAME}" \
    --arg pkg_origin_header_value "${WWW_PKG_ORIGIN_HEADER_VALUE}" \
    --arg oac_id "${oac_id}" \
    --arg function_arn "${function_arn}" \
    --arg response_headers_policy_id "${response_headers_policy_id}" \
    --arg cache_policy_id "${cache_policy_id}" \
    --arg pkg_cache_policy_id "${pkg_cache_policy_id}" \
    --arg pkg_search_cache_policy_id "${pkg_search_cache_policy_id}" \
    --arg cert_arn "${WWW_CERTIFICATE_ARN}" \
    --arg domain_a "${WWW_DOMAIN}" \
    --arg domain_b "${WWW_WWW_DOMAIN}" \
    --arg price_class "${WWW_CLOUDFRONT_PRICE_CLASS}" \
    '
    def behavior($pattern; $policy):
      {
        PathPattern: $pattern,
        TargetOriginId: $pkg_origin_id,
        ViewerProtocolPolicy: "allow-all",
        AllowedMethods: {
          Quantity: 2,
          Items: ["HEAD", "GET"],
          CachedMethods: {
            Quantity: 2,
            Items: ["HEAD", "GET"]
          }
        },
        Compress: true,
        SmoothStreaming: false,
        CachePolicyId: $policy,
        ResponseHeadersPolicyId: $response_headers_policy_id,
        TrustedSigners: {
          Enabled: false,
          Quantity: 0
        },
        TrustedKeyGroups: {
          Enabled: false,
          Quantity: 0
        },
        LambdaFunctionAssociations: {
          Quantity: 0
        },
        FunctionAssociations: {
          Quantity: 1,
          Items: [{
            EventType: "viewer-request",
            FunctionARN: $function_arn
          }]
        },
        FieldLevelEncryptionId: ""
      };
    {
      CallerReference: $caller_reference,
      Comment: $comment,
      Enabled: true,
      DefaultRootObject: "index.html",
      Origins: {
        Quantity: 2,
        Items: [{
          Id: $origin_id,
          DomainName: $domain_name,
          OriginPath: "",
          OriginAccessControlId: $oac_id,
          S3OriginConfig: {
            OriginAccessIdentity: ""
          }
        }, {
          Id: $pkg_origin_id,
          DomainName: $pkg_origin_domain,
          OriginPath: "",
          CustomHeaders: {
            Quantity: 1,
            Items: [{
              HeaderName: $pkg_origin_header_name,
              HeaderValue: $pkg_origin_header_value
            }]
          },
          CustomOriginConfig: {
            HTTPPort: 80,
            HTTPSPort: 443,
            OriginProtocolPolicy: "https-only",
            OriginSslProtocols: {
              Quantity: 1,
              Items: ["TLSv1.2"]
            },
            OriginReadTimeout: 30,
            OriginKeepaliveTimeout: 5
          }
        }]
      },
      Aliases: {
        Quantity: 2,
        Items: [$domain_a, $domain_b]
      },
      DefaultCacheBehavior: {
        TargetOriginId: $origin_id,
        ViewerProtocolPolicy: "allow-all",
        AllowedMethods: {
          Quantity: 2,
          Items: ["HEAD", "GET"],
          CachedMethods: {
            Quantity: 2,
            Items: ["HEAD", "GET"]
          }
        },
        Compress: true,
        SmoothStreaming: false,
        CachePolicyId: $cache_policy_id,
        ResponseHeadersPolicyId: $response_headers_policy_id,
        TrustedSigners: {
          Enabled: false,
          Quantity: 0
        },
        TrustedKeyGroups: {
          Enabled: false,
          Quantity: 0
        },
        LambdaFunctionAssociations: {
          Quantity: 0
        },
        FunctionAssociations: {
          Quantity: 1,
          Items: [{
            EventType: "viewer-request",
            FunctionARN: $function_arn
          }]
        },
        FieldLevelEncryptionId: ""
      },
      CacheBehaviors: {
        Quantity: 7,
        Items: [
          behavior("pkg/search.json"; $pkg_search_cache_policy_id),
          behavior("*/pkg/search.json"; $pkg_search_cache_policy_id),
          behavior("pkg*"; $pkg_cache_policy_id),
          behavior("de/pkg*"; $pkg_cache_policy_id),
          behavior("fr/pkg*"; $pkg_cache_policy_id),
          behavior("ja/pkg*"; $pkg_cache_policy_id),
          behavior("zh-hans/pkg*"; $pkg_cache_policy_id)
        ]
      },
      CustomErrorResponses: {
        Quantity: 1,
        Items: [{
          ErrorCode: 403,
          ResponsePagePath: "/404.html",
          ResponseCode: "404",
          ErrorCachingMinTTL: 60
        }]
      },
      Restrictions: {
        GeoRestriction: {
          RestrictionType: "none",
          Quantity: 0
        }
      },
      ViewerCertificate: {
        ACMCertificateArn: $cert_arn,
        SSLSupportMethod: "sni-only",
        MinimumProtocolVersion: "TLSv1.2_2021",
        Certificate: $cert_arn,
        CertificateSource: "acm"
      },
      PriceClass: $price_class,
      HttpVersion: "http2",
      IsIPV6Enabled: true
    }' >"${output_file}"
}

upsert_distribution() {
  local oac_id="$1"
  local function_arn="$2"
  local response_headers_policy_id="$3"
  local cache_policy_id="$4"
  local pkg_cache_policy_id="$5"
  local pkg_search_cache_policy_id="$6"
  local distribution_id etag config_file response_file
  log_step "Preparing CloudFront distribution"
  config_file="$(mktemp)"
  response_file="$(mktemp)"

  if distribution_id="$(distribution_id_for_alias)"; then
    log "  Updating distribution ${distribution_id}"
    aws cloudfront get-distribution-config \
      --id "${distribution_id}" >"${response_file}"
    etag="$(jq -r '.ETag' "${response_file}")"
    jq \
      --arg comment "${distribution_comment}" \
      --arg origin_id "${WWW_BUCKET}-origin" \
      --arg domain_name "${origin_domain}" \
      --arg pkg_origin_id "${pkg_origin_id}" \
      --arg pkg_origin_domain "${WWW_PKG_ORIGIN_DOMAIN}" \
      --arg pkg_origin_header_name "${WWW_PKG_ORIGIN_HEADER_NAME}" \
      --arg pkg_origin_header_value "${WWW_PKG_ORIGIN_HEADER_VALUE}" \
      --arg oac_id "${oac_id}" \
      --arg function_arn "${function_arn}" \
      --arg response_headers_policy_id "${response_headers_policy_id}" \
      --arg cache_policy_id "${cache_policy_id}" \
      --arg pkg_cache_policy_id "${pkg_cache_policy_id}" \
      --arg pkg_search_cache_policy_id "${pkg_search_cache_policy_id}" \
      --arg cert_arn "${WWW_CERTIFICATE_ARN}" \
      --arg domain_a "${WWW_DOMAIN}" \
      --arg domain_b "${WWW_WWW_DOMAIN}" \
      --arg price_class "${WWW_CLOUDFRONT_PRICE_CLASS}" \
      '
      def behavior($pattern; $policy):
        {
          PathPattern: $pattern,
          TargetOriginId: $pkg_origin_id,
          ViewerProtocolPolicy: "allow-all",
          AllowedMethods: {
            Quantity: 2,
            Items: ["HEAD", "GET"],
            CachedMethods: {
              Quantity: 2,
              Items: ["HEAD", "GET"]
            }
          },
          Compress: true,
          SmoothStreaming: false,
          CachePolicyId: $policy,
          ResponseHeadersPolicyId: $response_headers_policy_id,
          TrustedSigners: {
            Enabled: false,
            Quantity: 0
          },
          TrustedKeyGroups: {
            Enabled: false,
            Quantity: 0
          },
          LambdaFunctionAssociations: {
            Quantity: 0
          },
          FunctionAssociations: {
            Quantity: 1,
            Items: [{
              EventType: "viewer-request",
              FunctionARN: $function_arn
            }]
          },
          FieldLevelEncryptionId: ""
        };
      .DistributionConfig.Comment = $comment
      | .DistributionConfig.DefaultRootObject = "index.html"
      | .DistributionConfig.Enabled = true
      | .DistributionConfig.PriceClass = $price_class
      | .DistributionConfig.Origins.Quantity = 2
      | .DistributionConfig.Origins.Items = [(
          .DistributionConfig.Origins.Items[0]
          | .Id = $origin_id
          | .DomainName = $domain_name
          | .OriginPath = ""
          | .OriginAccessControlId = $oac_id
          | .S3OriginConfig = ((.S3OriginConfig // {}) + {OriginAccessIdentity: ""})
        ), {
          Id: $pkg_origin_id,
          DomainName: $pkg_origin_domain,
          OriginPath: "",
          CustomHeaders: {
            Quantity: 1,
            Items: [{
              HeaderName: $pkg_origin_header_name,
              HeaderValue: $pkg_origin_header_value
            }]
          },
          CustomOriginConfig: {
            HTTPPort: 80,
            HTTPSPort: 443,
            OriginProtocolPolicy: "https-only",
            OriginSslProtocols: {
              Quantity: 1,
              Items: ["TLSv1.2"]
            },
            OriginReadTimeout: 30,
            OriginKeepaliveTimeout: 5
          }
        }]
      | .DistributionConfig.Aliases = {
          Quantity: 2,
          Items: [$domain_a, $domain_b]
        }
      | .DistributionConfig.DefaultCacheBehavior.TargetOriginId = $origin_id
      | .DistributionConfig.DefaultCacheBehavior.ViewerProtocolPolicy = "allow-all"
      | .DistributionConfig.DefaultCacheBehavior.AllowedMethods = {
          Quantity: 2,
          Items: ["HEAD", "GET"],
          CachedMethods: {
            Quantity: 2,
            Items: ["HEAD", "GET"]
          }
        }
      | .DistributionConfig.DefaultCacheBehavior.Compress = true
      | .DistributionConfig.DefaultCacheBehavior.SmoothStreaming = false
      | .DistributionConfig.DefaultCacheBehavior.CachePolicyId = $cache_policy_id
      | .DistributionConfig.DefaultCacheBehavior.ResponseHeadersPolicyId = $response_headers_policy_id
      | .DistributionConfig.DefaultCacheBehavior.TrustedSigners = {
          Enabled: false,
          Quantity: 0
        }
      | .DistributionConfig.DefaultCacheBehavior.TrustedKeyGroups = {
          Enabled: false,
          Quantity: 0
        }
      | .DistributionConfig.DefaultCacheBehavior.LambdaFunctionAssociations = {
          Quantity: 0
        }
      | .DistributionConfig.DefaultCacheBehavior.FunctionAssociations = {
          Quantity: 1,
          Items: [{
            EventType: "viewer-request",
            FunctionARN: $function_arn
          }]
        }
      | .DistributionConfig.DefaultCacheBehavior.FieldLevelEncryptionId = ""
      | del(
          .DistributionConfig.DefaultCacheBehavior.ForwardedValues,
          .DistributionConfig.DefaultCacheBehavior.MinTTL,
          .DistributionConfig.DefaultCacheBehavior.DefaultTTL,
          .DistributionConfig.DefaultCacheBehavior.MaxTTL
        )
      | .DistributionConfig.CacheBehaviors = {
          Quantity: 7,
          Items: [
            behavior("pkg/search.json"; $pkg_search_cache_policy_id),
            behavior("*/pkg/search.json"; $pkg_search_cache_policy_id),
            behavior("pkg*"; $pkg_cache_policy_id),
            behavior("de/pkg*"; $pkg_cache_policy_id),
            behavior("fr/pkg*"; $pkg_cache_policy_id),
            behavior("ja/pkg*"; $pkg_cache_policy_id),
            behavior("zh-hans/pkg*"; $pkg_cache_policy_id)
          ]
        }
      | .DistributionConfig.CustomErrorResponses = {
          Quantity: 1,
          Items: [{
            ErrorCode: 403,
            ResponsePagePath: "/404.html",
            ResponseCode: "404",
            ErrorCachingMinTTL: 60
          }]
        }
      | .DistributionConfig.ViewerCertificate = {
          ACMCertificateArn: $cert_arn,
          SSLSupportMethod: "sni-only",
          MinimumProtocolVersion: "TLSv1.2_2021",
          Certificate: $cert_arn,
          CertificateSource: "acm"
        }
      ' "${response_file}" | jq '.DistributionConfig' >"${config_file}"

    if ! aws cloudfront update-distribution \
      --id "${distribution_id}" \
      --if-match "${etag}" \
      --distribution-config "file://${config_file}" \
      --query 'Distribution.Id' \
      --output text; then
      return 1
    fi
    return 0
  fi

  log "  Creating distribution for ${WWW_DOMAIN}, ${WWW_WWW_DOMAIN}"
  build_distribution_config "${oac_id}" "${function_arn}" "${response_headers_policy_id}" "${cache_policy_id}" "${pkg_cache_policy_id}" "${pkg_search_cache_policy_id}" "${config_file}"
  aws cloudfront create-distribution \
    --distribution-config "file://${config_file}" \
    --query 'Distribution.Id' \
    --output text
}

put_bucket_policy() {
  local distribution_id="$1"
  local policy_file
  log_step "Restricting bucket reads to CloudFront"
  policy_file="$(mktemp)"

  jq -n \
    --arg bucket "${WWW_BUCKET}" \
    --arg account_id "${AWS_ACCOUNT_ID}" \
    --arg distribution_id "${distribution_id}" \
    '{
      Version: "2012-10-17",
      Statement: [{
        Sid: "AllowCloudFrontServicePrincipalReadOnly",
        Effect: "Allow",
        Principal: {
          Service: "cloudfront.amazonaws.com"
        },
        Action: "s3:GetObject",
        Resource: ("arn:aws:s3:::" + $bucket + "/*"),
        Condition: {
          StringEquals: {
            "AWS:SourceArn": ("arn:aws:cloudfront::" + $account_id + ":distribution/" + $distribution_id)
          }
        }
      }]
    }' >"${policy_file}"

  aws s3api put-bucket-policy \
    --bucket "${WWW_BUCKET}" \
    --policy "file://${policy_file}"
  log_ok "Bucket policy applied"
}

sync_site() {
  local upload_site_dir="${prepared_site_dir:-${site_dir}}"

  if [[ "${static_only}" != true ]]; then
    ensure_package_origin_prefixes_absent
  fi

  log_step "Syncing static assets"
  aws s3 sync "${upload_site_dir}/" "s3://${WWW_BUCKET}/" \
    --delete \
    --exclude "AGENTS.md" \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "Automic Vault.dmg" \
    --exclude "scanner.gz" \
    --exclude "scanner.sh" \
    --exclude "db.json" \
    --exclude "pkg/*" \
    --exclude "*/pkg/*" \
    --exclude "pagefind/*" \
    --exclude "*.html" \
    --exclude "*.xml" \
    --exclude "*.txt" \
    --exclude "*.md" \
    --exclude "*.json" \
    --cache-control "${WWW_ASSET_CACHE_CONTROL}"

  log_step "Syncing crawlable HTML and XML content"
  aws s3 sync "${upload_site_dir}/" "s3://${WWW_BUCKET}/" \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "*" \
    --include "*.html" \
    --include "*.xml" \
    --exclude "AGENTS.md" \
    --exclude "pkg/*" \
    --exclude "*/pkg/*" \
    --exclude "pagefind/*" \
    --cache-control "${WWW_HTML_CACHE_CONTROL}"

  log_step "Syncing crawlable plain text content"
  aws s3 sync "${upload_site_dir}/" "s3://${WWW_BUCKET}/" \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "*" \
    --include "*.txt" \
    --exclude "AGENTS.md" \
    --exclude "pkg/*" \
    --exclude "*/pkg/*" \
    --exclude "pagefind/*" \
    --content-type "text/plain; charset=utf-8" \
    --cache-control "${WWW_HTML_CACHE_CONTROL}"

  log_step "Syncing crawlable markdown content"
  aws s3 sync "${upload_site_dir}/" "s3://${WWW_BUCKET}/" \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "*" \
    --include "*.md" \
    --exclude "AGENTS.md" \
    --exclude "pkg/*" \
    --exclude "*/pkg/*" \
    --exclude "pagefind/*" \
    --content-type "text/markdown; charset=utf-8" \
    --cache-control "${WWW_HTML_CACHE_CONTROL}"

  log_step "Syncing crawlable JSON content"
  aws s3 sync "${upload_site_dir}/" "s3://${WWW_BUCKET}/" \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "*" \
    --include "*.json" \
    --exclude "AGENTS.md" \
    --exclude "pkg/*" \
    --exclude "*/pkg/*" \
    --exclude "pagefind/*" \
    --content-type "application/json; charset=utf-8" \
    --cache-control "${WWW_HTML_CACHE_CONTROL}"

  log_step "Syncing scanner shell entrypoint"
  if [[ ! -f "${upload_site_dir}/scanner.sh" ]]; then
    die "Missing scanner shell entrypoint: ${upload_site_dir}/scanner.sh"
  fi
  aws s3 cp "${upload_site_dir}/scanner.sh" "s3://${WWW_BUCKET}/scanner.sh" \
    --content-type "text/x-shellscript; charset=utf-8" \
    --cache-control "${WWW_HTML_CACHE_CONTROL}"

  log_step "Removing repo-local guidance from S3"
  aws s3 rm "s3://${WWW_BUCKET}/AGENTS.md"

  log_ok "S3 content synced"
}

ensure_package_origin_prefixes_absent() {
  log_step "Ensuring package-origin prefixes are absent from S3"
  aws s3 rm "s3://${WWW_BUCKET}/pkg/" --recursive >/dev/null 2>&1 || true
  aws s3 rm "s3://${WWW_BUCKET}/de/pkg/" --recursive >/dev/null 2>&1 || true
  aws s3 rm "s3://${WWW_BUCKET}/fr/pkg/" --recursive >/dev/null 2>&1 || true
  aws s3 rm "s3://${WWW_BUCKET}/ja/pkg/" --recursive >/dev/null 2>&1 || true
  aws s3 rm "s3://${WWW_BUCKET}/zh-hans/pkg/" --recursive >/dev/null 2>&1 || true
  aws s3 rm "s3://${WWW_BUCKET}/pagefind/" --recursive >/dev/null 2>&1 || true
  log_ok "Package-origin prefixes are absent from S3"
}

ensure_certificate_issued() {
  local status
  log_step "Checking ACM certificate"
  status="$(
    aws acm describe-certificate \
      --region us-east-1 \
      --certificate-arn "${WWW_CERTIFICATE_ARN}" \
      --query 'Certificate.Status' \
      --output text
  )"

  if [[ "${status}" != "ISSUED" ]]; then
    die "Certificate is not issued: ${status}"
  fi
  log_ok "Certificate is issued"
}

log_header
assert_www_i18n_current
prepare_site_for_upload
if [[ "${static_only}" == true ]]; then
  log_step "Checking S3 bucket"
  aws s3api head-bucket --bucket "${WWW_BUCKET}" >/dev/null
  log_ok "S3 bucket exists"
  sync_site

  if [[ "${use_color}" == true ]]; then
    cat <<EOF

${green}${glyph_ok}${reset} ${bold}Static deployment complete${reset}
  Bucket                      ${WWW_BUCKET}
  CloudFront configuration    skipped
EOF
  else
    cat <<EOF

Static deployment complete.
Bucket: ${WWW_BUCKET}
CloudFront configuration: skipped
EOF
  fi
  exit 0
fi
ensure_bucket
oac_id="$(ensure_oac)"
ensure_redirect_function
response_headers_policy_id="$(ensure_response_headers_policy)"
cache_policy_id="$(ensure_cache_policy)"
pkg_cache_policy_id="$(ensure_pkg_cache_policy)"
pkg_search_cache_policy_id="$(ensure_pkg_search_cache_policy)"
log_step "Reading CloudFront function ARN"
function_arn="$(
  aws cloudfront describe-function \
    --name "${redirect_function_name}" \
    --stage LIVE \
    --query 'FunctionSummary.FunctionMetadata.FunctionARN' \
    --output text
)"
log_ok "Function ARN resolved"
ensure_certificate_issued
distribution_id="$(upsert_distribution "${oac_id}" "${function_arn}" "${response_headers_policy_id}" "${cache_policy_id}" "${pkg_cache_policy_id}" "${pkg_search_cache_policy_id}")"
put_bucket_policy "${distribution_id}"
log_step "Waiting for CloudFront deployment"
aws cloudfront wait distribution-deployed --id "${distribution_id}"
log_ok "Distribution deployed"
sync_site
if [[ "${WWW_EMERGENCY_INVALIDATE}" == "true" ]]; then
  log_step "Submitting emergency CloudFront invalidation"
  aws cloudfront create-invalidation \
    --distribution-id "${distribution_id}" \
    --paths '/*' >/dev/null
  log_ok "Emergency invalidation submitted"
else
  log_ok "Skipped CloudFront invalidation; package origin cache refreshes daily"
fi

distribution_domain="$(
  aws cloudfront get-distribution \
    --id "${distribution_id}" \
    --query 'Distribution.DomainName' \
    --output text
)"

if [[ "${use_color}" == true ]]; then
  cat <<EOF

${green}${glyph_ok}${reset} ${bold}Deployment complete${reset}
  CloudFront distribution ID  ${distribution_id}
  CloudFront domain           ${distribution_domain}
  Bucket                      ${WWW_BUCKET}
  Aliases                     ${WWW_DOMAIN}, ${WWW_WWW_DOMAIN}
EOF
else
  cat <<EOF

Deployment complete.
CloudFront distribution ID: ${distribution_id}
CloudFront domain: ${distribution_domain}
Bucket: ${WWW_BUCKET}
Aliases: ${WWW_DOMAIN}, ${WWW_WWW_DOMAIN}
EOF
fi
