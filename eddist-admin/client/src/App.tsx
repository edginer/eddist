import {
  Refine,
  WelcomePage,
  Authenticated,
  AuthProvider,
} from "@refinedev/core";
import { DevtoolsPanel, DevtoolsProvider } from "@refinedev/devtools";
import { RefineKbar, RefineKbarProvider } from "@refinedev/kbar";

import {
  AuthPage,
  ErrorComponent,
  RefineSnackbarProvider,
  ThemedLayoutV2,
  useNotificationProvider,
} from "@refinedev/mui";

import dataProvider, { GraphQLClient } from "@refinedev/graphql";
import CssBaseline from "@mui/material/CssBaseline";
import GlobalStyles from "@mui/material/GlobalStyles";
import { BrowserRouter, Route, Routes, Outlet } from "react-router-dom";
import routerBindings, {
  UnsavedChangesNotifier,
  DocumentTitleHandler,
} from "@refinedev/react-router-v6";
import axios from "axios";
import { useKeycloak } from "@react-keycloak/web";
import { ColorModeContextProvider } from "./contexts/color-mode";
import { Header } from "./components/header";
import { Login } from "./pages/login";
import { Boards } from "./pages/boards";
import { ThreadList } from "./pages/threadList";
import { Thread } from "./pages/thread";

const API_URL = import.meta.env.DEV
  ? "http://localhost:8081/api/graphql"
  : "/api/graphql";

const client = new GraphQLClient(API_URL);
const gqlDataProvider = dataProvider(client);

const Title = () => {
  return <span>Refine</span>;
};

function App() {
  const { keycloak, initialized } = useKeycloak();

  if (!initialized) {
    return <div>Loading...</div>;
  }

  const authProvider: AuthProvider = {
    login: async () => {
      const urlSearchParams = new URLSearchParams(window.location.search);
      const { to } = Object.fromEntries(urlSearchParams.entries());
      await keycloak.login({
        redirectUri: to ? `${window.location.origin}${to}` : undefined,
      });
      return {
        success: true,
      };
    },
    logout: async () => {
      try {
        await keycloak.logout({
          redirectUri: window.location.origin,
        });
        return {
          success: true,
          redirectTo: "/login",
        };
      } catch (error) {
        return {
          success: false,
          error: new Error("Logout failed"),
        };
      }
    },
    onError: async (error) => {
      console.error(error);
      return { error };
    },
    check: async () => {
      try {
        const { token } = keycloak;
        if (token) {
          axios.defaults.headers.common = {
            Authorization: `Bearer ${token}`,
          };
          return {
            authenticated: true,
          };
        } else {
          return {
            authenticated: false,
            logout: true,
            redirectTo: "/login",
            error: {
              message: "Check failed",
              name: "Token not found",
            },
          };
        }
      } catch (error) {
        return {
          authenticated: false,
          logout: true,
          redirectTo: "/login",
          error: {
            message: "Check failed",
            name: "Token not found",
          },
        };
      }
    },
    getPermissions: async () => null,
    getIdentity: async () => {
      if (keycloak?.tokenParsed) {
        return {
          name: keycloak.tokenParsed.family_name,
        };
      }
      return null;
    },
  };

  return (
    <BrowserRouter>
      <RefineKbarProvider>
        <ColorModeContextProvider>
          <CssBaseline />
          <GlobalStyles styles={{ html: { WebkitFontSmoothing: "auto" } }} />
          <RefineSnackbarProvider>
            <DevtoolsProvider>
              <Refine
                dataProvider={gqlDataProvider}
                notificationProvider={useNotificationProvider}
                routerProvider={routerBindings}
                authProvider={authProvider}
                options={{
                  syncWithLocation: true,
                  warnWhenUnsavedChanges: true,
                  useNewQueryKeys: true,
                  projectId: "ahO5GS-09bDp3-INFSfL",
                  disableTelemetry: true,
                }}
                resources={[
                  {
                    name: "Boards",
                    list: "/boards",
                    create: "/boards/create",
                  },
                  {
                    name: "Threads",
                    list: "/boards/:boardKey",
                    show: "/boards/:boardKey/:threadKey",
                    meta: {
                      parent: "Boards",
                      canDelete: true,
                    },
                  },
                  {
                    name: "Responses",
                    edit: "/boards/:boardKey/:threadKey/:responseId",
                    show: "/boards/:boardKey/:threadKey/:responseId",
                    meta: {
                      parent: "Threads",
                      canDelete: true,
                    },
                  },
                ]}
              >
                <Routes>
                  <Route
                    element={
                      <ThemedLayoutV2 Header={Header} Title={Title}>
                        <Outlet />
                      </ThemedLayoutV2>
                    }
                  >
                    <Route index element={<></>} />
                    <Route path="/boards" element={<Boards />} />
                    <Route path="/boards/:boardKey" element={<ThreadList />} />
                    <Route
                      path="/boards/:boardKey/:threadKey"
                      element={<Thread />}
                    />
                    <Route path="/login" element={<Login />} />
                    <Route path="*" element={<ErrorComponent />} />
                  </Route>
                </Routes>
                <RefineKbar />
                <UnsavedChangesNotifier />
                <DocumentTitleHandler />
              </Refine>
              {/* <DevtoolsPanel /> */}
            </DevtoolsProvider>
          </RefineSnackbarProvider>
        </ColorModeContextProvider>
      </RefineKbarProvider>
    </BrowserRouter>
  );
}

export default App;
