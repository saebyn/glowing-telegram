import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  define: {
    "process.env": process.env,
  },
  server: {
    host: true,
    strictPort: true,
    port: 3000,
    watch: {
      usePolling: true,
      interval: 100,
    },
    hmr: false,
  },
  base: "./",
});
