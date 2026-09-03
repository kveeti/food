(() => {
  const initialized = new WeakSet();
  const step = 10;

  function setup(form) {
    if (initialized.has(form)) return;
    initialized.add(form);

    const input = form.querySelector("[data-water-amount]");
    const output = form.querySelector("[data-water-output]");
    const stage = form.querySelector("[data-water-stage]");
    const vesselElements = [...form.querySelectorAll("[data-water-vessel]")];
    if (!input || !output || !stage || vesselElements.length !== 2) return;

    const vessels = new Map();
    const amounts = new Map();
    const frameMotion = !globalThis.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    let selected = "glass";
    let animationFrame;
    let previousFrame;

    for (const element of vesselElements) {
      const mode = element.dataset.waterVessel;
      const vessel = {
        element,
        bubbleClip: element.querySelector(
          `[data-water-bubble-clip="${mode}"]`,
        ),
        liquid: element.querySelector(`[data-water-liquid="${mode}"]`),
        min: Number(element.dataset.waterMin),
        max: Number(element.dataset.waterMax),
        top: Number(element.dataset.waterTop),
        bottom: Number(element.dataset.waterBottom),
        currentY: undefined,
        targetY: undefined,
      };
      vessels.set(mode, vessel);
      amounts.set(mode, Number(element.dataset.waterDefault));
    }

    function renderLiquid(vessel, y) {
      const transform = `translateY(${y}px)`;
      vessel.liquid.style.transform = transform;
      vessel.bubbleClip.style.transform = transform;
    }

    function animateLiquids(time) {
      const elapsed = Math.min(time - (previousFrame ?? time - 16), 32);
      const blend = 1 - Math.exp(-elapsed / 64);
      let moving = false;

      for (const vessel of vessels.values()) {
        if (vessel.currentY === undefined || vessel.targetY === undefined) {
          continue;
        }
        const distance = vessel.targetY - vessel.currentY;
        if (Math.abs(distance) < 0.05) {
          vessel.currentY = vessel.targetY;
        } else {
          vessel.currentY += distance * blend;
          moving = true;
        }
        renderLiquid(vessel, vessel.currentY);
      }

      if (moving) {
        previousFrame = time;
        animationFrame = requestAnimationFrame(animateLiquids);
      } else {
        previousFrame = undefined;
        animationFrame = undefined;
      }
    }

    function updateLiquid(mode) {
      const vessel = vessels.get(mode);
      const amount = amounts.get(mode);
      const ratio = amount / vessel.max;
      const visualRatio = ratio + (1 - ratio) * 0.025;
      const y = vessel.bottom - visualRatio * (vessel.bottom - vessel.top);

      if (!frameMotion || vessel.currentY === undefined) {
        vessel.currentY = y;
        vessel.targetY = y;
        renderLiquid(vessel, y);
      } else {
        vessel.targetY = y;
        if (animationFrame === undefined) {
          animationFrame = requestAnimationFrame(animateLiquids);
        }
      }

      vessel.element.setAttribute("aria-valuenow", String(amount));
      vessel.element.setAttribute("aria-valuetext", `${amount} millilitres`);
    }

    function updateSelection() {
      for (const [mode, vessel] of vessels) {
        const isSelected = mode === selected;
        vessel.element.dataset.selected = String(isSelected);
        vessel.element.setAttribute("aria-current", String(isSelected));
      }
    }

    function setAmount(value) {
      const vessel = vessels.get(selected);
      const amount = Math.min(
        vessel.max,
        Math.max(vessel.min, Math.round(value / step) * step),
      );
      amounts.set(selected, amount);
      input.value = String(amount);
      output.textContent = `${amount.toLocaleString("en-US")} ml`;
      updateLiquid(selected);
    }

    function switchMode(mode) {
      if (mode === selected) return;
      selected = mode;
      updateSelection();
      setAmount(amounts.get(mode));
    }

    function setFromPointer(vessel, event) {
      const bounds = vessel.element.getBoundingClientRect();
      const ratio = (bounds.bottom - event.clientY) / bounds.height;
      setAmount(ratio * vessel.max);
    }

    for (const [mode, vessel] of vessels) {
      vessel.element.addEventListener("click", () => switchMode(mode));
      vessel.element.addEventListener("pointerdown", (event) => {
        if (mode !== selected) return;
        event.preventDefault();
        vessel.element.setPointerCapture(event.pointerId);
        setFromPointer(vessel, event);
      });
      vessel.element.addEventListener("pointermove", (event) => {
        if (
          mode === selected &&
          vessel.element.hasPointerCapture(event.pointerId)
        ) {
          setFromPointer(vessel, event);
        }
      });
      vessel.element.addEventListener("keydown", (event) => {
        if (mode !== selected) {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            switchMode(mode);
          } else if (event.key === "Tab" && !event.shiftKey) {
            const submit = form.querySelector(
              'button[type="submit"], button:not([type])',
            );
            if (submit) {
              event.preventDefault();
              submit.focus();
            }
          }
          return;
        }

        if (event.key === "Enter") {
          event.preventDefault();
          form.requestSubmit();
          return;
        }

        if (event.key === "Tab" && !event.shiftKey) {
          event.preventDefault();
          const inactive = [...vessels.entries()].find(([name]) =>
            name !== selected
          );
          inactive?.[1].element.focus();
          return;
        }

        const amount = amounts.get(mode);
        let next;
        if (event.key === "ArrowUp" || event.key === "ArrowRight") {
          next = amount + step;
        }
        if (event.key === "ArrowDown" || event.key === "ArrowLeft") {
          next = amount - step;
        }
        if (event.key === "Home") next = vessel.min;
        if (event.key === "End") next = vessel.max;
        if (next === undefined) return;
        event.preventDefault();
        setAmount(next);
      });
    }

    for (const mode of vessels.keys()) updateLiquid(mode);
    updateSelection();
    setAmount(amounts.get("glass"));
    input.disabled = false;
    stage.classList.remove("invisible");
    output.classList.remove("invisible");
  }

  function setupAll(root) {
    if (root.matches?.("[data-water-glass]")) setup(root);
    root.querySelectorAll?.("[data-water-glass]").forEach(setup);
  }

  document.addEventListener("DOMContentLoaded", () => setupAll(document));
  document.addEventListener(
    "htmx:after:process",
    (event) => setupAll(event.target),
  );
})();
