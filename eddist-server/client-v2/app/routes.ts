import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/TopPage.tsx"),
  route("notices", "routes/NoticeListPage.tsx"),
  route("notices/:id", "routes/NoticeDetailPage.tsx"),
  route(":boardKey", "routes/ThreadListPage.tsx"),
  route(":boardKey/:threadKey", "routes/ThreadPage.tsx"),
  route("*", "routes/NotFoundPage.tsx"),
] satisfies RouteConfig;
