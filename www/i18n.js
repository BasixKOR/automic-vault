(() => {
  const locales = [{"code": "ja", "slug": "ja", "nativeName": "日本語", "languages": ["ja", "ja-jp"]}, {"code": "de", "slug": "de", "nativeName": "Deutsch", "languages": ["de", "de-at", "de-ch", "de-de"]}, {"code": "fr", "slug": "fr", "nativeName": "Français", "languages": ["fr", "fr-be", "fr-ca", "fr-ch", "fr-fr"]}, {"code": "zh-Hans", "slug": "zh-hans", "nativeName": "简体中文", "languages": ["zh", "zh-cn", "zh-hans", "zh-sg"]}];
  const dismissedKey = "av-i18n-dismissed";
  if (localStorage.getItem(dismissedKey) === "1") return;
  const path = window.location.pathname;
  if (/^\/(ja|de|fr|zh-hans)(\/|$)/.test(path)) return;
  const languages = navigator.languages || [navigator.language || ""];
  const match = languages
    .map((item) => String(item).toLowerCase())
    .map((item) => locales.find((locale) => locale.languages.includes(item) || locale.languages.includes(item.split("-")[0])))
    .find(Boolean);
  if (!match) return;
  const localized = "/" + match.slug + (path === "/" ? "/" : path);
  fetch(localized, { method: "HEAD" })
    .then((response) => {
      if (!response.ok) return;
      const banner = document.createElement("aside");
      banner.className = "i18n-suggestion";
      banner.setAttribute("aria-label", "Language suggestion");
      banner.innerHTML = `<a href="${localized}">Read this page in ${match.nativeName}</a><button type="button" aria-label="Dismiss language suggestion">×</button>`;
      banner.querySelector("button").addEventListener("click", () => {
        localStorage.setItem(dismissedKey, "1");
        banner.remove();
      });
      document.body.appendChild(banner);
    })
    .catch(() => {});
})();
