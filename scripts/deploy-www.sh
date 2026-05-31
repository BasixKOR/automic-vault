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
  log "${dim}Static site -> S3 -> CloudFront${reset}"
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

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    die "Set ${name} in .envrc."
  fi
}

for tool in aws jq node; do
  command -v "$tool" >/dev/null 2>&1 || {
    die "Missing required tool: ${tool}."
  }
done

for env_name in \
  AWS_REGION \
  WWW_DOMAIN \
  WWW_WWW_DOMAIN \
  WWW_CANONICAL_HOST \
  WWW_BUCKET \
  WWW_CERTIFICATE_ARN \
  WWW_CLOUDFRONT_PRICE_CLASS \
  WWW_HTML_CACHE_CONTROL \
  WWW_ASSET_CACHE_CONTROL
do
  require_env "${env_name}"
done

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
site_dir="${repo_root}/www"
llms_full_generator="${repo_root}/scripts/generate-llms-full.mjs"
package_pages_generator="${repo_root}/scripts/generate-pkg-pages.py"
package_page_enrichment_generator="${repo_root}/scripts/generate-pkg-page-enrichment.py"
package_version_freshness_generator="${repo_root}/scripts/generate-pkg-version-freshness.py"
package_manager_indexes_generator="${repo_root}/scripts/generate-pkg-manager-indexes.py"
package_cross_ecosystem_generator="${repo_root}/scripts/generate-pkg-cross-ecosystem.py"
package_graph_curation_generator="${repo_root}/scripts/generate-pkg-graph-curation.py"
package_graph_generator="${repo_root}/scripts/generate-pkg-graph.py"
search_index_generator="${repo_root}/scripts/generate-search-index.py"
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
distribution_comment="${WWW_DOMAIN} static site"
oac_name="${WWW_DOMAIN}-s3-oac"
redirect_function_name="${WWW_DOMAIN//./-}-redirect-to-canonical"
response_headers_policy_name="${WWW_DOMAIN//./-}-security-headers"
cache_policy_name="${WWW_DOMAIN//./-}-brotli-cache"

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
  local product_version secured_package_count secured_package_display_count index_path
  log_step "Preparing deploy-time site content"
  product_version="$(read_product_version)"
  secured_package_count="$(count_scan_log_entries)"
  secured_package_display_count="$(format_count_for_display "${secured_package_count}")"
  prepared_site_dir="$(mktemp -d)"
  rsync -a \
    --exclude '/pkg/' \
    --exclude '/*/pkg/' \
    "${site_dir}/" "${prepared_site_dir}/"
  stamp_product_version "${product_version}"

  index_path="${prepared_site_dir}/index.html"
  if [[ ! -f "${index_path}" ]]; then
    die "Missing prepared index: ${index_path}"
  fi

  SECURED_PACKAGE_COUNT="${secured_package_display_count}" perl -0pi -e '
    BEGIN {
      $count = $ENV{"SECURED_PACKAGE_COUNT"};
      $matches = 0;
    }
    $matches += s{(<small\b[^>]*\bdata-secured-package-count\b[^>]*>)[^<]*(</small>)}{$1$count$2}g;
    END {
      die "Expected exactly one secured package count replacement, got $matches\n"
        unless $matches == 1;
    }
  ' "${index_path}"

  node "${llms_full_generator}" "${prepared_site_dir}" "${prepared_site_dir}/llms-full.txt"
  stamp_product_version "${product_version}"

  log_ok "Stamped Automic Vault ${product_version}"
  log_ok "Stamped ${secured_package_display_count} secured packages"
}

