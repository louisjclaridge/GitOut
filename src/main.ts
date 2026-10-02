import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// In a plain browser during development, fake the Rust backend.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window && "invoke" in (window as any).__TAURI_INTERNALS__)) {
  (await import("./dev/mock")).installMock();
}

const app = mount(App, { target: document.getElementById("app")! });

export default app;
