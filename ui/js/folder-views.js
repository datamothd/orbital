"use strict";

function createFolderPicker(id, choices, placeholder, fallback, matches = (a, b) => a === b) {
  const picker = byId(`${id}-picker`);
  const menu = byId(`${id}-options`);
  const selection = byId(`${id}-selection`);
  const apply = byId(`${id}-apply`);
  const value = byId(`${id}-value`);
  let selected = -1;
  let active = 0;

  const options = choices.map((choice, index) => {
    const option = document.createElement("div");
    option.id = `${id}-option-${index}`;
    option.setAttribute("role", "option");
    option.setAttribute("aria-selected", "false");
    option.textContent = choice.label;
    option.addEventListener("click", () => choose(index));
    menu.append(option);
    return option;
  });

  function close(focus = false) {
    menu.hidden = true;
    picker.setAttribute("aria-expanded", "false");
    menu.removeAttribute("aria-activedescendant");
    if (focus) picker.focus();
  }

  function activate(index) {
    active = (index + choices.length) % choices.length;
    options.forEach((option, index) => option.classList.toggle("active", index === active));
    menu.setAttribute("aria-activedescendant", options[active].id);
    options[active].scrollIntoView({ block: "nearest" });
  }

  function select(index) {
    selected = index;
    selection.textContent = choices[index]?.label || placeholder;
    options.forEach((option, index) => option.setAttribute("aria-selected", String(index === selected)));
    apply.disabled = picker.disabled || selected < 0;
  }

  function choose(index) {
    select(index);
    close(true);
  }

  function open(index = selected < 0 ? 0 : selected) {
    if (picker.disabled) return;
    const bounds = picker.getBoundingClientRect();
    const available = Math.max(80, Math.max(bounds.top, window.innerHeight - bounds.bottom) - 16);
    menu.style.width = `${Math.min(bounds.width, window.innerWidth - 32)}px`;
    menu.style.maxHeight = `${Math.min(260, available)}px`;
    menu.hidden = false;
    const style = getComputedStyle(menu);
    const scrollbarWidth = menu.offsetWidth - menu.clientWidth
      - parseFloat(style.borderLeftWidth) - parseFloat(style.borderRightWidth);
    menu.style.width = `${Math.min(bounds.width + scrollbarWidth, window.innerWidth - 32)}px`;
    menu.style.left = `${Math.max(16, Math.min(bounds.left, window.innerWidth - menu.offsetWidth - 16))}px`;
    menu.style.top = `${bounds.bottom + menu.offsetHeight + 8 <= window.innerHeight
      ? bounds.bottom + 6 : Math.max(8, bounds.top - menu.offsetHeight - 6)}px`;
    picker.setAttribute("aria-expanded", "true");
    menu.focus({ preventScroll: true });
    activate(index);
  }

  picker.addEventListener("click", () => menu.hidden ? open() : close(true));
  picker.addEventListener("keydown", (event) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      open(selected < 0 && event.key === "ArrowUp" ? choices.length - 1 : undefined);
    }
  });
  menu.addEventListener("keydown", (event) => {
    if (event.key === "Tab") {
      close(true);
      return;
    }
    if (["ArrowDown", "ArrowUp", "Home", "End", "Enter", " ", "Escape"].includes(event.key)) {
      event.preventDefault();
      if (event.key === "Escape") close(true);
      else if (event.key === "Enter" || event.key === " ") choose(active);
      else activate(event.key === "Home" ? 0 : event.key === "End" ? choices.length - 1
        : active + (event.key === "ArrowDown" ? 1 : -1));
    } else if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
      const index = choices.findIndex((choice, index) => index > active
        && choice.label.toLowerCase().startsWith(event.key.toLowerCase()));
      const fallback = choices.findIndex(choice => choice.label.toLowerCase().startsWith(event.key.toLowerCase()));
      if (index >= 0 || fallback >= 0) activate(index >= 0 ? index : fallback);
    }
  });
  document.addEventListener("pointerdown", (event) => {
    if (!picker.contains(event.target) && !menu.contains(event.target)) close();
  });
  window.addEventListener("resize", () => close());
  window.addEventListener("scroll", (event) => {
    if (!menu.contains(event.target)) close();
  }, true);

  return {
    matches,
    labelFor(value) {
      return choices.find(choice => matches(choice.value, value))?.label || fallback;
    },
    load(current, available = true) {
      select(choices.findIndex(choice => matches(choice.value, current)));
      value.textContent = !available ? "Unavailable" : selected < 0 ? fallback : choices[selected].label;
      close();
    },
    setDisabled(disabled) {
      picker.disabled = disabled;
      apply.disabled = disabled || selected < 0;
      if (disabled) close();
    },
    get selected() {
      return choices[selected]?.value ?? null;
    },
    get label() {
      return choices[selected]?.label;
    },
  };
}

const folderSorting = createFolderPicker("folder-sort", [
  ["name", false, "Name · A to Z"],
  ["name", true, "Name · Z to A"],
  ["date_modified", true, "Modified · newest first"],
  ["date_modified", false, "Modified · oldest first"],
  ["date_created", true, "Created · newest first"],
  ["date_created", false, "Created · oldest first"],
  ["type", false, "Type · A to Z"],
  ["type", true, "Type · Z to A"],
  ["size", true, "Size · largest first"],
  ["size", false, "Size · smallest first"],
].map(([column, descending, label]) => ({ value: { column, descending }, label })),
"Choose sorting", "Windows defaults or mixed sorting",
(a, b) => a.column === b?.column && a.descending === b?.descending);

const folderGrouping = createFolderPicker("folder-grouping", [
  ["none", "None"],
  ["name", "Name"],
  ["date_modified", "Date modified"],
  ["date_created", "Date created"],
  ["type", "Type"],
  ["size", "Size"],
].map(([value, label]) => ({ value, label })),
"Choose grouping", "Windows defaults or mixed grouping");
