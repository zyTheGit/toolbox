import { createApp } from "vue";
import App from "./App.vue";
// Inter 可变字体随包自托管，离线可用；仅含西文子集，中文由回退字体渲染
import "@fontsource-variable/inter/wght.css";
import "./styles.css";

createApp(App).mount("#app");
