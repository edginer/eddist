import type { MetaFunction } from "react-router";
import { Link } from "react-router";

export const meta: MetaFunction = () => {
  return [
    { title: "New Remix App" },
    { name: "description", content: "Welcome to Remix!" },
  ];
};

export default function Index() {
  return (
    <div className="font-sans p-4">
      <h1 className="text-3xl">Welcome to Eddist Admin Page</h1>
      <ul className="list-disc mt-4 pl-6 space-y-2">
        <li className="text-blue-700 underline visited:text-purple-900">
          <Link
            to="/dashboard"
            className="text-blue-700 underline visited:text-purple-900"
          >
            Admin Dashboard
          </Link>
        </li>
        {
          // TODO: Add ArgoCD, Grafana links
        }
      </ul>
    </div>
  );
}
