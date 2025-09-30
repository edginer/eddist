import "react-router";
import { createRequestHandler } from "@react-router/express";
import express from "express";

declare module "react-router" {
  interface AppLoadContext {
    EDDIST_SERVER_URL: string;
    BBS_NAME: string;
    ENABLE_USER_REGISTRATION: boolean;
  }
}

export const app = express();
const EDDIST_SERVER_URL =
  process.env.EDDIST_SERVER_URL ?? "http://localhost:8080";
const BBS_NAME = process.env.BBS_NAME ?? "エッヂ掲示板";
const ENABLE_USER_REGISTRATION =
  process.env.ENABLE_USER_REGISTRATION === "true";

app.use(
  createRequestHandler({
    build: () => import("virtual:react-router/server-build"),
    getLoadContext: () => ({
      EDDIST_SERVER_URL: EDDIST_SERVER_URL,
      BBS_NAME: BBS_NAME,
      ENABLE_USER_REGISTRATION: ENABLE_USER_REGISTRATION,
    }),
  })
);
