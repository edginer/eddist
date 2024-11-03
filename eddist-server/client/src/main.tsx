/* eslint-disable react-refresh/only-export-components */
import React from "react";
import { StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { ErrorBoundary } from "react-error-boundary";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const TopPage = React.lazy(() => import("./pages/TopPage.tsx"));
const ThreadListPage = React.lazy(() => import("./pages/ThreadListPage.tsx"));
const ThreadPage = React.lazy(() => import("./pages/ThreadPage.tsx"));

// React-router-dom v6
// Pages
// Top page (Boards, link to individual board, auth-code, terms of service) (/)
// ├─ Board page (ThreadList, link to individual thread, create thread modal) (/:boardKey)
// │  └─ Thread page (Thread, post form) (/:boardKey/:threadKey)
// ├─ Auth page (is not feature of this app) (/auth-code, link is not React Router Link)
// └─ Terms of service page (/terms)

const router = createBrowserRouter([
  {
    path: "/",
    element: <TopPage></TopPage>,
  },
  {
    path: "/:boardKey",
    element: <ThreadListPage></ThreadListPage>,
  },
  {
    path: "/:boardKey/:threadKey",
    element: <ThreadPage></ThreadPage>,
  },
]);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <div className="container p-4 lg:px-16 lg:pt-12 mx-auto">
      <ErrorBoundary fallback={<div>Error!</div>}>
        <Suspense fallback={<div>Loading...</div>}>
          <QueryClientProvider client={new QueryClient()}>
            <RouterProvider router={router} />
          </QueryClientProvider>
        </Suspense>
      </ErrorBoundary>
    </div>
  </StrictMode>
);
