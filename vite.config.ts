import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  clearScreen: false,
  // Externalize collaboration/Yjs deps (not used, only pulled by @yiitap/vue)
  build: {
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules")) {
            if (id.includes("@tiptap") || id.includes("@yiitap") || id.includes("prosemirror")) {
              return "vendor-editor";
            }
            if (id.includes("katex")) {
              return "vendor-katex";
            }
            if (id.includes("xterm") || id.includes("@xterm")) {
              return "vendor-xterm";
            }
            if (id.includes("mermaid")) {
              return "vendor-mermaid";
            }
            if (id.includes("diff") || id.includes("jsdiff")) {
              return "vendor-diff";
            }
            if (id.includes("jszip")) {
              return "vendor-zip";
            }
            if (id.includes("vue") || id.includes("pinia") || id.includes("@vue")) {
              return "vendor-vue";
            }
            return "vendor-libs";
          }
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
