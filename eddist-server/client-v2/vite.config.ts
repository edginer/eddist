import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig, type UserConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";
import flowbiteReact from "flowbite-react/plugin/vite";

export default defineConfig(
  ({ isSsrBuild }): UserConfig => ({
    plugins: [tailwindcss(), reactRouter(), tsconfigPaths(), flowbiteReact()],
    build: {
      rollupOptions: {
        output: {
          entryFileNames: `assets/[name].js`,
          chunkFileNames: `assets/[name].js`,
          assetFileNames: `assets/[name].[ext]`,
        },
        input: isSsrBuild
          ? {
              input: "./server/app.ts",
            }
          : undefined,
      },
    },
    server: {
      host: "0.0.0.0",
      port: 5173,
      allowedHosts: ["host.docker.internal"],
      proxy: {
        "/api": {
          target: process.env.VITE_PROXY_TARGET || "http://localhost:8080",
          changeOrigin: true,
        },
        "^/.+/subject.txt": {
          // target: "https://bbs.eddibb.cc",
          target: process.env.VITE_PROXY_TARGET || "http://localhost:8080",
          changeOrigin: true,
        },
        "^/.+/dat/.+\\.dat": {
          // target: "https://bbs.eddibb.cc",
          target: process.env.VITE_PROXY_TARGET || "http://localhost:8080",
          changeOrigin: true,
        },
        "/auth-code": {
          // target: "https://bbs.eddibb.cc",
          target: process.env.VITE_PROXY_TARGET || "http://localhost:8080",
          changeOrigin: true,
        },
        "/test/bbs.cgi": {
          // target: "https://bbs.eddibb.cc",
          target: process.env.VITE_PROXY_TARGET || "http://localhost:8080",
          changeOrigin: true,
        },
      },
    },
  })
);
