(() => {
  function focusAmount(selector = "[data-food-amount]") {
    const amount = document.querySelector(selector);
    if (!(amount instanceof HTMLInputElement)) return;

    amount.focus();
    amount.select();
  }

  function updateCalories(input) {
    const output = input.closest("[data-food-calorie-field]")?.querySelector(
      "[data-food-kcal-output]",
    );
    if (!(output instanceof HTMLOutputElement)) return;

    const amount = Number(input.value);
    const energyPer100 = Number(input.dataset.energyPerHundred);
    output.textContent = Number.isFinite(amount) && amount > 0 &&
        input.dataset.energyPerHundred !== "" && Number.isFinite(energyPer100)
      ? `${Math.round(energyPer100 * amount / 100).toLocaleString()} kcal`
      : "-- kcal";
  }

  function setupCalories(root) {
    const inputs = [];
    if (root.matches?.("[data-food-calorie-input]")) inputs.push(root);
    inputs.push(...root.querySelectorAll?.("[data-food-calorie-input]") ?? []);

    for (const input of inputs) {
      if (input instanceof HTMLInputElement) updateCalories(input);
    }
  }

  function resetSearch() {
    const combobox = document.querySelector("server-combobox");
    combobox?.reset?.();
    document.querySelector("[data-combobox-input]")?.focus();
  }

  document.addEventListener("DOMContentLoaded", () => {
    setupCalories(document);
    if (document.querySelector("[data-food-amount][autofocus]")) focusAmount();
  });

  document.addEventListener("input", (event) => {
    if (
      event.target instanceof HTMLInputElement &&
      event.target.matches("[data-food-calorie-input]")
    ) {
      updateCalories(event.target);
    }
  });

  document.addEventListener(
    "htmx:after:process",
    (event) => setupCalories(event.target),
  );

  document.addEventListener(
    "toggle",
    (event) => {
      const details = event.target;
      if (
        !(details instanceof HTMLDetailsElement) ||
        !details.matches("[data-food-entry-details]") || !details.open
      ) {
        return;
      }
      const amount = details.querySelector("[data-food-edit-amount]");
      if (amount instanceof HTMLInputElement) {
        amount.focus();
        amount.select();
      }
    },
    true,
  );

  document.addEventListener("click", (event) => {
    const cancel = event.target.closest?.("[data-food-edit-cancel]");
    if (cancel) {
      event.preventDefault();
      const details = cancel.closest("[data-food-entry-details]");
      details?.querySelector("form")?.reset();
      if (details instanceof HTMLDetailsElement) details.open = false;
    }
  });

  document.addEventListener("htmx:after:settle", (event) => {
    if (!(event.target instanceof HTMLElement)) return;
    setupCalories(event.target);

    if (event.target.id === "food-preview") {
      if (event.target.querySelector("[data-food-amount]")) {
        focusAmount();
      } else {
        document.querySelector("[data-combobox-input]")?.focus();
      }
    }

    if (
      event.target.id === "food-log" &&
      event.target.dataset.foodAdded === "true"
    ) {
      resetSearch();
    }
  });
})();
