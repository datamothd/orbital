"use strict";

const defaultWindowsApps = (() => {
  const current = [
    ["Calculator", "Microsoft.WindowsCalculator"], ["Camera", "Microsoft.WindowsCamera"],
    ["Clock", "Microsoft.WindowsAlarms"], ["Clipchamp", "Clipchamp.Clipchamp"],
    ["Copilot", "Microsoft.Copilot"], ["Feedback Hub", "Microsoft.WindowsFeedbackHub"],
    ["Get Help", "Microsoft.GetHelp"], ["Microsoft 365 Copilot / Office hub", "Microsoft.MicrosoftOfficeHub"],
    ["Microsoft Teams", "MSTeams"], ["Microsoft To Do", "Microsoft.Todos"],
    ["MSN News", "Microsoft.BingNews"], ["MSN Weather", "Microsoft.BingWeather"],
    ["Notepad", "Microsoft.WindowsNotepad"], ["Outlook for Windows", "Microsoft.OutlookForWindows"],
    ["Paint", "Microsoft.Paint"], ["Phone Link", "Microsoft.YourPhone"],
    ["Power Automate", "Microsoft.PowerAutomateDesktop"],
    ["Photos", "Microsoft.Windows.Photos"], ["Quick Assist", "MicrosoftCorporationII.QuickAssist"],
    ["Snipping Tool", "Microsoft.ScreenSketch"], ["Solitaire Collection", "Microsoft.MicrosoftSolitaireCollection"],
    ["Sound Recorder", "Microsoft.WindowsSoundRecorder"], ["Sticky Notes", "Microsoft.MicrosoftStickyNotes"],
    ["Windows Media Player", "Microsoft.ZuneMusic"], ["Windows Terminal", "Microsoft.WindowsTerminal"],
    ["Xbox", "Microsoft.GamingApp"], ["Xbox Game Bar", "Microsoft.XboxGamingOverlay"],
    ["Xbox Identity Provider", "Microsoft.XboxIdentityProvider"], ["Xbox Speech To Text Overlay", "Microsoft.XboxSpeechToTextOverlay"],
    ["Xbox TCUI", "Microsoft.Xbox.TCUI"],
  ];
  const components = [
    ["App Installer / WinGet", "Microsoft.DesktopAppInstaller"],
    ["Microsoft Store", "Microsoft.WindowsStore"], ["Store Purchase App", "Microsoft.StorePurchaseApp"],
    ["HEIF Image Extensions", "Microsoft.HEIFImageExtension"], ["HEVC Video Extensions (device dependent)", "Microsoft.HEVCVideoExtension"],
    ["VP9 Video Extensions", "Microsoft.VP9VideoExtensions"], ["Web Media Extensions", "Microsoft.WebMediaExtensions"],
    ["WebP Image Extension", "Microsoft.WebpImageExtension"], ["AV1 Video Extension", "Microsoft.AV1VideoExtension"],
    ["Raw Image Extension", "Microsoft.RawImageExtension"],
    ["Windows Web Experience Pack / Widgets", "MicrosoftWindows.Client.WebExperience"],
  ];
  return [
    ...current.map(([name, id]) => ({ name, id, kind: "appx", availability: "Windows 11; availability varies by image, region and edition" })),
    ...components.map(([name, id]) => ({ name, id, kind: "appx", availability: "Bundled component; release/device dependent", caution: "Other apps may depend on this component." })),
    { name: "Microsoft Edge", id: "Microsoft.Edge", kind: "desktop", availability: "Windows 11; removal availability varies by region" },
    { name: "OneDrive", id: "Microsoft.OneDrive", kind: "desktop", availability: "Windows 11; availability varies by edition and image" },
  ].sort((a, b) => a.name.localeCompare(b.name));
})();

function defaultAppMatch(entry, app) {
  const ids = [app.id, app.catalog_id || "", ...(app.details || [])
    .filter(field => /^(local identifier|package family name|package name)$/i.test(field.label))
    .map(field => field.value)].map(value => String(value).toLowerCase());
  const id = entry.id.toLowerCase();
  return ids.some(value => value === id || (entry.kind === "appx" &&
    (value.startsWith(`${id}_`) || value.startsWith(`msix\\${id}_`))));
}
