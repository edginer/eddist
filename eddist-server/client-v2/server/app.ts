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
const PUBLIC_REDIRECT_CACHE_CONTROL = "public, max-age=60, s-maxage=300";
const PRIVATE_REDIRECT_CACHE_CONTROL = "private, no-store";
const BOARD_KEY_PATTERN = /^[a-z0-9_-]{1,63}$/;
const THREAD_KEY_PATTERN = /^\d{10}$/;
// Keep private, authenticated, and operational namespaces out of shared redirect caching.
const NON_PUBLIC_ROUTE_PREFIXES = new Set([
  "api",
  "user",
  "auth-code",
  "re-auth",
  "health-check",
  "metrics",
  "test",
]);

const isPublicRedirectPath = (pathname: string): boolean => {
  const segments = pathname.split("/").filter(Boolean);
  const firstSegment = segments[0];

  if (!firstSegment || NON_PUBLIC_ROUTE_PREFIXES.has(firstSegment)) {
    return false;
  }

  if (segments.length === 1) {
    return (
      firstSegment === "terms" ||
      firstSegment === "notices" ||
      BOARD_KEY_PATTERN.test(firstSegment)
    );
  }

  if (segments.length === 2) {
    return firstSegment === "notices"
      ? segments[1].length > 0
      : BOARD_KEY_PATTERN.test(firstSegment) && THREAD_KEY_PATTERN.test(segments[1]);
  }

  return false;
};

app.use((req, res, next) => {
  const url = new URL(req.originalUrl, "http://localhost");

  if (url.pathname.length > 1 && url.pathname.endsWith("/")) {
    url.pathname = url.pathname.slice(0, -1);
    const canUseSharedCache =
      (req.method === "GET" || req.method === "HEAD") &&
      url.search === "" &&
      isPublicRedirectPath(url.pathname);
    res.setHeader(
      "Cache-Control",
      canUseSharedCache ? PUBLIC_REDIRECT_CACHE_CONTROL : PRIVATE_REDIRECT_CACHE_CONTROL,
    );
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
