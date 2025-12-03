import { Link, useSearchParams } from "react-router";
import type { Route } from "./+types/NoticeListPage";
import { fetchNotices, type NoticeListItem } from "~/api-client/notice";

export const loader = async ({ context, request }: Route.LoaderArgs) => {
  const baseUrl =
    context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL;

  const url = new URL(request.url);
  const page = parseInt(url.searchParams.get("page") || "0", 10);
  const limit = 10;

  return {
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
    },
    noticeData: await fetchNotices({ baseUrl, page, limit }),
  };
};

const Meta = ({ bbsName }: { bbsName: string }) => (
  <>
    <title>お知らせ一覧 - {bbsName}</title>
    <meta property="og:title" content={`お知らせ一覧 - ${bbsName}`} />
  </>
);

function NoticeListPage({ loaderData }: Route.ComponentProps) {
  const [searchParams] = useSearchParams();
  const currentPage = parseInt(searchParams.get("page") || "0", 10);
  const { eddistData, noticeData } = loaderData;
  const totalPages = Math.ceil(noticeData.total / noticeData.limit);

  return (
    <div className="min-h-[calc(100vh-1rem)] lg:min-h-[calc(100vh-4rem)] flex flex-col">
      <Meta bbsName={eddistData.bbsName} />
      <article className="flex-1">
        <header>
          <h1 className="text-3xl lg:text-5xl mb-3">お知らせ一覧</h1>
          <Link to="/" className="text-blue-500 text-sm">
            ← トップページに戻る
          </Link>
        </header>
        <section className="py-4 pt-8">
          <ul className="space-y-4">
            {noticeData.notices.map((notice: NoticeListItem) => (
              <li key={notice.slug} className="border-b pb-4">
                <Link
                  to={`/notices/${notice.slug}`}
                  className="text-blue-500 text-xl hover:underline"
                >
                  {notice.title}
                </Link>
                <p className="text-gray-600 text-sm mt-1">
                  {new Date(notice.published_at).toLocaleDateString("ja-JP", {
                    year: "numeric",
                    month: "2-digit",
                    day: "2-digit",
                  })}
                </p>
              </li>
            ))}
          </ul>
        </section>

        {/* Pagination */}
        {totalPages > 1 && (
          <nav className="flex justify-center gap-2 py-4">
            {currentPage > 0 && (
              <Link
                to={`/notices?page=${currentPage - 1}`}
                className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
              >
                前へ
              </Link>
            )}
            <span className="px-4 py-2">
              {currentPage + 1} / {totalPages}
            </span>
            {currentPage < totalPages - 1 && (
              <Link
                to={`/notices?page=${currentPage + 1}`}
                className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
              >
                次へ
              </Link>
            )}
          </nav>
        )}
      </article>
      <footer
        id="footer"
        className="py-2 text-center bg-white border-t border-gray-300"
      >
        <p className="text-xs text-gray-500">
          This BBS is powered by{" "}
          <a
            href="https://github.com/edginer/eddist"
            className="text-blue-500 underline"
          >
            Eddist
          </a>
          .
        </p>
      </footer>
    </div>
  );
}

export default NoticeListPage;
