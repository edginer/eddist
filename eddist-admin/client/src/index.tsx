import React from "react";
import { createRoot } from "react-dom/client";

import Keycloak from "keycloak-js";
import { ReactKeycloakProvider } from "@react-keycloak/web";

import App from "./App";

const keycloak = new Keycloak({
  clientId: "eddist-admin-client",
  url: import.meta.env.DEV
    ? "http://localhost:8087/"
    : import.meta.env.VITE_EDDIST_ADMIN_AUTH_SERVER_URL,
  realm: "eddist-admin",
});

const container = document.getElementById("root") as HTMLElement;
const root = createRoot(container);

root.render(
  <ReactKeycloakProvider authClient={keycloak}>
    <App />
  </ReactKeycloakProvider>
);
