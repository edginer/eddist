import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:8080",
        changeOrigin: true,
      },
      "^/.+/subject.txt": {
        // target: "https://bbs.eddibb.cc",
        target: "http://localhost:8080",
        changeOrigin: true,
      },
      "^/.+/dat/.+\\.dat": {
        // target: "https://bbs.eddibb.cc",
        target: "http://localhost:8080",
        changeOrigin: true,
      },
      "/auth-code": {
        // target: "https://bbs.eddibb.cc",
        target: "http://localhost:8080",
        changeOrigin: true,
      },
      "/test/bbs.cgi": {
        // target: "https://bbs.eddibb.cc",
        target: "http://localhost:8080",
        changeOrigin: true,
      },
    },
  },
});
