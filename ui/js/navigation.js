"use strict";

const navigationTabs = [...document.querySelectorAll('.navigation-tabs [role="tab"]')];

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
    if (event.key === "ArrowRight") nextIndex = (index + 1) % navigationTabs.length;
    else if (event.key === "ArrowLeft") nextIndex = (index - 1 + navigationTabs.length) % navigationTabs.length;
    else if (event.key === "Home") nextIndex = 0;
    else if (event.key === "End") nextIndex = navigationTabs.length - 1;
    else return;

    event.preventDefault();
    selectNavigationTab(navigationTabs[nextIndex]);
    navigationTabs[nextIndex].focus();
  });
}
