import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  worker: {
    format: "es",
  },
  server: {
    fs: {
      // Allow serving the wasm-pack output that lives under src/wasm.
      allow: [".."],
    },
  },
});
