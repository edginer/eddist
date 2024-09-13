import {
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
} from "@remix-run/react";
import { Spinner } from "flowbite-react";
import "./tailwind.css";
import { ErrorBoundary } from "react-error-boundary";
import { Suspense, useEffect } from "react";
import { ToastContainer } from "react-toastify";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const reactQueryClient = new QueryClient({});

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
      </head>
      <body>
        {children}
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function App() {
  useEffect(() => {
    const f = async () => {
      const checkResult = await fetch("/auth/check");
      const checkResultJson = await checkResult.json();
      if (!checkResultJson) {
        window.location.href = "/login";
      }
    };

    let cancelled = false;

    // execute infinite loop with sleep 10s using setTimeout
    const fSetTimeout = () => {
      f();

      if (!cancelled) {
        setTimeout(fSetTimeout, 10000);
      }
    };
    fSetTimeout();

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <QueryClientProvider client={reactQueryClient}>
      <ErrorBoundary
        fallbackRender={(p) => <div>Error: {p.error.message}</div>}
      >
        <Suspense
          fallback={<Spinner size="xl" className="fixed inset-0 m-auto" />}
        >
          <Outlet />
        </Suspense>
      </ErrorBoundary>
      <ToastContainer theme="colored" />
    </QueryClientProvider>
  );
}
