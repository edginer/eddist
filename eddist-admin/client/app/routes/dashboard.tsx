import React, { Suspense, useState } from "react";
import { IoMdMenu } from "react-icons/io";
import { twMerge } from "tailwind-merge";
import { Spinner } from "flowbite-react";
import { Link, Outlet, useLocation } from "react-router";

const NAV_ITEMS = [
  { kind: "boards", label: "Boards" },
  { kind: "caps", label: "Caps" },
  { kind: "ngwords", label: "Ng Words" },
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
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M4 6h16M4 12h16M4 18h16"
    />
  </svg>
);

const Layout: React.FC = () => {
  const location = useLocation();
  const navBarSection = location.pathname.split("/")[2];
  const [isNavbarOpen, setIsNavbarOpen] = useState(false);

  return (
    <div className="flex h-screen flex-col sm:flex-row">
      <div className="hidden sm:block bg-gray-800 w-64">
        <div className="flex items-center justify-center mt-10">
          <Link
            to="/dashboard"
            className="text-white text-2xl mx-2 font-semibold"
          >
            Eddist Dashboard
          </Link>
        </div>
        <nav className="mt-10">
          {NAV_ITEMS.map((item) => (
            <Link
              key={item.kind}
              to={`/dashboard/${item.kind}`}
              className={twMerge(
                "flex items-center py-2 px-8 hover:bg-gray-700",
                navBarSection === item.kind
                  ? "bg-gray-900 text-gray-400"
                  : "text-gray-400",
              )}
            >
              <Hamburger />
              <span className="mx-4 font-medium">{item.label}</span>
            </Link>
          ))}
        </nav>
      </div>
      <div className="w-full flex bg-gray-900 text-gray-300 sm:hidden">
        <nav className="flex flex-col w-full">
          <div className="flex flex-row">
            <div className="grow p-2 text-xl">Dashboard</div>
            <button
              data-collapse-toggle="navbar-dropdown"
              type="button"
              className="inline-flex items-center p-2 ms-3 w-10 h-10 justify-center text-sm text-gray-500 rounded-lg sm:hidden hover:bg-gray-100 focus-visible:outline-none focus-visible:ring-2 focus:ring-gray-200 dark:text-gray-400 dark:hover:bg-gray-700 dark:focus-visible:ring-gray-600"
              aria-controls="navbar-dropdown"
              aria-expanded="false"
              onClick={() => setIsNavbarOpen((x) => !x)}
            >
              <span className="sr-only">Open main menu</span>
              <IoMdMenu className="w-8 h-8" />
            </button>
          </div>
          <div
            className={twMerge(
              " bg-gray-50 text-black z-10 flex flex-col h-screen",
              !isNavbarOpen && "hidden",
            )}
          >
            <ul className="pt-2 text-lg text-blue-700 font-semibold">
              {NAV_ITEMS.map((item, idx) => (
                <li
                  key={item.kind}
                  className={twMerge(
                    "pl-2 border-slate-400",
                    idx < NAV_ITEMS.length - 1 &&
                      "border-b pb-1 border-spacing-y-6",
                  )}
                >
                  <Link
                    to={`/dashboard/${item.kind}`}
                    onClick={() => setIsNavbarOpen((x) => !x)}
                  >
                    {item.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        </nav>
      </div>
      <div className="flex-1 overflow-scroll overflow-x-hidden">
        <Suspense
          fallback={
            <div className="h-full w-full flex items-center justify-center">
              <Spinner size="xl" />
            </div>
          }
        >
          <Outlet />
        </Suspense>
      </div>
    </div>
  );
};

export default Layout;
