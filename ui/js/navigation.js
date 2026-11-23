"use strict";

const navigationTabs = [...document.querySelectorAll('.navigation-tabs [role="tab"]')];
let answerScrollTop = 0;

byId("collapse-all").addEventListener("click", () => {
  const panel = byId("inject-tab").getAttribute("aria-selected") === "true" ? byId("answer-groups") : byId("tweaks-panel");
  for (const section of panel.querySelectorAll("details[open]")) section.open = false;
  void waitForWindowLayout();
});

for (const section of byId("visual-settings").querySelectorAll(".tweak-subgroups > details")) {
  section.addEventListener("toggle", () => {
    closeChoicePicker();
    if (!section.open) return;
    section.querySelector("summary").scrollIntoView({ block: "nearest" });
  });
}

function selectNavigationTab(selectedTab) {
  if (selectedTab.getAttribute("aria-selected") === "true") return;
  const workspace = byId("workspace");
  const answerOpen = !byId("answer-page").hidden;
  if (answerOpen && !byId("inject-panel").hidden) answerScrollTop = workspace.scrollTop;
  for (const tab of navigationTabs) {
    const selected = tab === selectedTab;
    tab.setAttribute("aria-selected", String(selected));
    tab.tabIndex = selected ? 0 : -1;
    byId(tab.getAttribute("aria-controls")).hidden = !selected;
  }
  stopScrolling();
  workspace.scrollTo(0, answerOpen && selectedTab.id === "inject-tab" ? answerScrollTop : 0);
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
