const toggle = document.querySelector(".nav-toggle");
const nav = document.querySelector(".nav");
const scrollMeter = document.querySelector(".scroll-meter span");
const securedFeed = document.querySelector("[data-secured-feed]");

const securedPackages = [
  ["gh", "GitHub tokens saved in Keychain and injected only for gh commands", "accent-green", "/pkg/brew/gh/"],
  ["awscli", "AWS keys moved from ~/.aws/credentials to credential_process", "accent-green", "/pkg/brew/awscli/"],
  ["curl", "netrc and curlrc credentials detected as hazards", "accent-hot", "/pkg/brew/curl/"],
  ["git", "plaintext credential-store files flagged before agent runs", "accent-gold", "/pkg/brew/git/"],
  ["npm", "registry tokens mounted through a temporary npm config", "accent-blue", "/pkg/brew/node/"],
  ["docker", "ambient registry credential helpers flagged as hazards", "accent-hot", "/pkg/brew/docker/"],
  ["terraform", "cloud tokens served through Terraform helper flow", "accent-blue", "/pkg/brew/tfenv/"],
  ["openssh", "unencrypted private keys reported before agent runs", "accent-hot", "/pkg/brew/openssh/"],
  ["kubectl", "kubeconfig credentials served through exec helpers", "accent-blue", "/pkg/brew/kubernetes-cli/"],
  ["bitwarden", "token-bearing app state moved into Keychain", "accent-green", "/pkg/brew/bitwarden-cli/"],
  ["heroku", "API token injected only for Heroku CLI execution", "accent-gold", "/pkg/brew/heroku/"],
  ["firebase", "refresh token isolated behind a temporary config home", "accent-hot", "/pkg/brew/firebase-cli/"],
  ["pulumi", "cloud credentials injected through a temporary path", "accent-blue", "/pkg/brew/pulumi/"],
  ["rclone", "remote credentials mounted only while rclone runs", "accent-green", "/pkg/brew/rclone/"],
  ["sentry-cli", "auth token hidden outside Sentry CLI execution", "accent-gold", "/pkg/brew/sentry-cli/"],
  ["snyk", "API token kept out of configstore plaintext", "accent-hot", "/pkg/brew/snyk-cli/"],
  ["uv", "package index credentials detected and isolated", "accent-blue", "/pkg/brew/uv/"],
  ["opentofu", "registry tokens served through Terraform helper flow", "accent-green", "/pkg/brew/opentofu/"],
  ["oci-cli", "OCI config and key files injected at runtime", "accent-gold", "/pkg/brew/oci-cli/"],
  ["snowflake", "connection passwords moved out of local config", "accent-hot", "/pkg/brew/snowflake-cli/"],
  ["jfrog", "server credentials mounted only for jfrog commands", "accent-blue", "/pkg/brew/jfrog-cli/"],
  ["doctl", "DigitalOcean tokens isolated from config.yaml", "accent-green", "/pkg/brew/doctl/"],
  ["glab", "GitLab tokens exposed only through GLAB_CONFIG_DIR", "accent-gold", "/pkg/brew/glab/"],
  ["helm", "chart repository credentials held in Keychain", "accent-hot", "/pkg/brew/helm/"],
  ["podman", "registry auth served through a temporary helper shim", "accent-blue", "/pkg/brew/podman/"],
  ["curl", "netrc and curlrc credentials detected as hazards", "accent-green", "/pkg/brew/curl/"],
  ["ruby", "RubyGems API keys detected before agent runs", "accent-gold", "/pkg/brew/ruby/"],
  ["netlify", "API tokens restored into a temporary home", "accent-green", "/pkg/brew/netlify-cli/"],
  ["minio-mc", "S3 alias secrets scoped to mc execution", "accent-gold", "/pkg/brew/minio-mc/"],
];

if (toggle && nav) {
  toggle.addEventListener("click", () => {
    const isOpen = toggle.getAttribute("aria-expanded") === "true";
    toggle.setAttribute("aria-expanded", String(!isOpen));
    nav.classList.toggle("is-open", !isOpen);
  });

  nav.addEventListener("click", (event) => {
    if (event.target instanceof HTMLAnchorElement) {
      toggle.setAttribute("aria-expanded", "false");
      nav.classList.remove("is-open");
    }
  });
}

if (scrollMeter) {
  let scrollMeterFrame = 0;

  const updateScrollMeter = () => {
    scrollMeterFrame = 0;
    const scrollable = document.documentElement.scrollHeight - window.innerHeight;
    const progress = scrollable > 0 ? window.scrollY / scrollable : 0;
    scrollMeter.style.transform = `scaleX(${Math.min(1, Math.max(0, progress))})`;
  };

  const scheduleScrollMeterUpdate = () => {
    if (scrollMeterFrame) {
      return;
    }

    scrollMeterFrame = window.requestAnimationFrame(updateScrollMeter);
  };

  updateScrollMeter();
  window.addEventListener("scroll", scheduleScrollMeterUpdate, { passive: true });
  window.addEventListener("resize", scheduleScrollMeterUpdate);
}

if (securedFeed) {
  const motionAllowed = window.matchMedia("(prefers-reduced-motion: no-preference)");
  const rows = Array.from(securedFeed.querySelectorAll(".feed-row"));
  const swapDuration = 360;
  const litDuration = 2080;
  const swapInterval = 3280;
  let cursor = rows.length;
  let rowCursor = 0;

  if (motionAllowed.matches && rows.length > 0) {
    window.setInterval(() => {
      const visibleRows = rows.filter((row) => getComputedStyle(row).display !== "none");

      if (visibleRows.length === 0) {
        return;
      }

      const row = visibleRows[rowCursor % visibleRows.length];
      const next = securedPackages[cursor % securedPackages.length];
      rowCursor += 1;
      cursor += 1;

      row.classList.add("is-swapping");
      row.classList.remove("is-lit");

      window.setTimeout(() => {
        const [name, detail, accent, href] = next;
        const label = row.querySelector("span");
        const text = row.querySelector("p");

        row.setAttribute("href", href);
        row.setAttribute("aria-label", `${name}: ${detail}`);

        if (label) {
          label.textContent = name;
        }

        if (text) {
          text.textContent = detail;
        }

        row.className = `feed-row ${accent} is-entering`;
        window.requestAnimationFrame(() => {
          row.classList.remove("is-entering");
          row.classList.add("is-lit");

          window.setTimeout(() => {
            row.classList.remove("is-lit");
          }, litDuration);
        });
      }, swapDuration);
    }, swapInterval);
  }
}
