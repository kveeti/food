(() =>{

    function normalizeSwapStyle(style) {
        return style === 'before' ? 'beforebegin' :
            style === 'after' ? 'afterend' :
                style === 'prepend' ? 'afterbegin' :
                    style === 'append' ? 'beforeend' : style;
    }

    let api;

    function insertOptimisticContent(ctx) {
        ctx.optimistic = api.attributeValue(ctx.sourceElement, "hx-optimistic");
        if (!ctx.optimistic) {
            return
        }

        let sourceElt = document.querySelector(ctx.optimistic);
        if (!sourceElt) return;

        let target = ctx.target;

        if (typeof target === 'string') {
            target = document.querySelector(target);
        }
        if (!target) return;

        let sourceNodes = sourceElt instanceof HTMLTemplateElement ? sourceElt.content.childNodes : sourceElt.childNodes;

        // Clone template children directly — no wrapper div.
        let clonedNodes = [];
        for (let child of sourceNodes) {
            let clone = child.cloneNode(true);
            clone.classList.add('hx-optimistic');
            clonedNodes.push(clone);
        }

        // Set data-* for each request param on each cloned node
        if (ctx.optimisticBody) {
            let keys = new Set(ctx.optimisticBody.keys());
            for (let k of keys) {
                let values = ctx.optimisticBody.getAll(k).filter(v => typeof v === 'string');
                if (!values.length) continue;
                let val = values.length === 1 ? values[0] : JSON.stringify(values);
                for (let clone of clonedNodes) {
                    try {
                        clone.dataset[k] = val;
                    } catch (e) {
                        try {
                            clone.setAttribute('data-' + k, val);
                        } catch (e2) { /* skip */ }
                    }
                }
            }
        }

        let swapStyle = normalizeSwapStyle(ctx.swap);
        ctx.optHidden = [];

        if (swapStyle === 'innerHTML') {
            for (let child of target.children) {
                child.style.display = 'none';
                ctx.optHidden.push(child);
            }
            for (let clone of clonedNodes) target.appendChild(clone);
        } else if (['beforebegin', 'afterbegin', 'beforeend', 'afterend'].includes(swapStyle)) {
            for (let clone of clonedNodes) {
                target.insertAdjacentElement(swapStyle, clone);
            }
        } else {
            target.style.display = 'none';
            ctx.optHidden.push(target);
            for (let clone of clonedNodes) target.after(clone);
        }
        ctx.optimisticNodes = clonedNodes;
        for (let clone of clonedNodes) htmx.process(clone);
    }

    function removeOptimisticContent(ctx) {
        if (!ctx.optimisticNodes) return;

        for (let node of ctx.optimisticNodes) node.remove();
        ctx.optimisticNodes = null;

        for (let elt of ctx.optHidden) {
            elt.style.display = '';
        }
    }

    htmx.registerExtension('hx-optimistic', {
        init: (internalAPI) => { api = internalAPI; },
        htmx_config_request: (elt, detail) => {
            let body = detail.ctx.request.body;
            if (body?.entries) detail.ctx.optimisticBody = body;
        },
        htmx_before_request: (elt, detail) => {
            insertOptimisticContent(detail.ctx);
        },
        htmx_error : (elt, detail) => {
            removeOptimisticContent(detail.ctx)
        },
        htmx_before_swap : (elt, detail) => {
            removeOptimisticContent(detail.ctx)
        }
    });
})();
