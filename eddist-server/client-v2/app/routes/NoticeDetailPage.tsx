import { Link } from "react-router";
import { Footer } from "~/components/Footer";
import { fetchNoticeBySlug } from "~/api-client/notice";
import { parseMarkdown } from "~/utils/markdown";
import type { Route } from "./+types/NoticeDetailPage";

export const headers = () => ({
  "Cache-Control": "s-maxage=3600",
});

export const loader = async ({ context, params }: Route.LoaderArgs) => {
  const baseUrl = context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL;

  const notice = await fetchNoticeBySlug({ baseUrl, slug: params.slug });

  return {
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
    },
    notice,
  };
};

const Meta = ({ title, bbsName }: { title: string; bbsName: string }) => (
  <>
    <title>{`${title} - ${bbsName}`}</title>
    <meta property="og:title" content={`${title} - ${bbsName}`} />
  </>
);

function NoticeDetailPage({ loaderData }: Route.ComponentProps) {
  const { eddistData, notice } = loaderData;

  return (
    <div className="min-h-[calc(100vh-1rem)] lg:min-h-[calc(100vh-4rem)] flex flex-col dark:text-gray-100">
      <Meta title={notice.title} bbsName={eddistData.bbsName} />
      <article className="flex-1">
        <header>
          <h1 className="text-3xl lg:text-5xl">{notice.title}</h1>
          <p className="text-gray-600 dark:text-gray-400 text-sm mt-2">
            {new Date(notice.published_at).toLocaleDateString("ja-JP", {
              year: "numeric",
              month: "long",
              day: "numeric",
            })}
          </p>
          <div className="flex gap-4 mt-4">
            <Link to="/notices" className="text-blue-500 text-sm">
              ← お知らせ一覧に戻る
            </Link>
            <Link to="/" className="text-blue-500 text-sm">
              トップページ
            </Link>
          </div>
        </header>
        <section className="py-4 pt-8">
          <div className="max-w-none">{parseMarkdown(notice.content)}</div>
        </section>
      </article>
      <Footer />
    </div>
  );
}

export default NoticeDetailPage;
