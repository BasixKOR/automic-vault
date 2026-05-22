const toggle = document.querySelector(".nav-toggle");
const nav = document.querySelector(".nav");
const revealTargets = document.querySelectorAll(
  ".feature-section, .highlight-card, .final-cta"
);
const scrollMeter = document.querySelector(".scroll-meter span");
const securedFeed = document.querySelector("[data-secured-feed]");

const securedPackages = [
  ["docker", "ambient registry credential helpers flagged as hazards", "accent-hot"],
  ["aws-cli", "AWS credentials served through av credential-helper", "accent-green"],
  ["terraform", "cloud tokens served through Terraform's helper protocol", "accent-blue"],
  ["git", "plaintext credential-store files detected in the GUI", "accent-gold"],
  ["openssh", "unencrypted private keys reported before agent runs", "accent-hot"],
  ["kubectl", "kubeconfig credentials served through exec helpers", "accent-blue"],
  ["bitwarden", "token-bearing app state moved into Keychain", "accent-green"],
  ["heroku", "API token injected only for Heroku CLI execution", "accent-gold"],
  ["firebase", "refresh token isolated behind a temporary config home", "accent-hot"],
  ["pulumi", "cloud credentials injected through a temporary path", "accent-blue"],
  ["rclone", "remote credentials mounted only while rclone runs", "accent-green"],
  ["sentry-cli", "auth token hidden outside Sentry CLI execution", "accent-gold"],
  ["snyk", "API token kept out of configstore plaintext", "accent-hot"],
  ["uv", "package index credentials detected and isolated", "accent-blue"],
  ["opentofu", "registry tokens served through Terraform helper flow", "accent-green"],
  ["oci-cli", "OCI config and key files injected at runtime", "accent-gold"],
  ["snowflake", "connection passwords moved out of local config", "accent-hot"],
  ["jfrog", "server credentials mounted only for jfrog commands", "accent-blue"],
  ["doctl", "DigitalOcean tokens isolated from config.yaml", "accent-green"],
  ["glab", "GitLab tokens exposed only through GLAB_CONFIG_DIR", "accent-gold"],
  ["helm", "chart repository credentials held in Keychain", "accent-hot"],
  ["podman", "registry auth served through a temporary helper shim", "accent-blue"],
  ["curl", "netrc and curlrc credentials detected as hazards", "accent-green"],
  ["ruby", "RubyGems API keys detected before agent runs", "accent-gold"],
  ["netlify", "API tokens restored into a temporary home", "accent-green"],
  ["minio-mc", "S3 alias secrets scoped to mc execution", "accent-gold"],
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
  const updateScrollMeter = () => {
    const scrollable = document.documentElement.scrollHeight - window.innerHeight;
    const progress = scrollable > 0 ? window.scrollY / scrollable : 0;
    scrollMeter.style.width = `${Math.min(1, Math.max(0, progress)) * 100}%`;
  };

  updateScrollMeter();
  window.addEventListener("scroll", updateScrollMeter, { passive: true });
  window.addEventListener("resize", updateScrollMeter);
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
        const [name, detail, accent] = next;
        const label = row.querySelector("span");
        const text = row.querySelector("p");

        if (label) {
          label.textContent = name;
        }

        if (text) {
          text.textContent = detail;
        }

        row.className = `feed-row ${accent} is-entering`;
        void row.offsetWidth;
        row.classList.remove("is-entering");
        row.classList.add("is-lit");

        window.setTimeout(() => {
          row.classList.remove("is-lit");
        }, litDuration);
      }, swapDuration);
    }, swapInterval);
  }
}

if (revealTargets.length > 0) {
  const motionAllowed = window.matchMedia("(prefers-reduced-motion: no-preference)");

  if (motionAllowed.matches && "IntersectionObserver" in window) {
    document.body.classList.add("reveal-ready");

    const revealObserver = new IntersectionObserver(
      (entries, observer) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-visible");
            observer.unobserve(entry.target);
          }
        }
      },
      {
        rootMargin: "0px 0px -14% 0px",
        threshold: 0.14,
      }
    );

    for (const target of revealTargets) {
      revealObserver.observe(target);
    }
  } else {
    for (const target of revealTargets) {
      target.classList.add("is-visible");
    }
  }
}
