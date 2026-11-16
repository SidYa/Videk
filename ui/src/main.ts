import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// No browser context menu / reload / find: this is an app window.
window.addEventListener("contextmenu", (e) => e.preventDefault());
window.addEventListener("keydown", (e) => {
  if (e.key === "F5" || (e.ctrlKey && ["r", "f", "p", "g", "u"].includes(e.key.toLowerCase())))
    e.preventDefault();
});

export default mount(App, { target: document.getElementById("app")! });
