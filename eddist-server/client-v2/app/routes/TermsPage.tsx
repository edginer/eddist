import type { Route } from "./+types/TermsPage";
import { fetchTerms } from "~/api-client/terms";
import { parseMarkdown } from "~/utils/markdown";

export const headers = () => ({
  "Cache-Control": "s-maxage=3600",
});

export const loader = async ({ context }: Route.LoaderArgs) => {
  const baseUrl =
    context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL;

  const terms = await fetchTerms({ baseUrl });

  return {
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
    },
    terms,
  };
};

const Meta = ({ bbsName }: { bbsName: string }) => (
  <>
    <title>利用規約 - {bbsName}</title>
    <meta property="og:title" content={`利用規約 - ${bbsName}`} />
  </>
);

function TermsPage({ loaderData }: Route.ComponentProps) {
  const { eddistData, terms } = loaderData;

  return (
    <div className="bg-gray-50">
      <Meta bbsName={eddistData.bbsName} />
      <div className="min-h-screen py-8">
        <div className="max-w-4xl mx-auto p-6">
          <div className="bg-white rounded-lg shadow-sm border p-8">
            <h1 className="text-3xl font-bold text-gray-900 mb-8 text-center">
              利用規約
            </h1>
            <div className="space-y-6">{parseMarkdown(terms.content)}</div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default TermsPage;
