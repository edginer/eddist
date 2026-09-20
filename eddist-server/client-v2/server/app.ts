import "react-router";
import { createRequestHandler } from "@react-router/express";
import express from "express";

declare module "react-router" {
  interface AppLoadContext {
    EDDIST_SERVER_URL: string;
    BBS_NAME: string;
    PUBLIC_BASE_URL: string;
  }
}

export const app = express();
const EDDIST_SERVER_URL = process.env.EDDIST_SERVER_URL ?? "http://localhost:8080";
const BBS_NAME = process.env.BBS_NAME ?? "エッヂ掲示板";
// Override this when deploying the client under a different public origin.
const PUBLIC_BASE_URL = process.env.PUBLIC_BASE_URL ?? process.env.EDDIST_SERVER_URL ?? "";

app.use((req, res, next) => {
  const url = new URL(req.originalUrl, "http://localhost");

  if (url.pathname.length > 1 && url.pathname.endsWith("/")) {
    url.pathname = url.pathname.slice(0, -1);
    res.redirect(308, `${url.pathname}${url.search}`);
    return;
  }

  next();
});

app.use(
  createRequestHandler({
    build: () => import("virtual:react-router/server-build"),
    getLoadContext: () => ({
      EDDIST_SERVER_URL: EDDIST_SERVER_URL,
      BBS_NAME: BBS_NAME,
      PUBLIC_BASE_URL: PUBLIC_BASE_URL,
    }),
  }),
);
