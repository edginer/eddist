import "react-router";
import { createRequestHandler } from "@react-router/express";
import express from "express";

declare module "react-router" {
  interface AppLoadContext {
    EDDIST_SERVER_URL: string;
  }
}

export const app = express();
const EDDIST_SERVER_URL =
  process.env.EDDIST_SERVER_URL ?? "http://localhost:8080";

app.use(
  createRequestHandler({
    build: () => import("virtual:react-router/server-build"),
    getLoadContext: () => ({
      EDDIST_SERVER_URL: EDDIST_SERVER_URL,
    }),
  })
);
