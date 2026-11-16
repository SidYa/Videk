// Applies the saved theme and accent before the first paint (no flash of the wrong colours).
// Plain script (not a module) so it runs synchronously in <head>.
(function () {
  var mode = "system";
  try {
    mode = localStorage.getItem("videk.theme") || "system";
  } catch (e) {}
  var dark =
    mode === "dark" ||
    (mode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
})();
