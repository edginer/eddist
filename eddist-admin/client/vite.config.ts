import { reactRouter } from "@react-router/dev/vite";
import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import tsconfigPaths from "vite-tsconfig-paths";
import flowbiteReact from "flowbite-react/plugin/vite";

export default defineConfig({
  plugins: [reactRouter(), tsconfigPaths(), tailwindcss(), flowbiteReact()],
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
