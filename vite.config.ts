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
        // UI 库单独分包：避免单 chunk 超限，且库代码变化少、缓存友好。
        manualChunks(id) {
          if (!id.includes("node_modules")) return undefined;
          if (
            id.includes("naive-ui") ||
            id.includes("vueuc") ||
            id.includes("css-render") ||
            id.includes("vdirs") ||
            id.includes("vfonts") ||
            id.includes("treemir") ||
            id.includes("async-validator")
          ) {
            return "naive-ui";
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
