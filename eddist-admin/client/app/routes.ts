import type { RouteConfig } from "@react-router/dev/routes";
import { flatRoutes } from "@react-router/fs-routes";

const routes: RouteConfig = [
  ...(await flatRoutes({
    rootDirectory: "./routes",
  })),
];

export default routes;
