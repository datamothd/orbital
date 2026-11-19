"use strict";

const terminalChannel = new BroadcastChannel("orbital-terminal");
const terminalHistory = byId("tweaks-output-history");
const terminalEntries = [];
let terminalOpening = false;

async function openTerminal() {
  if (!invoke || terminalOpening) return;
  terminalOpening = true;
  try {
    await invoke("open_terminal");
  } catch (error) {
    console.error("Could not open terminal output:", error);
  } finally {
    terminalOpening = false;
  }
}

function writeTweaksOutput(message, { error = false, openOnError = true } = {}) {
  const text = String(message || "").replace(/\r\n?/g, "\n").trim() || "Ready.";
  const previous = terminalEntries.at(-1);
  if (previous?.text !== text || previous.error !== error) {
    terminalEntries.push({ text, error });
    if (terminalEntries.length > 200) terminalEntries.shift();
    terminalChannel.postMessage(terminalEntries);
  }
  if (error && openOnError) openTerminal();
}

if (terminalHistory) {
  terminalChannel.onmessage = ({ data }) => {
    if (!Array.isArray(data)) return;
    terminalHistory.replaceChildren(...(data.length ? data : [{ text: "Ready." }]).map(({ text, error }) => {
      const row = element("div", error ? "tweaks-output-line error" : "tweaks-output-line");
      const prompt = element("span", "tweaks-output-prompt", ">");
      prompt.setAttribute("aria-hidden", "true");
      row.append(prompt, element("p", "tweaks-output-message", text));
      return row;
    }));
    terminalHistory.scrollTop = terminalHistory.scrollHeight;
  };
  terminalChannel.postMessage("ready");
} else {
  terminalChannel.onmessage = ({ data }) => {
    if (data === "ready") terminalChannel.postMessage(terminalEntries);
  };
  const button = byId("open-terminal");
  button.disabled = !invoke;
  button.addEventListener("click", openTerminal);
}
