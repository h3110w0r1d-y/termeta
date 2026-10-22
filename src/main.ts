import { initializeLocale } from "./i18n";
import { createApp } from "vue";
import { createPinia } from "pinia";
import "@xterm/xterm/css/xterm.css";
import "./assets/terminal-fonts.css";
import "./assets/base.css";
import { desktop } from "./services/desktop/client";

if (import.meta.env.PROD) {
  // 禁用 WebView 默认菜单中的刷新入口；保留事件传播供业务右键菜单处理。
  window.addEventListener("contextmenu", (event) => event.preventDefault(), {
    capture: true,
  });
}

initializeLocale();
const { default: Root } = desktop.isSettingsWindow()
  ? await import("./modules/settings/SettingsWindow.vue")
  : await import("./App.vue");
createApp(Root).use(createPinia()).mount("#app");
