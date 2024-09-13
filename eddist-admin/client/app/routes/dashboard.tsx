import React, { Suspense, useState } from "react";
import { IoMdMenu } from "react-icons/io";
import { twMerge } from "tailwind-merge";
import { Spinner } from "flowbite-react";
import { Link, Outlet } from "@remix-run/react";

const Layout: React.FC = () => {
  const [isNavbarOpen, setIsNavbarOpen] = useState(false);
  return (
    <div className="flex h-screen flex-col sm:flex-row">
      <div className="hidden sm:block bg-gray-800 w-64">
        <div className="flex items-center justify-center mt-10">
          <Link
            to="/dashboard"
            className="text-white text-2xl mx-2 font-semibold"
          >
            Dashboard
          </Link>
        </div>
        <nav className="mt-10">
          <Link
            to="/dashboard/boards"
            className="flex items-center py-2 px-8 bg-gray-900 text-gray-400 hover:bg-gray-700"
          >
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
            <span className="mx-4 font-medium">Boards</span>
          </Link>
          <Link
            to="/dashboard/caps"
            className="flex items-center py-2 px-8 text-gray-400 hover:bg-gray-700"
          >
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
            <span className="mx-4 font-medium">Caps</span>
          </Link>
          <Link
            to="/dashboard/ngwords"
            className="flex items-center py-2 px-8 text-gray-400 hover:bg-gray-700"
          >
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
            <span className="mx-4 font-medium">Ng Words</span>
          </Link>
          <Link
            to="/dashboard/global"
            className="flex items-center py-2 px-8 text-gray-400 hover:bg-gray-700"
          >
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
            <span className="mx-4 font-medium">Global</span>
          </Link>
        </nav>
      </div>
      <div className="w-full flex bg-gray-900 text-gray-300 sm:hidden">
        <nav className="flex flex-col w-full">
          <div className="flex flex-row">
            <div className="flex-grow p-2 text-xl">Dashboard</div>
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
              !isNavbarOpen && "hidden"
            )}
          >
            <ul className="pt-2 text-lg text-blue-700 font-semibold">
              <li className="pl-2 border-b pb-1 border-slate-400 border-spacing-y-6">
                <Link
                  to="/dashboard/boards"
                  onClick={() => setIsNavbarOpen((x) => !x)}
                >
                  Boards
                </Link>
              </li>
              <li className="pl-2 border-b pb-1 border-slate-400 border-spacing-y-6">
                <Link
                  to="/dashboard/caps"
                  onClick={() => setIsNavbarOpen((x) => !x)}
                >
                  Caps
                </Link>
              </li>
              <li className="pl-2 border-b pb-1 border-slate-400 border-spacing-y-6">
                <Link
                  to="/dashboard/ngwords"
                  onClick={() => setIsNavbarOpen((x) => !x)}
                >
                  Ng Words
                </Link>
              </li>
              <li className="pl-2 border-slate-400">
                <Link
                  to="/dashboard/global"
                  onClick={() => setIsNavbarOpen((x) => !x)}
                >
                  Global
                </Link>
              </li>
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
