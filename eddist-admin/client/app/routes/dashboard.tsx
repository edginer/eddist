import { Spinner } from "flowbite-react";
import type React from "react";
import { Suspense, useEffect, useState } from "react";
import { IoMdClose, IoMdMenu } from "react-icons/io";
import { Link, Outlet, useLocation } from "react-router";
import { twMerge } from "tailwind-merge";

const NAV_ITEMS = [
  { kind: "boards", label: "Boards" },
  { kind: "caps", label: "Caps" },
  { kind: "ngwords", label: "Ng Words" },
  { kind: "idps", label: "IdPs" },
  { kind: "notices", label: "Notices" },
  { kind: "terms", label: "Terms" },
  { kind: "captcha-configs", label: "Captcha Configs" },
  { kind: "server-settings", label: "Server Settings" },
  { kind: "global", label: "Global" },
  { kind: "authed-token", label: "Authed Token" },
  { kind: "users", label: "Users" },
  { kind: "restriction-rules", label: "Restriction Rules" },
] as const;

const Hamburger = () => (
  <svg
    className="h-6 w-6 text-gray-300"
    fill="none"
    viewBox="0 0 24 24"
    stroke="currentColor"
    aria-hidden="true"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M4 6h16M4 12h16M4 18h16"
    />
  </svg>
);

const Navigation = ({
  isMobile = false,
  onNavigate,
}: {
  isMobile?: boolean;
  onNavigate?: () => void;
}) => {
  const location = useLocation();

  return (
    <div className={isMobile ? "grid gap-1" : "mt-8 space-y-1 px-3"}>
      {NAV_ITEMS.map((item) => {
        const isActive = location.pathname.startsWith(`/dashboard/${item.kind}`);

        return (
          <Link
            key={item.kind}
            to={`/dashboard/${item.kind}`}
            onClick={onNavigate}
            className={twMerge(
              "flex min-h-11 items-center rounded-lg px-3 text-sm font-medium transition-colors",
              isMobile
                ? isActive
                  ? "bg-blue-50 text-blue-700"
                  : "text-gray-700 hover:bg-gray-100"
                : isActive
                  ? "bg-gray-900 text-gray-100"
                  : "text-gray-400 hover:bg-gray-700 hover:text-white",
            )}
          >
            {!isMobile && <Hamburger />}
            <span className={isMobile ? "" : "mx-4"}>{item.label}</span>
          </Link>
        );
      })}
    </div>
  );
};

const Layout: React.FC = () => {
  const location = useLocation();
  const [isNavbarOpen, setIsNavbarOpen] = useState(false);

  useEffect(() => {
    if (location.pathname) {
      setIsNavbarOpen(false);
    }
  }, [location.pathname]);

  return (
    <div className="relative flex h-dvh min-h-0 flex-col bg-gray-100 sm:flex-row">
      <aside className="hidden h-full w-64 shrink-0 flex-col overflow-y-auto bg-gray-800 sm:flex">
        <div className="flex min-h-20 items-center border-b border-gray-700 px-5">
          <Link to="/dashboard" className="text-xl font-semibold text-white">
            Eddist Dashboard
          </Link>
        </div>
        <Navigation />
      </aside>

      <div className="relative flex min-h-0 min-w-0 flex-1 flex-col">
        <header className="relative z-50 flex h-14 shrink-0 items-center justify-between bg-gray-900 px-4 text-gray-100 sm:hidden">
          <Link to="/dashboard" className="text-lg font-semibold">
            Eddist Dashboard
          </Link>
          <button
            type="button"
            className="inline-flex min-h-10 min-w-10 items-center justify-center rounded-lg text-gray-300 hover:bg-gray-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
            aria-controls="dashboard-navigation"
            aria-expanded={isNavbarOpen}
            aria-label={isNavbarOpen ? "Close navigation" : "Open navigation"}
            onClick={() => setIsNavbarOpen((x) => !x)}
          >
            {isNavbarOpen ? <IoMdClose className="h-7 w-7" /> : <IoMdMenu className="h-7 w-7" />}
          </button>
        </header>

        {isNavbarOpen && (
          <>
            <button
              type="button"
              className="fixed inset-0 z-40 bg-gray-900/50 sm:hidden"
              aria-label="Close navigation"
              onClick={() => setIsNavbarOpen(false)}
            />
            <nav
              id="dashboard-navigation"
              className="absolute inset-x-0 top-14 z-50 max-h-[calc(100dvh-3.5rem)] overflow-y-auto border-b border-gray-200 bg-white p-3 shadow-xl sm:hidden"
            >
              <Navigation isMobile onNavigate={() => setIsNavbarOpen(false)} />
            </nav>
          </>
        )}

        <main className="min-h-0 min-w-0 flex-1 overflow-x-auto overflow-y-auto overscroll-contain">
          <Suspense
            fallback={
              <div className="h-full w-full flex items-center justify-center">
                <Spinner size="xl" />
              </div>
            }
          >
            <Outlet />
          </Suspense>
        </main>
      </div>
    </div>
  );
};

export default Layout;
