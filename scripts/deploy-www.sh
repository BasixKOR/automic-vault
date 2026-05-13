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

for tool in aws jq; do
  command -v "$tool" >/dev/null 2>&1 || {
    die "Missing required tool: ${tool}."
  }
done

for env_name in \
  AWS_REGION \
  AWS_ACCOUNT_ID \
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
db_source="${repo_root}/data/combined.json"
db_cache_control="public, max-age=3600"
scan_log_source="${repo_root}/data/radioisotopes/SCAN_LOG.md"
prepared_site_dir=""

if [[ ! -d "${site_dir}" ]]; then
  die "Missing site directory: ${site_dir}"
fi

if [[ ! -f "${db_source}" ]]; then
  die "Missing database source: ${db_source}"
fi

origin_domain="${WWW_BUCKET}.s3.${AWS_REGION}.amazonaws.com"
distribution_comment="${WWW_DOMAIN} static site"
oac_name="${WWW_DOMAIN}-s3-oac"
redirect_function_name="${WWW_DOMAIN//./-}-redirect-to-canonical"

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

prepare_site_for_upload() {
  local secured_package_count index_path
  log_step "Preparing deploy-time site content"
  secured_package_count="$(count_scan_log_entries)"
  prepared_site_dir="$(mktemp -d)"
  cp -R "${site_dir}/." "${prepared_site_dir}/"

  index_path="${prepared_site_dir}/index.html"
  if [[ ! -f "${index_path}" ]]; then
    die "Missing prepared index: ${index_path}"
  fi

  SECURED_PACKAGE_LABEL="${secured_package_count} Packages" perl -0pi -e '
    BEGIN {
      $label = $ENV{"SECURED_PACKAGE_LABEL"};
      $matches = 0;
    }
    $matches += s{<small>Packages</small>}{<small>$label</small>}g;
    END {
      die "Expected exactly one Packages status label replacement, got $matches\n"
        unless $matches == 1;
    }
  ' "${index_path}"

  log_ok "Stamped ${secured_package_count} secured packages"
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

  if (request.uri === "/install.sh") {
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
  local output_file="$3"

  jq -n \
    --arg caller_reference "${WWW_DOMAIN}-$(date +%s)" \
    --arg comment "${distribution_comment}" \
    --arg origin_id "${WWW_BUCKET}-origin" \
    --arg domain_name "${origin_domain}" \
    --arg oac_id "${oac_id}" \
    --arg function_arn "${function_arn}" \
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
        FunctionAssociations: {
          Quantity: 1,
          Items: [{
            EventType: "viewer-request",
            FunctionARN: $function_arn
          }]
        },
        ForwardedValues: {
          QueryString: false,
          Cookies: {
            Forward: "none"
          },
          Headers: {
            Quantity: 0
          },
          QueryStringCacheKeys: {
            Quantity: 0
          }
        },
        MinTTL: 0,
        DefaultTTL: 86400,
        MaxTTL: 31536000
      },
      CustomErrorResponses: {
        Quantity: 0
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
      | .DistributionConfig.DefaultCacheBehavior.FunctionAssociations = {
          Quantity: 1,
          Items: [{
            EventType: "viewer-request",
            FunctionARN: $function_arn
          }]
        }
      | .DistributionConfig.DefaultCacheBehavior.ForwardedValues = {
          QueryString: false,
          Cookies: {Forward: "none"},
          Headers: {Quantity: 0},
          QueryStringCacheKeys: {Quantity: 0}
        }
      | .DistributionConfig.DefaultCacheBehavior.MinTTL = 0
      | .DistributionConfig.DefaultCacheBehavior.DefaultTTL = 86400
      | .DistributionConfig.DefaultCacheBehavior.MaxTTL = 31536000
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
  build_distribution_config "${oac_id}" "${function_arn}" "${config_file}"
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
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "Automic Vault.dmg" \
    --exclude "db.json" \
    --exclude "*.html" \
    --exclude "*.xml" \
    --cache-control "${WWW_ASSET_CACHE_CONTROL}"

  log_step "Syncing HTML and XML"
  aws s3 sync "${upload_site_dir}/" "s3://${WWW_BUCKET}/" \
    --exclude ".DS_Store" \
    --exclude "*/.DS_Store" \
    --exclude "*" \
    --include "*.html" \
    --include "*.xml" \
    --cache-control "${WWW_HTML_CACHE_CONTROL}"

  log_ok "S3 content synced"
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
prepare_site_for_upload
ensure_bucket
oac_id="$(ensure_oac)"
ensure_redirect_function
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
distribution_id="$(upsert_distribution "${oac_id}" "${function_arn}")"
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
