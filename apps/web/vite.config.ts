import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: { port: 5173 },
  preview: { port: 4173 },
  build: {
    target: "es2022",
    // Le site vitrine doit rester léger : il est consulté surtout en 4G.
    cssMinify: "lightningcss",
    reportCompressedSize: true,
  },
});
