(() => {
    function formatNumber(value) {
        if (value === 0) return "0";
        if (value >= 100) return value.toFixed(0);
        if (value >= 10) return value.toFixed(1);
        return value.toFixed(2);
    }

    function formatNutrient(value, unit) {
        if (unit === "kJ") return `${formatNumber(value)} kJ`;
        if (value >= 1) return `${formatNumber(value)} g`;
        if (value >= 0.001) return `${formatNumber(value * 1000)} mg`;
        return `${formatNumber(value * 1_000_000)} µg`;
    }

    function updateNutrition(input) {
        const preview = input.closest("#food-preview");
        const nutrition = preview?.querySelector("#food-nutrition");
        if (!nutrition) return;

        const rawAmount = input.value.trim();
        const amount = Number(rawAmount);
        const valid = rawAmount && Number.isFinite(amount) && amount > 0;
        const basisUnit = nutrition.dataset.basisUnit;
        const factor = valid ? (basisUnit === "count" ? amount : amount / 100) : 1;

        const label = nutrition.querySelector("[data-nutrition-label]");
        if (valid) {
            const unit = basisUnit === "count" ? (amount === 1 ? "item" : "items") : basisUnit;
            label.textContent = `Nutrition for ${rawAmount} ${unit}`;
        } else {
            const basis = basisUnit === "count" ? "item" : `100 ${basisUnit}`;
            label.textContent = `Nutrition per ${basis}`;
        }

        for (const output of nutrition.querySelectorAll("[data-nutrition]")) {
            const nutrient = output.dataset.nutrition;
            const basisValue = nutrition.dataset[nutrient];
            const unit = nutrient === "energy" ? "kcal" : "g";
            output.textContent = basisValue
                ? `${formatNumber(Number(basisValue) * factor)} ${unit}`
                : `— ${unit}`;
        }

        for (const output of preview.querySelectorAll("[data-nutrient-detail]")) {
            const value = Number(output.dataset.basisValue) * factor;
            output.textContent = formatNutrient(value, output.dataset.unit);
        }
    }

    document.addEventListener("input", (event) => {
        if (event.target.matches?.("#food-amount")) updateNutrition(event.target);
    });
})();
