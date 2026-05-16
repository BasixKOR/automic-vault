const toggle = document.querySelector(".nav-toggle");
const nav = document.querySelector(".nav");
const revealTargets = document.querySelectorAll(
  ".feature-section, .highlight-card, .final-cta"
);
const scrollMeter = document.querySelector(".scroll-meter span");

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
