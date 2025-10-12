import React, { Suspense, useState } from "react";
import { IoMdMenu } from "react-icons/io";
import { twMerge } from "tailwind-merge";
import { Spinner } from "flowbite-react";
import { Link, Outlet, useLocation } from "react-router";

type NavBarSectionKind =
  | "boards"
  | "caps"
  | "ngwords"
  | "global"
  | "authed-token"
  | "users"
  | "restriction-rules";

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

const NavBarSection = ({
  selected,
  kind,
}: {
  selected: boolean;
  kind: NavBarSectionKind;
}) => {
  let displayText = "";
  switch (kind) {
    case "boards":
      displayText = "Boards";
      break;
    case "caps":
      displayText = "Caps";
      break;
    case "ngwords":
      displayText = "Ng Words";
      break;
    case "global":
      displayText = "Global";
      break;
    case "authed-token":
      displayText = "Authed Token";
      break;
    case "users":
      displayText = "Users";
      break;
    case "restriction-rules":
      displayText = "Restriction Rules";
      break;
  }

  return (
    <Link
      to={`/dashboard/${kind}`}
      className={twMerge(
        "flex items-center py-2 px-8 hover:bg-gray-700",
        selected ? "bg-gray-900 text-gray-400" : "text-gray-400"
      )}
    >
      <Hamburger />
      <span className="mx-4 font-medium">{displayText}</span>
    </Link>
  );
};

const Layout: React.FC = () => {
  const location = useLocation();

  const navBarSection = location.pathname.split("/")[2] as NavBarSectionKind;

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
          <NavBarSection selected={navBarSection === "boards"} kind="boards" />
          <NavBarSection selected={navBarSection === "caps"} kind="caps" />
          <NavBarSection
            selected={navBarSection === "ngwords"}
            kind="ngwords"
          />
          <NavBarSection selected={navBarSection === "global"} kind="global" />
          <NavBarSection
            selected={navBarSection === "authed-token"}
            kind="authed-token"
          />
          <NavBarSection selected={navBarSection === "users"} kind="users" />
          <NavBarSection selected={navBarSection === "restriction-rules"} kind="restriction-rules" />
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
              <li className="pl-2 border-b pb-1 border-slate-400 border-spacing-y-6">
                <Link
                  to="/dashboard/global"
                  onClick={() => setIsNavbarOpen((x) => !x)}
                >
                  Global
                </Link>
              </li>
              <li className="pl-2 border-b pb-1 border-slate-400 border-spacing-y-6">
                <Link
                  to="/dashboard/authed-token"
                  onClick={() => setIsNavbarOpen((x) => !x)}
                >
                  Authed Token
                </Link>
              </li>
              <li className="pl-2 border-b pb-1 border-slate-400 border-spacing-y-6">
                <Link
                  to="/dashboard/users"
                  onClick={() => setIsNavbarOpen((x) => !x)}
                >
                  Users
                </Link>
              </li>
              <li className="pl-2 border-slate-400">
                <Link
                  to="/dashboard/restriction-rules"
                  onClick={() => setIsNavbarOpen((x) => !x)}
                >
                  Restriction Rules
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
