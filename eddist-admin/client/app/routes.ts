import { type RouteConfig, route } from "@react-router/dev/routes";
import { flatRoutes } from "@react-router/fs-routes";

const routes: RouteConfig = [
  ...(await flatRoutes({
    rootDirectory: "app/routes",
  })),
];

export default routes;
