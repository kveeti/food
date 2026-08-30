(() => {
  function suggestSettings(root) {
    let suggested = false;

    const locale = root.querySelector?.("#locale");
    if (locale && !locale.value.trim() && navigator.language) {
      locale.value = navigator.language;
      suggested = true;
    }

    const timezone = root.querySelector?.("#timezone");
    const deviceTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (timezone && !timezone.value.trim() && deviceTimezone) {
      timezone.value = deviceTimezone;
      suggested = true;
    }

    const hint = root.querySelector?.("#device-settings-hint");
    if (hint && suggested) hint.hidden = false;
  }

  document.addEventListener(
    "DOMContentLoaded",
    () => suggestSettings(document),
  );
  document.addEventListener("htmx:after:process", (event) => {
    suggestSettings(event.target);
  });
})();
