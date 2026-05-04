const toggle = document.querySelector(".nav-toggle");
const nav = document.querySelector(".nav");
const revealTargets = document.querySelectorAll(
  ".status-strip, .feature-section, .friction"
);

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
        rootMargin: "0px 0px -16% 0px",
        threshold: 0.18,
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
