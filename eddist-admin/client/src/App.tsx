import { Refine, Authenticated, AuthProvider } from "@refinedev/core";
import { DevtoolsPanel, DevtoolsProvider } from "@refinedev/devtools";
import { RefineKbar, RefineKbarProvider } from "@refinedev/kbar";

import {
  ErrorComponent,
  RefineSnackbarProvider,
  ThemedLayoutV2,
  useNotificationProvider,
} from "@refinedev/mui";

import dataProvider, { GraphQLClient } from "@refinedev/graphql";
import CssBaseline from "@mui/material/CssBaseline";
import GlobalStyles from "@mui/material/GlobalStyles";
import {
  BrowserRouter,
  Route,
  Routes,
  Outlet,
  Navigate,
} from "react-router-dom";
import routerBindings, {
  UnsavedChangesNotifier,
  DocumentTitleHandler,
} from "@refinedev/react-router-v6";
import { ColorModeContextProvider } from "./contexts/color-mode";
import { Header } from "./components/header";
import { Login } from "./pages/login";
import { Boards } from "./pages/boards";
import { ThreadList } from "./pages/threadList";
import { Thread } from "./pages/thread";
import BaseRoutes from "./Routes";

const API_URL = import.meta.env.DEV
  ? "http://localhost:8081/api/graphql"
  : "/api/graphql";

const client = new GraphQLClient(API_URL);
const gqlDataProvider = dataProvider(client);

const Title = () => {
  return <span>Refine</span>;
};

function App() {
  const authProvider: AuthProvider = {
    login: async () => {
      window.location.href = "/login";
      return { redirectTo: "/login", success: true };
    },
    logout: async () => {
      await fetch("/auth/logout");
      return {
        success: true,
        redirectTo: "/login",
      };
    },
    onError: async (error) => {
      console.error(error);
      return { error };
    },
    check: async () => {
      if (import.meta.env.DEV) {
        return { authenticated: true };
      }
      const json = await fetch("/auth/check");
      const data = await json.json();
      if (data.error) {
        return { authenticated: false, redirectTo: "/login" };
      }
      return { authenticated: data };
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
                    list: "/dashboard/boards",
                    create: "/dashboard/boards/create",
                  },
                  {
                    name: "Threads",
                    list: "/dashboard/boards/:boardKey",
                    show: "/dashboard/boards/:boardKey/:threadKey",
                    meta: {
                      parent: "Boards",
                      canDelete: true,
                    },
                  },
                  {
                    name: "Responses",
                    edit: "/dashboard/boards/:boardKey/:threadKey/:responseId",
                    show: "/dashboard/boards/:boardKey/:threadKey/:responseId",
                    meta: {
                      parent: "Threads",
                      canDelete: true,
                    },
                  },
                ]}
              >
                <BaseRoutes />
                <Routes>
                  <Route path="/login" element={<Login />} />
                  <Route path="*" element={<ErrorComponent />} />
                  <Route
                    path="/dashboard"
                    element={
                      <Authenticated
                        key="authentication-root"
                        fallback={<Navigate replace to="/login" />}
                        // loading={isLoadingLogin}
                      >
                        <ThemedLayoutV2 Header={Header} Title={Title}>
                          <Outlet />
                        </ThemedLayoutV2>
                      </Authenticated>
                    }
                  >
                    <Route index element={<></>} />
                    <Route path="boards" element={<Boards />} />
                    <Route path="boards/:boardKey" element={<ThreadList />} />
                    <Route
                      path="boards/:boardKey/:threadKey"
                      element={<Thread />}
                    />
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