assert_package_pages_current() {
  log_step "Checking package page enrichment"
  if [[ ! -x "${package_page_enrichment_generator}" && ! -f "${package_page_enrichment_generator}" ]]; then
    die "Missing package page enrichment generator: ${package_page_enrichment_generator}"
  fi
  python3 "${package_page_enrichment_generator}" --check

  log_step "Checking package version freshness"
  if [[ ! -x "${package_version_freshness_generator}" && ! -f "${package_version_freshness_generator}" ]]; then
    die "Missing package version freshness generator: ${package_version_freshness_generator}"
  fi
  python3 "${package_version_freshness_generator}" --check

  log_step "Checking package manager indexes"
  if [[ ! -x "${package_manager_indexes_generator}" && ! -f "${package_manager_indexes_generator}" ]]; then
    die "Missing package manager index generator: ${package_manager_indexes_generator}"
  fi
  python3 "${package_manager_indexes_generator}" --check

  log_step "Checking package cross-ecosystem install commands"
  if [[ ! -x "${package_cross_ecosystem_generator}" && ! -f "${package_cross_ecosystem_generator}" ]]; then
    die "Missing package cross-ecosystem generator: ${package_cross_ecosystem_generator}"
  fi
  python3 "${package_cross_ecosystem_generator}" --check

  log_step "Checking package relationship graph"
  if [[ ! -x "${package_graph_curation_generator}" && ! -f "${package_graph_curation_generator}" ]]; then
    die "Missing package graph curation generator: ${package_graph_curation_generator}"
  fi
  python3 "${package_graph_curation_generator}" --check

  if [[ ! -x "${package_graph_generator}" && ! -f "${package_graph_generator}" ]]; then
    die "Missing package graph generator: ${package_graph_generator}"
  fi
  python3 "${package_graph_generator}" --check

  log_step "Checking generated package SEO pages"
  if [[ ! -x "${package_pages_generator}" && ! -f "${package_pages_generator}" ]]; then
    die "Missing package page generator: ${package_pages_generator}"
  fi
  python3 "${package_pages_generator}" --check
}

