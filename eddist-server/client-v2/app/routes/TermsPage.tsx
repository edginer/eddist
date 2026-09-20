import { fetchTerms } from "~/api-client/terms";
import { PageMetadata } from "~/components/PageMetadata";
import { parseMarkdown } from "~/utils/markdown";
import { getCanonicalUrl } from "~/utils/metadata";
import type { Route } from "./+types/TermsPage";

export const headers = () => ({
  "Cache-Control": "s-maxage=3600",
});

export const loader = async ({ context, request }: Route.LoaderArgs) => {
  const baseUrl = context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL;

  const terms = await fetchTerms({ baseUrl });

  return {
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
    },
    canonicalUrl: getCanonicalUrl(context.PUBLIC_BASE_URL, request, "/terms"),
    terms,
  };
};

function TermsPage({ loaderData }: Route.ComponentProps) {
  const { eddistData, terms, canonicalUrl } = loaderData;

  return (
    <div className="bg-gray-50 dark:bg-gray-800">
      <PageMetadata
        title={`利用規約 - ${eddistData.bbsName}`}
        description={`${eddistData.bbsName}の利用規約、サービス利用上の注意事項です。`}
        siteName={eddistData.bbsName}
        canonicalUrl={canonicalUrl}
      />
      <div className="min-h-screen py-8">
        <div className="max-w-4xl mx-auto p-6">
          <div className="bg-white dark:bg-gray-900 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-8">
            <div className="mb-8">
              <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100 text-center">
                利用規約
              </h1>
            </div>
            <div className="space-y-6 text-gray-900 dark:text-gray-100">
              {parseMarkdown(terms.content)}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default TermsPage;
