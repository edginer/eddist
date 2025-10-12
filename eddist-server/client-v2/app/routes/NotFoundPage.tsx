import { Link } from "react-router";

export default function NotFoundPage() {
  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="text-center">
        <h1 className="mb-4 text-6xl font-semibold text-gray-800 dark:text-gray-200">
          404
        </h1>
        <p className="mb-4 text-lg text-gray-600 dark:text-gray-400">
          Page not found
        </p>
        <p className="mb-8 text-gray-500 dark:text-gray-500">
          The page you're looking for doesn't exist.
        </p>
        <Link
          to="/"
          className="rounded-lg bg-blue-600 px-6 py-2.5 text-sm font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800"
        >
          Go back home
        </Link>
      </div>
    </div>
  );
}
