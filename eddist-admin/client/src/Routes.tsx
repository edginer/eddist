import { Authenticated, ErrorComponent, useLogin } from "@refinedev/core";
import { ThemedLayoutV2 } from "@refinedev/mui";
import React from "react";
import { Navigate, Outlet, Route, Routes } from "react-router-dom";
import { Header } from "./components/header";
import { Boards } from "./pages/boards";
import { ThreadList } from "./pages/threadList";
import { Thread } from "./pages/thread";
import { Login } from "./pages/login";

const Title = () => {
  return <span>Refine</span>;
};

const BaseRoutes = () => {
  const { ...t } = useLogin();
  console.log(t);

  return (
    <Routes>
      <Route
        path="/login"
        element={
          <ThemedLayoutV2 Header={Header} Title={Title}>
            <Login />
          </ThemedLayoutV2>
        }
      />
      <Route path="*" element={<ErrorComponent />} />
      {/* <Authenticated
        key="authentication-root"
        fallback={<Navigate replace to="/login" />}
        loading={isLoadingLogin}
      > */}
      <Route
        path="/dashboard"
        element={
          <Authenticated
            key="authentication-root"
            fallback={<Navigate replace to="/login" />}
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
        <Route path="boards/:boardKey/:threadKey" element={<Thread />} />
      </Route>
      {/* </Authenticated> */}
    </Routes>
  );
};

export default BaseRoutes;
