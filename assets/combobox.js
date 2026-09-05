class ServerCombobox extends HTMLElement {
  connectedCallback() {
    if (this.connected) return;

    this.input = this.querySelector("[data-combobox-input]");
    this.spinner = this.querySelector("[data-combobox-spinner]");
    this.popup = this.querySelector("[data-combobox-popup]");
    this.list = this.popup?.querySelector("[data-combobox-list]");
    if (!this.input || !this.popup || !this.list) return;

    this.connected = true;
    this.activeIndex = -1;
    this.userActivated = false;
    this.pendingRequests = 0;
    this.loadingUntil = null;
    this.loadingTimer = null;
    this.dataset.enhanced = "";
    this.popup.dataset.enhanced = "";

    this.input.setAttribute("role", "combobox");
    this.input.setAttribute("aria-autocomplete", "list");
    this.input.setAttribute("aria-haspopup", "listbox");

    if (!this.popup.id) {
      this.popup.id = `combobox-popup-${crypto.randomUUID()}`;
    }

    this.supportsPopover = "showPopover" in HTMLElement.prototype;
    if (this.supportsPopover) {
      const anchorName = `--${this.popup.id}-anchor`;
      this.style.anchorName = anchorName;
      this.popup.style.positionAnchor = anchorName;
      this.popup.setAttribute("popover", "manual");
      this.popup.hidden = false;
    }

    this.onKeydown = this.onKeydown.bind(this);
    this.onInput = this.onInput.bind(this);
    this.onFocus = this.onFocus.bind(this);
    this.onPointerMove = this.onPointerMove.bind(this);
    this.onClick = this.onClick.bind(this);
    this.onDocumentPointerDown = this.onDocumentPointerDown.bind(this);
    this.onBeforeRequest = this.onBeforeRequest.bind(this);
    this.onFinallyRequest = this.onFinallyRequest.bind(this);
    this.onAfterSettle = this.onAfterSettle.bind(this);

    this.input.addEventListener("keydown", this.onKeydown);
    this.input.addEventListener("input", this.onInput);
    this.input.addEventListener("focus", this.onFocus);
    this.popup.addEventListener("pointermove", this.onPointerMove);
    this.popup.addEventListener("click", this.onClick);
    this.addEventListener("htmx:before:request", this.onBeforeRequest);
    this.addEventListener("htmx:finally:request", this.onFinallyRequest);
    this.addEventListener("htmx:after:settle", this.onAfterSettle);
    document.addEventListener("pointerdown", this.onDocumentPointerDown);

    this.observer = new MutationObserver(() => this.refresh());
    this.observer.observe(this.popup, { childList: true, subtree: true });
    this.refresh();
  }

  disconnectedCallback() {
    if (!this.connected) return;

    this.observer?.disconnect();
    clearTimeout(this.loadingTimer);
    delete this.dataset.loading;
    this.spinner?.removeAttribute("data-loading");
    this.input.removeEventListener("keydown", this.onKeydown);
    this.input.removeEventListener("input", this.onInput);
    this.input.removeEventListener("focus", this.onFocus);
    this.popup.removeEventListener("pointermove", this.onPointerMove);
    this.popup.removeEventListener("click", this.onClick);
    this.removeEventListener("htmx:before:request", this.onBeforeRequest);
    this.removeEventListener("htmx:finally:request", this.onFinallyRequest);
    this.removeEventListener("htmx:after:settle", this.onAfterSettle);
    document.removeEventListener("pointerdown", this.onDocumentPointerDown);
    this.connected = false;
  }

  get options() {
    return this.list
      ? [...this.list.querySelectorAll("[data-combobox-option]")]
      : [];
  }

  reset() {
    this.input.value = "";
    this.list?.replaceChildren();
    const status = this.popup.querySelector("[data-combobox-status]");
    if (status) status.textContent = "";
    this.close();
    this.input.dispatchEvent(new Event("input", { bubbles: true }));
  }

  get loop() {
    return this.getAttribute("loop") !== "false";
  }

  set loop(value) {
    if (value) {
      this.removeAttribute("loop");
    } else {
      this.setAttribute("loop", "false");
    }
  }

  refresh() {
    const list = this.popup.querySelector("[data-combobox-list]");
    if (!list) {
      this.close();
      return;
    }

    this.list = list;
    if (!this.list.id) {
      this.list.id = `combobox-list-${crypto.randomUUID()}`;
    }
    this.list.setAttribute("role", "listbox");
    this.list.dataset.enhanced = "";
    this.input.setAttribute("aria-controls", this.list.id);
    this.setBusy(this.pendingRequests > 0);

    const options = this.options;
    options.forEach((option, index) => {
      option.id ||= `${this.list.id}-option-${index}`;
      option.setAttribute("role", "option");
      option.setAttribute("aria-selected", "false");
      option.tabIndex = -1;
      option.closest("li")?.setAttribute("role", "none");
    });

    this.activeIndex = -1;
    this.input.removeAttribute("aria-activedescendant");

    if (!this.hasContent()) {
      this.close();
      return;
    }

    if (this.userActivated) {
      this.open();
      if (document.activeElement === this.input && options.length > 0) {
        this.setActive(0);
      }
    } else {
      this.close();
    }
  }

  hasContent() {
    const status = this.popup.querySelector("[data-combobox-status]");
    return this.options.length > 0 || Boolean(status?.textContent.trim());
  }

  open() {
    if (!this.hasContent()) return;

    this.userActivated = true;
    if (this.supportsPopover) {
      if (!this.popup.matches(":popover-open")) this.popup.showPopover();
    } else {
      this.popup.hidden = false;
    }
    this.input.setAttribute("aria-expanded", "true");
  }

  close() {
    this.userActivated = false;
    if (this.supportsPopover) {
      if (this.popup.matches(":popover-open")) this.popup.hidePopover();
    } else {
      this.popup.hidden = true;
    }
    this.input.setAttribute("aria-expanded", "false");
    this.setActive(-1);
  }

  isOpen() {
    return this.supportsPopover
      ? this.popup.matches(":popover-open")
      : !this.popup.hidden;
  }

  setActive(index) {
    const options = this.options;
    for (const option of options) {
      option.setAttribute("aria-selected", "false");
    }

    if (index < 0 || index >= options.length) {
      this.activeIndex = -1;
      this.input.removeAttribute("aria-activedescendant");
      return;
    }

    const option = options[index];
    this.activeIndex = index;
    option.setAttribute("aria-selected", "true");
    this.input.setAttribute("aria-activedescendant", option.id);
    option.scrollIntoView({ block: "nearest" });
  }

  setBusy(busy) {
    if (busy) {
      this.input.setAttribute("aria-busy", "true");
      this.list?.setAttribute("aria-busy", "true");
    } else {
      this.input.removeAttribute("aria-busy");
      this.list?.removeAttribute("aria-busy");
    }
  }

  startLoading() {
    clearTimeout(this.loadingTimer);
    this.loadingTimer = null;
    this.loadingUntil = Math.max(
      this.loadingUntil ?? 0,
      performance.now() + 300,
    );
    this.dataset.loading = "";
    this.spinner?.setAttribute("data-loading", "");
  }

  prolongLoading() {
    if (this.loadingUntil === null) return;

    clearTimeout(this.loadingTimer);
    this.loadingTimer = null;
    this.loadingUntil = Math.max(this.loadingUntil, performance.now() + 300);
    this.stopLoading();
  }

  stopLoading() {
    if (this.pendingRequests > 0 || this.loadingUntil === null) return;

    clearTimeout(this.loadingTimer);
    const remaining = this.loadingUntil - performance.now();
    if (remaining > 0) {
      this.loadingTimer = setTimeout(() => this.stopLoading(), remaining);
      return;
    }

    delete this.dataset.loading;
    this.spinner?.removeAttribute("data-loading");
    this.loadingUntil = null;
    this.loadingTimer = null;
  }

  onInput() {
    this.prolongLoading();
    this.open();
    if (this.options.length > 0) this.setActive(0);
  }

  onFocus() {
    this.open();
    if (this.options.length > 0) this.setActive(0);
  }

  onKeydown(event) {
    if (event.isComposing || event.keyCode === 229) return;

    const options = this.options;

    if (event.key === "ArrowDown" && options.length > 0) {
      event.preventDefault();
      this.open();
      const nextIndex = this.activeIndex >= options.length - 1 && this.loop
        ? 0
        : Math.min(this.activeIndex + 1, options.length - 1);
      this.setActive(nextIndex);
      return;
    }

    if (event.key === "ArrowUp" && options.length > 0) {
      event.preventDefault();
      this.open();
      const nextIndex = this.activeIndex <= 0 && this.loop
        ? options.length - 1
        : this.activeIndex < 0
        ? options.length - 1
        : Math.max(this.activeIndex - 1, 0);
      this.setActive(nextIndex);
      return;
    }

    if (event.key === "Enter" && this.activeIndex >= 0 && this.isOpen()) {
      event.preventDefault();
      options[this.activeIndex].click();
      return;
    }

    if (event.key === "Escape" && this.isOpen()) {
      event.preventDefault();
      this.close();
      return;
    }

    if (event.key === "Tab") this.close();
  }

  onPointerMove(event) {
    const option = event.target.closest?.("[data-combobox-option]");
    if (!option) return;
    this.setActive(this.options.indexOf(option));
  }

  onClick(event) {
    if (event.target.closest?.("[data-combobox-option]")) this.close();
  }

  onDocumentPointerDown(event) {
    if (!this.contains(event.target)) this.close();
  }

  onBeforeRequest(event) {
    if (event.target.closest?.("[data-combobox-option]")) return;
    this.userActivated = true;
    this.pendingRequests += 1;
    this.setBusy(true);
    this.startLoading();
  }

  onFinallyRequest(event) {
    if (event.target.closest?.("[data-combobox-option]")) return;
    this.pendingRequests = Math.max(0, this.pendingRequests - 1);
    this.setBusy(this.pendingRequests > 0);
    this.stopLoading();
  }

  onAfterSettle(event) {
    if (event.target !== this.popup) return;
    this.refresh();
  }
}

if (!customElements.get("server-combobox")) {
  customElements.define("server-combobox", ServerCombobox);
}
