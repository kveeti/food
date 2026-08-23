(() => {
    const closingDialogs = new WeakMap();

    function closeDialog(dialog) {
        if (!dialog?.open) return Promise.resolve();
        const current = closingDialogs.get(dialog);
        if (current) return current;

        const closing = (async () => {
            if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
                dialog.classList.add("is-closing");
                void dialog.offsetWidth;
                const backdrop = dialog.nextElementSibling;
                const animations = [
                    ...dialog.getAnimations(),
                    ...(backdrop?.getAnimations() ?? []),
                ];
                await Promise.allSettled(animations.map((animation) => animation.finished));
            }

            if (dialog.open) dialog.close();
            dialog.classList.remove("is-closing");
        })();
        closingDialogs.set(dialog, closing);
        closing.finally(() => closingDialogs.delete(dialog));
        return closing;
    }

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

    function setMealEntrySelected(entry, selected) {
        entry.checked = selected;
        entry.closest("li").querySelector('input[type="number"]').disabled = !selected;
    }

    function updateSelectAll(form) {
        const selectAll = form.querySelector("[data-select-all]");
        const entries = [...form.querySelectorAll("[data-meal-entry]:not(:disabled)")];
        const selected = entries.filter((entry) => entry.checked).length;
        selectAll.disabled = entries.length === 0;
        selectAll.checked = entries.length > 0 && selected === entries.length;
        selectAll.indeterminate = selected > 0 && selected < entries.length;
    }

    function showMealStageControls(root) {
        for (const control of root.querySelectorAll?.("[data-select-all-control]") ?? []) {
            control.hidden = false;
            updateSelectAll(control.closest("[data-meal-stage]"));
        }
    }

    document.addEventListener("submit", async (event) => {
        const closeForm = event.target.closest?.("[data-close-dialog-form]");
        const deleteForm = event.target.closest?.("[data-delete-dialog-form]");
        if (!closeForm && (!deleteForm || !window.htmx)) return;
        if (deleteForm?.dataset.dialogSubmitReady) {
            delete deleteForm.dataset.dialogSubmitReady;
            return;
        }

        event.preventDefault();
        event.stopImmediatePropagation();
        const submitter = event.submitter;
        await closeDialog(event.target.closest("dialog"));
        if (deleteForm) {
            deleteForm.dataset.dialogSubmitReady = "true";
            deleteForm.requestSubmit(submitter);
        }
    }, true);
    document.addEventListener("cancel", (event) => {
        if (!event.target.matches?.(".food-delete-dialog")) return;
        event.preventDefault();
        closeDialog(event.target);
    }, true);
    document.addEventListener("input", (event) => {
        if (event.target.matches?.("#food-amount")) updateNutrition(event.target);
    });
    document.addEventListener("change", (event) => {
        const form = event.target.closest?.("[data-meal-stage]");
        if (!form) return;

        if (event.target.matches("[data-select-all]")) {
            for (const entry of form.querySelectorAll("[data-meal-entry]:not(:disabled)")) {
                setMealEntrySelected(entry, event.target.checked);
            }
        } else if (event.target.matches("[data-meal-entry]")) {
            setMealEntrySelected(event.target, event.target.checked);
        } else {
            return;
        }
        updateSelectAll(form);
    });
    document.addEventListener("DOMContentLoaded", () => showMealStageControls(document));
    document.addEventListener("htmx:after:process", (event) => showMealStageControls(event.target));
})();