assert_search_index_current() {
  log_step "Checking generated Pagefind search index"
  if [[ ! -x "${search_index_generator}" && ! -f "${search_index_generator}" ]]; then
    die "Missing search index generator: ${search_index_generator}"
  fi
  python3 "${search_index_generator}" --check
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

  if (request.uri === "/install.sh" || request.uri === "/scanner.sh" || request.uri === "/scanner.gz") {
    return request;
  }
  if (request.uri === "/av.dmg") {
    return {
      statusCode: 301,
      statusDescription: "Moved Permanently",
      headers: {
        location: { value: appendQueryString("/Automic%20Vault.dmg") }
      }
    };
  }
  if (host === "${WWW_DOMAIN}") {
    var location = appendQueryString("https://${WWW_CANONICAL_HOST}" + request.uri);
    return {
      statusCode: 301,
      statusDescription: "Moved Permanently",
      headers: {
        location: { value: location }
      }
    };
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
      Comment: "Static site cache policy with Gzip and Brotli variants",
      DefaultTTL: 86400,
      MaxTTL: 31536000,
      MinTTL: 0,
      ParametersInCacheKeyAndForwardedToOrigin: {
        EnableAcceptEncodingGzip: true,
        EnableAcceptEncodingBrotli: true,
        HeadersConfig: {
          HeaderBehavior: "none"
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
  local output_file="$5"

  jq -n \
    --arg caller_reference "${WWW_DOMAIN}-$(date +%s)" \
    --arg comment "${distribution_comment}" \
    --arg origin_id "${WWW_BUCKET}-origin" \
    --arg domain_name "${origin_domain}" \
    --arg oac_id "${oac_id}" \
    --arg function_arn "${function_arn}" \
    --arg response_headers_policy_id "${response_headers_policy_id}" \
    --arg cache_policy_id "${cache_policy_id}" \
    --arg cert_arn "${WWW_CERTIFICATE_ARN}" \
    --arg domain_a "${WWW_DOMAIN}" \
    --arg domain_b "${WWW_WWW_DOMAIN}" \
    --arg price_class "${WWW_CLOUDFRONT_PRICE_CLASS}" \
    '{
      CallerReference: $caller_reference,
      Comment: $comment,
      Enabled: true,
      DefaultRootObject: "index.html",
      Origins: {
        Quantity: 1,
        Items: [{
          Id: $origin_id,
          DomainName: $domain_name,
          OriginAccessControlId: $oac_id,
          S3OriginConfig: {
            OriginAccessIdentity: ""
          }
        }]
      },
      Aliases: {
        Quantity: 2,
        Items: [$domain_a, $domain_b]
      },
      DefaultCacheBehavior: {
        TargetOriginId: $origin_id,
        ViewerProtocolPolicy: "redirect-to-https",
        AllowedMethods: {
          Quantity: 2,
          Items: ["HEAD", "GET"],
          CachedMethods: {
            Quantity: 2,
            Items: ["HEAD", "GET"]
          }
        },
        Compress: true,
        CachePolicyId: $cache_policy_id,
        ResponseHeadersPolicyId: $response_headers_policy_id,
        FunctionAssociations: {
          Quantity: 1,
          Items: [{
            EventType: "viewer-request",
            FunctionARN: $function_arn
          }]
        },
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
      --arg oac_id "${oac_id}" \
      --arg function_arn "${function_arn}" \
      --arg response_headers_policy_id "${response_headers_policy_id}" \
      --arg cache_policy_id "${cache_policy_id}" \
      --arg cert_arn "${WWW_CERTIFICATE_ARN}" \
      --arg domain_a "${WWW_DOMAIN}" \
      --arg domain_b "${WWW_WWW_DOMAIN}" \
      --arg price_class "${WWW_CLOUDFRONT_PRICE_CLASS}" \
      '
      .DistributionConfig.Comment = $comment
      | .DistributionConfig.DefaultRootObject = "index.html"
      | .DistributionConfig.Enabled = true
      | .DistributionConfig.PriceClass = $price_class
      | .DistributionConfig.Origins.Quantity = 1
      | .DistributionConfig.Origins.Items = [(
          .DistributionConfig.Origins.Items[0]
          | .Id = $origin_id
          | .DomainName = $domain_name
          | .OriginAccessControlId = $oac_id
          | .S3OriginConfig = ((.S3OriginConfig // {}) + {OriginAccessIdentity: ""})
        )]
      | .DistributionConfig.Aliases = {
          Quantity: 2,
          Items: [$domain_a, $domain_b]
        }
      | .DistributionConfig.DefaultCacheBehavior.TargetOriginId = $origin_id
      | .DistributionConfig.DefaultCacheBehavior.ViewerProtocolPolicy = "redirect-to-https"
      | .DistributionConfig.DefaultCacheBehavior.AllowedMethods = {
          Quantity: 2,
          Items: ["HEAD", "GET"],
          CachedMethods: {
            Quantity: 2,
            Items: ["HEAD", "GET"]
          }
        }
      | .DistributionConfig.DefaultCacheBehavior.Compress = true
      | .DistributionConfig.DefaultCacheBehavior.CachePolicyId = $cache_policy_id
      | .DistributionConfig.DefaultCacheBehavior.ResponseHeadersPolicyId = $response_headers_policy_id
      | .DistributionConfig.DefaultCacheBehavior.FunctionAssociations = {
          Quantity: 1,
          Items: [{
            EventType: "viewer-request",
            FunctionARN: $function_arn
          }]
        }
      | del(
          .DistributionConfig.DefaultCacheBehavior.ForwardedValues,
          .DistributionConfig.DefaultCacheBehavior.MinTTL,
          .DistributionConfig.DefaultCacheBehavior.DefaultTTL,
          .DistributionConfig.DefaultCacheBehavior.MaxTTL
        )
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

    aws cloudfront update-distribution \
      --id "${distribution_id}" \
      --if-match "${etag}" \
      --distribution-config "file://${config_file}" \
      --query 'Distribution.Id' \
      --output text
    return 0
  fi

  log "  Creating distribution for ${WWW_DOMAIN}, ${WWW_WWW_DOMAIN}"
  build_distribution_config "${oac_id}" "${function_arn}" "${response_headers_policy_id}" "${cache_policy_id}" "${config_file}"
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

  log_step "Syncing immutable Pagefind search data"
  aws s3 sync "${upload_site_dir}/pagefind/" "s3://${WWW_BUCKET}/pagefind/" \
    --size-only \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "*" \
    --include "fragment/*.pf_fragment" \
    --include "index/*.pf_index" \
    --include "pagefind.*.pf_meta" \
    --cache-control "${WWW_ASSET_CACHE_CONTROL}"

  log_step "Syncing mutable Pagefind runtime assets"
  aws s3 sync "${upload_site_dir}/pagefind/" "s3://${WWW_BUCKET}/pagefind/" \
    --delete \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "*" \
    --include ".manifest.json" \
    --include "pagefind-entry.json" \
    --include "pagefind.js" \
    --include "pagefind-*.js" \
    --include "pagefind-*.css" \
    --include "wasm.*.pagefind" \
    --cache-control "${WWW_HTML_CACHE_CONTROL}"

  log_step "Normalizing mutable Pagefind cache headers"
  local normalized_pagefind_count=0
  local pagefind_runtime_asset pagefind_runtime_key pagefind_runtime_head pagefind_runtime_cache pagefind_runtime_type
  while IFS= read -r -d '' pagefind_runtime_asset; do
    pagefind_runtime_key="pagefind/${pagefind_runtime_asset##*/}"
    pagefind_runtime_head="$(
      aws s3api head-object \
        --bucket "${WWW_BUCKET}" \
        --key "${pagefind_runtime_key}" \
        --output json
    )"
    pagefind_runtime_cache="$(jq -r '.CacheControl // "None"' <<<"${pagefind_runtime_head}")"
    pagefind_runtime_type="$(jq -r '.ContentType // "None"' <<<"${pagefind_runtime_head}")"
    if [[ "${pagefind_runtime_cache}" == "${WWW_HTML_CACHE_CONTROL}" ]]; then
      continue
    fi

    local copy_args=(
      s3api copy-object
      --bucket "${WWW_BUCKET}"
      --key "${pagefind_runtime_key}"
      --copy-source "${WWW_BUCKET}/${pagefind_runtime_key}"
      --metadata-directive REPLACE
      --cache-control "${WWW_HTML_CACHE_CONTROL}"
    )
    if [[ -n "${pagefind_runtime_type}" && "${pagefind_runtime_type}" != "None" ]]; then
      copy_args+=(--content-type "${pagefind_runtime_type}")
    fi
    aws "${copy_args[@]}" >/dev/null
    normalized_pagefind_count="$((normalized_pagefind_count + 1))"
  done < <(
    find "${upload_site_dir}/pagefind" -maxdepth 1 -type f \( \
      -name ".manifest.json" -o \
      -name "pagefind-entry.json" -o \
      -name "pagefind.js" -o \
      -name "pagefind-*.js" -o \
      -name "pagefind-*.css" -o \
      -name "wasm.*.pagefind" \
    \) -print0
  )
  log_ok "Normalized ${normalized_pagefind_count} mutable Pagefind headers"

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

  sync_package_pages

  log_step "Removing repo-local guidance from S3"
  aws s3 rm "s3://${WWW_BUCKET}/AGENTS.md"

  log_ok "S3 content synced"
}

sync_package_tree() {
  local source_dir="$1"
  local destination_prefix="$2"

  if [[ ! -d "${source_dir}" ]]; then
    die "Missing generated package page directory: ${source_dir}"
  fi

  log "  ${destination_prefix}/ from ${source_dir}"

  aws s3 sync "${source_dir}/" "s3://${WWW_BUCKET}/${destination_prefix}/" \
    --delete \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --cache-control "${WWW_HTML_CACHE_CONTROL}"
}

sync_package_pages() {
  local locale_dir locale_slug

  log_step "Syncing generated package pages from www"
  sync_package_tree "${site_dir}/pkg" "pkg"

  while IFS= read -r -d '' locale_dir; do
    locale_slug="${locale_dir#"${site_dir}/"}"
    locale_slug="${locale_slug%/pkg}"
    sync_package_tree "${locale_dir}" "${locale_slug}/pkg"
  done < <(
    find "${site_dir}" -mindepth 2 -maxdepth 2 -type d -path "${site_dir}/*/pkg" -print0
  )
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
assert_package_pages_current
assert_search_index_current
prepare_site_for_upload
ensure_bucket
oac_id="$(ensure_oac)"
ensure_redirect_function
response_headers_policy_id="$(ensure_response_headers_policy)"
cache_policy_id="$(ensure_cache_policy)"
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
distribution_id="$(upsert_distribution "${oac_id}" "${function_arn}" "${response_headers_policy_id}" "${cache_policy_id}")"
put_bucket_policy "${distribution_id}"
log_step "Waiting for CloudFront deployment"
aws cloudfront wait distribution-deployed --id "${distribution_id}"
log_ok "Distribution deployed"
sync_site
log_step "Invalidating CloudFront cache"
aws cloudfront create-invalidation \
  --distribution-id "${distribution_id}" \
  --paths '/*' >/dev/null
log_ok "Invalidation submitted"

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
