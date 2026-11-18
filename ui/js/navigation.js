"use strict";

const navigationTabs = [...document.querySelectorAll('.navigation-tabs [role="tab"]')];

byId("collapse-all").addEventListener("click", () => {
  const panel = byId("answer-page").hidden ? byId("tweaks-panel") : byId("answer-groups");
  collapseFromTop = true;
  for (const section of panel.querySelectorAll("details[open]")) section.open = false;
  void waitForWindowLayout();
});

const visualSections = [...byId("visual-settings").querySelectorAll(".tweak-subgroups > details")];
for (const section of visualSections) {
  section.addEventListener("toggle", () => {
    closeChoicePicker();
    if (!section.open) return;
    for (const other of visualSections) {
      if (other !== section) other.open = false;
    }
    section.querySelector("summary").scrollIntoView({ block: "nearest" });
  });
}

function selectNavigationTab(selectedTab) {
  for (const tab of navigationTabs) {
    const selected = tab === selectedTab;
    tab.setAttribute("aria-selected", String(selected));
    tab.tabIndex = selected ? 0 : -1;
    byId(tab.getAttribute("aria-controls")).hidden = !selected;
  }
}

for (const [index, tab] of navigationTabs.entries()) {
  tab.addEventListener("click", () => selectNavigationTab(tab));
  tab.addEventListener("keydown", (event) => {
    let nextIndex;
    if (event.key === "ArrowDown") nextIndex = (index + 1) % navigationTabs.length;
    else if (event.key === "ArrowUp") nextIndex = (index - 1 + navigationTabs.length) % navigationTabs.length;
    else if (event.key === "Home") nextIndex = 0;
    else if (event.key === "End") nextIndex = navigationTabs.length - 1;
    else return;

    event.preventDefault();
    selectNavigationTab(navigationTabs[nextIndex]);
    navigationTabs[nextIndex].focus();
  });
}
