(() => {
  function normalizeSwapStyle(style) {
    return style === "before"
      ? "beforebegin"
      : style === "after"
      ? "afterend"
      : style === "prepend"
      ? "afterbegin"
      : style === "append"
      ? "beforeend"
      : style;
  }

  let api;

  function insertOptimisticContent(ctx) {
    ctx.optimistic = api.attributeValue(ctx.sourceElement, "hx-optimistic");
    if (!ctx.optimistic) return;

    const source = document.querySelector(ctx.optimistic);
    if (!source) return;

    let target = ctx.target;
    if (typeof target === "string") target = document.querySelector(target);
    if (!target) return;

    const sourceNodes = source instanceof HTMLTemplateElement
      ? source.content.childNodes
      : source.childNodes;
    const optimisticNodes = [];
    for (const child of sourceNodes) {
      const clone = child.cloneNode(true);
      clone.classList.add("hx-optimistic");
      optimisticNodes.push(clone);
    }

    if (ctx.optimisticBody) {
      const keys = new Set(ctx.optimisticBody.keys());
      for (const key of keys) {
        const values = ctx.optimisticBody.getAll(key).filter((value) =>
          typeof value === "string"
        );
        if (!values.length) continue;
        const value = values.length === 1 ? values[0] : JSON.stringify(values);
        for (const clone of optimisticNodes) {
          try {
            clone.dataset[key] = value;
          } catch {
            try {
              clone.setAttribute(`data-${key}`, value);
            } catch { /* skip invalid attribute names */ }
          }
        }
      }
    }

    const swap = normalizeSwapStyle(ctx.swap);
    ctx.optHidden = [];

    if (swap === "innerHTML") {
      for (const child of target.children) {
        child.style.display = "none";
        ctx.optHidden.push(child);
      }
      for (const clone of optimisticNodes) target.appendChild(clone);
    } else if (
      ["beforebegin", "afterbegin", "beforeend", "afterend"].includes(swap)
    ) {
      for (const clone of optimisticNodes) {
        target.insertAdjacentElement(swap, clone);
      }
    } else {
      target.style.display = "none";
      ctx.optHidden.push(target);
      for (const clone of optimisticNodes) target.after(clone);
    }
    ctx.optimisticNodes = optimisticNodes;
    for (const clone of optimisticNodes) htmx.process(clone);
  }

  function removeOptimisticContent(ctx) {
    if (!ctx.optimisticNodes) return;

    for (const node of ctx.optimisticNodes) node.remove();
    ctx.optimisticNodes = null;

    for (const element of ctx.optHidden) element.style.display = "";
  }

  htmx.registerExtension("hx-optimistic", {
    init: (internalAPI) => {
      api = internalAPI;
    },
    htmx_config_request: (_element, detail) => {
      const body = detail.ctx.request.body;
      if (body?.entries) detail.ctx.optimisticBody = body;
    },
    htmx_before_request: (_element, detail) => {
      insertOptimisticContent(detail.ctx);
    },
    htmx_error: (_element, detail) => {
      removeOptimisticContent(detail.ctx);
    },
    htmx_before_swap: (_element, detail) => {
      removeOptimisticContent(detail.ctx);
    },
  });
})();
