import { vitePlugin as remix } from "@remix-run/dev";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [
    remix({
      future: {
        v3_fetcherPersist: true,
        v3_relativeSplatPath: true,
        v3_throwAbortReason: true,
      },
      ssr: false,
    }),
    tsconfigPaths(),
  ],
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
