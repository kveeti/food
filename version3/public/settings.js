(() => {
    function suggestDeviceTimezone(root) {
        const input = root.querySelector?.("#timezone-query");
        if (!input || input.dataset.deviceTimezone || input.value.trim()) return;

        input.dataset.deviceTimezone = "true";
        const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
        if (!timezone) return;

        input.value = timezone;
        const hint = root.querySelector?.("#device-timezone-hint");
        if (hint) hint.hidden = false;
        input.dispatchEvent(new Event("input", { bubbles: true }));
    }

    document.addEventListener("DOMContentLoaded", () => suggestDeviceTimezone(document));
    document.addEventListener("htmx:after:process", (event) => suggestDeviceTimezone(event.target));
})();
