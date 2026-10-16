import { createApp } from "vue";
import { createPinia } from "pinia";
import "@xterm/xterm/css/xterm.css";
import "./assets/terminal-fonts.css";
import "./assets/base.css";
import { desktop } from "./services/desktop/client";
const { default: Root } = desktop.isSettingsWindow()
  ? await import("./modules/settings/SettingsWindow.vue")
  : await import("./App.vue");
createApp(Root).use(createPinia()).mount("#app");
