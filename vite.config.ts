import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  build: {
    // naive-ui 完整 UI 库约 700 KB（已 tree-shaking），业务分包后属合理体积，
    // 阈值上调以避免对该库 chunk 的误报；业务代码超限仍会警告。
    chunkSizeWarningLimit: 750,
    rollupOptions: {
      output: {
        // 只固定稳定的底层依赖分组；naive-ui 组件跟随各自视图分包，
        // 配合视图懒加载实现首次进入才加载对应组件，避免整库前置。
        manualChunks(id) {
          if (!id.includes("node_modules")) return undefined;
          if (id.includes("date-fns")) return "date-fns";
          if (id.includes("lodash-es") || id.includes("/lodash/")) return "lodash";
          if (id.includes("/node_modules/vue/") || id.includes("/node_modules/@vue/")) {
            return "vue";
          }
          if (
            id.includes("vueuc") ||
            id.includes("css-render") ||
            id.includes("vdirs") ||
            id.includes("vooks") ||
            id.includes("treemate") ||
            id.includes("seemly") ||
            id.includes("evtd") ||
            id.includes("async-validator")
          ) {
            return "naive-ui-core";
          }
          return undefined;
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**", "**/prototype/**"],
    },
  },
});
