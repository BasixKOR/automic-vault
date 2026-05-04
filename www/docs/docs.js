const copyButtons = document.querySelectorAll("[data-copy]");

copyButtons.forEach((button) => {
  button.addEventListener("click", async () => {
    const scope = button.closest("[data-copy-scope]");
    const code = scope?.querySelector("code");

    if (!code) {
      button.textContent = "Error";
      return;
    }

    try {
      await navigator.clipboard.writeText(code.textContent.trim());
      button.textContent = "Copied";
      button.classList.add("is-copied");

      window.setTimeout(() => {
        button.textContent = "Copy";
        button.classList.remove("is-copied");
      }, 1800);
    } catch {
      button.textContent = "Copy failed";
      window.setTimeout(() => {
        button.textContent = "Copy";
      }, 1800);
    }
  });
});
