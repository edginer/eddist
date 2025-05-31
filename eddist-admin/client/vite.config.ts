import { reactRouter } from "@react-router/dev/vite";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [reactRouter(), tsconfigPaths()],
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:8081/",
        changeOrigin: true,
      },
      "/auth": {
        target: "http://localhost:8081/",
        changeOrigin: true,
      },
      "/login": {
        target: "http://localhost:8081/",
        changeOrigin: true,
      },
    },
  },
});
