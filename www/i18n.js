(() => {
  const locales = [{"code": "ja", "slug": "ja", "nativeName": "日本語", "languages": ["ja", "ja-jp"], "suggestionAria": "言語の提案", "suggestionText": "このページを日本語で読む", "dismissLabel": "言語提案を閉じる"}, {"code": "de", "slug": "de", "nativeName": "Deutsch", "languages": ["de", "de-at", "de-ch", "de-de"], "suggestionAria": "Sprachvorschlag", "suggestionText": "Diese Seite auf Deutsch lesen", "dismissLabel": "Sprachvorschlag schließen"}, {"code": "fr", "slug": "fr", "nativeName": "Français", "languages": ["fr", "fr-be", "fr-ca", "fr-ch", "fr-fr"], "suggestionAria": "Suggestion de langue", "suggestionText": "Lire cette page en français", "dismissLabel": "Fermer la suggestion de langue"}, {"code": "zh-Hans", "slug": "zh-hans", "nativeName": "简体中文", "languages": ["zh", "zh-cn", "zh-hans", "zh-sg"], "suggestionAria": "语言建议", "suggestionText": "用简体中文阅读本页", "dismissLabel": "关闭语言建议"}];
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
      banner.setAttribute("aria-label", match.suggestionAria);
      const link = document.createElement("a");
      link.href = localized;
      link.textContent = match.suggestionText;
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("aria-label", match.dismissLabel);
      button.textContent = "×";
      button.addEventListener("click", () => {
        localStorage.setItem(dismissedKey, "1");
        banner.remove();
      });
      banner.append(link, button);
      document.body.appendChild(banner);
    })
    .catch(() => {});
})();
