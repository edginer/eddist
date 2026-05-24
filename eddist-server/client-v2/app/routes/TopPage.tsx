import { useState } from "react";
import { Link } from "react-router";
import { type Board, fetchBoards } from "~/api-client/board";
import { fetchClientConfig } from "~/api-client/client-config";
import { fetchLatestNotices, type NoticeListItem } from "~/api-client/notice";
import { Footer } from "~/components/Footer";
import {
  type FavoriteEntry,
  type PostHistoryEntry,
  type ReadHistoryEntry,
  useThreadHistory,
} from "~/contexts/ThreadHistoryContext";
import type { Route } from "./+types/TopPage";

export const headers = () => ({
  "Cache-Control": "s-maxage=300",
});

export const loader = async ({ context }: Route.LoaderArgs) => {
  const baseUrl = context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL;

  const clientConfig = await fetchClientConfig({ baseUrl }).catch(() => ({
    enable_user_registration: false,
  }));

  return {
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
      availableUserRegistration: clientConfig.enable_user_registration,
    },
    boards: await fetchBoards({ baseUrl }),
    notices: await fetchLatestNotices({ baseUrl }).catch(() => []),
  };
};

const Meta = ({ bbsName }: { bbsName: string }) => (
  <>
    <title>{bbsName}</title>
    <meta property="og:title" content={`${bbsName}`} />
    <meta property="og:site_name" content={bbsName} />
    <meta property="og:type" content="website" />
    <meta name="twitter:title" content={`${bbsName}`} />
  </>
);

/* ── SVG Icons ───────────────────────────────────────────── */
const SearchIcon = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <path d="M10.68 11.74a6 6 0 0 1-7.922-8.982 6 6 0 0 1 8.982 7.922l3.04 3.04-.94.94ZM11.5 7a4.499 4.499 0 1 0-8.997 0A4.499 4.499 0 0 0 11.5 7Z" />
  </svg>
);
const StarIcon = ({ size = 14 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <path d="M8 .25a.75.75 0 0 1 .673.418l1.882 3.815 4.21.612a.75.75 0 0 1 .416 1.279l-3.046 2.97.719 4.192a.751.751 0 0 1-1.088.791L8 12.347l-3.766 1.98a.75.75 0 0 1-1.088-.79l.72-4.194L.818 6.374a.75.75 0 0 1 .416-1.28l4.21-.611L7.327.668A.75.75 0 0 1 8 .25Z" />
  </svg>
);
const ClockIcon = ({ size = 14 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <path d="M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0ZM1.5 8a6.5 6.5 0 1 0 13 0 6.5 6.5 0 0 0-13 0Zm7-3.25v2.992l2.028.812a.75.75 0 0 1-.557 1.392l-2.5-1A.751.751 0 0 1 7 8.25v-3.5a.75.75 0 0 1 1.5 0Z" />
  </svg>
);
const PenIcon = ({ size = 14 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <path d="M11.013 1.427a1.75 1.75 0 0 1 2.474 0l1.086 1.086a1.75 1.75 0 0 1 0 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 0 1-.927-.928l.929-3.25c.081-.286.235-.547.445-.758l8.61-8.61Zm1.414 1.06a.25.25 0 0 0-.354 0L10.811 3.75l1.439 1.44 1.263-1.263a.25.25 0 0 0 0-.354Zm-2.262 2.263L8.726 3.31 4.5 7.537v.463h.75a.75.75 0 0 1 .75.75v.75h.75a.75.75 0 0 1 .75.75v.75h.463Z" />
  </svg>
);
const ChevronRight = ({ size = 12 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <path d="M6.22 3.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L10.94 8 6.22 4.28a.75.75 0 0 1 0-1.06Z" />
  </svg>
);
const UserIcon = ({ size = 16 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <path d="M10.561 8.073a6.005 6.005 0 0 1 3.432 5.142.75.75 0 1 1-1.498.07 4.5 4.5 0 0 0-8.99 0 .75.75 0 0 1-1.498-.07 6.004 6.004 0 0 1 3.431-5.142 3.999 3.999 0 1 1 5.123 0ZM10.5 5a2.5 2.5 0 1 0-5 0 2.5 2.5 0 0 0 5 0Z" />
  </svg>
);

/* ── Section card wrapper ────────────────────────────────── */
function SectionCard({ children }: { children: React.ReactNode }) {
  return (
    <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md overflow-hidden shadow-sm">
      {children}
    </div>
  );
}

function SectionHeader({
  children,
  action,
}: {
  children: React.ReactNode;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between px-3 py-2 border-b border-gray-300 dark:border-gray-700 bg-gray-50 dark:bg-gray-800">
      <span className="text-xs font-semibold text-gray-800 dark:text-gray-200">{children}</span>
      {action && <span className="text-xs">{action}</span>}
    </div>
  );
}

/* ── Tab bar ─────────────────────────────────────────────── */
type MainTab = "boards" | "favorites" | "history";

function TabBar({
  active,
  onChange,
  favCount,
  histCount,
}: {
  active: MainTab;
  onChange: (t: MainTab) => void;
  favCount: number;
  histCount: number;
}) {
  const tabs: Array<{ id: MainTab; label: string; count?: number }> = [
    { id: "boards", label: "板一覧" },
    { id: "favorites", label: "お気に入り", count: favCount || undefined },
    { id: "history", label: "履歴", count: histCount || undefined },
  ];

  return (
    <div className="flex border-b border-gray-300 dark:border-gray-700">
      {tabs.map((t) => (
        <button
          key={t.id}
          type="button"
          onClick={() => onChange(t.id)}
          className={[
            "flex items-center gap-1.5 px-3.5 py-2 text-sm font-medium cursor-pointer border-b-2 -mb-px",
            "focus:outline-none",
            active === t.id
              ? "border-blue-600 dark:border-blue-400 text-blue-600 dark:text-blue-400"
              : "border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200",
          ].join(" ")}
        >
          {t.label}
          {t.count != null && (
            <span
              className={[
                "text-[10px] font-semibold px-1.5 py-0.5 rounded-full leading-none",
                active === t.id
                  ? "bg-blue-100 dark:bg-blue-900/50 text-blue-600 dark:text-blue-400"
                  : "bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400",
              ].join(" ")}
            >
              {t.count}
            </span>
          )}
        </button>
      ))}
    </div>
  );
}

/* ── Board list ──────────────────────────────────────────── */
function BoardList({
  boards,
  display,
  onDisplayChange,
}: {
  boards: Board[];
  display: "list" | "grid";
  onDisplayChange: (v: "list" | "grid") => void;
}) {
  return (
    <SectionCard>
      <SectionHeader
        action={
          <div className="flex gap-1">
            {(["list", "grid"] as const).map((v) => (
              <button
                key={v}
                type="button"
                onClick={() => onDisplayChange(v)}
                className={[
                  "px-1.5 py-0.5 rounded text-[11px] cursor-pointer border",
                  display === v
                    ? "bg-blue-600 text-white border-blue-600"
                    : "bg-white dark:bg-gray-900 text-gray-500 dark:text-gray-400 border-gray-300 dark:border-gray-600",
                ].join(" ")}
              >
                {v === "list" ? "リスト" : "グリッド"}
              </button>
            ))}
          </div>
        }
      >
        板一覧{" "}
        <span className="font-normal text-gray-400 dark:text-gray-500">({boards.length})</span>
      </SectionHeader>

      {display === "list" ? (
        <div>
          {boards.map((b) => (
            <Link
              key={b.board_key}
              to={`/${b.board_key}/`}
              className="flex items-center justify-between px-3 py-2.5 border-b last:border-b-0 border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
            >
              <span className="text-sm font-medium text-blue-600 dark:text-blue-400">{b.name}</span>
              <ChevronRight size={12} />
            </Link>
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-2 p-2">
          {boards.map((b) => (
            <Link
              key={b.board_key}
              to={`/${b.board_key}/`}
              className="block p-3 border border-gray-300 dark:border-gray-700 rounded-md hover:border-blue-500 dark:hover:border-blue-400 hover:shadow-sm transition-all"
            >
              <span className="text-sm font-semibold text-blue-600 dark:text-blue-400 leading-snug">
                {b.name}
              </span>
            </Link>
          ))}
        </div>
      )}
    </SectionCard>
  );
}

/* ── Favorites list ──────────────────────────────────────── */
function FavoritesList({ favorites }: { favorites: FavoriteEntry[] }) {
  return (
    <SectionCard>
      <SectionHeader>
        <span className="inline-flex items-center gap-1">
          <StarIcon size={12} />
          お気に入りスレッド
          <span className="font-normal text-gray-400 dark:text-gray-500">({favorites.length})</span>
        </span>
      </SectionHeader>
      {favorites.length === 0 ? (
        <div className="px-3 py-8 text-center text-xs text-gray-400 dark:text-gray-500">
          お気に入りに追加したスレッドはまだありません
        </div>
      ) : (
        <div>
          {favorites.map((f, i) => (
            <Link
              key={f.key}
              to={`/${f.boardKey}/${f.threadKey}`}
              className={[
                "flex items-start gap-2 px-3 py-2.5 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors",
                i < favorites.length - 1 ? "border-b border-gray-100 dark:border-gray-800" : "",
              ].join(" ")}
            >
              <StarIcon size={14} />
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-gray-900 dark:text-gray-100 leading-snug line-clamp-2">
                  {f.title}
                </div>
                <div className="flex items-center gap-2 mt-1 flex-wrap">
                  <span className="text-[10px] bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 px-1.5 py-0.5 rounded-full font-medium">
                    {f.boardKey}
                  </span>
                </div>
              </div>
            </Link>
          ))}
        </div>
      )}
      <div className="px-3 py-1.5 border-t border-gray-100 dark:border-gray-800 text-[10px] text-gray-400 dark:text-gray-500">
        この端末のみで保存中
      </div>
    </SectionCard>
  );
}

/* ── History list ────────────────────────────────────────── */
function HistoryList({
  history,
  postHistory,
}: {
  history: ReadHistoryEntry[];
  postHistory: PostHistoryEntry[];
}) {
  const postHistoryByThread = new Map<string, PostHistoryEntry>();
  for (const p of postHistory) {
    const threadKey = `${p.boardKey}/${p.threadKey}`;
    if (!postHistoryByThread.has(threadKey)) {
      postHistoryByThread.set(threadKey, p);
    }
  }

  return (
    <>
      <SectionCard>
        <SectionHeader>
          <span className="inline-flex items-center gap-1">
            <ClockIcon size={12} />
            閲覧履歴
            <span className="font-normal text-gray-400 dark:text-gray-500">({history.length})</span>
          </span>
        </SectionHeader>
        {history.length === 0 ? (
          <div className="px-3 py-8 text-center text-xs text-gray-400 dark:text-gray-500">
            閲覧したスレッドはまだありません
          </div>
        ) : (
          <div>
            {history.map((h, i) => {
              const myPost = postHistoryByThread.get(h.key);
              return (
                <Link
                  key={h.key}
                  to={`/${h.boardKey}/${h.threadKey}`}
                  className={[
                    "flex items-start gap-2 px-3 py-2.5 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors",
                    i < history.length - 1 ? "border-b border-gray-100 dark:border-gray-800" : "",
                  ].join(" ")}
                >
                  <ClockIcon size={14} />
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium text-gray-900 dark:text-gray-100 leading-snug truncate">
                      {h.title}
                    </div>
                    <div className="flex items-center gap-2 mt-1 flex-wrap">
                      <span className="text-[10px] bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 px-1.5 py-0.5 rounded-full font-medium">
                        {h.boardKey}
                      </span>
                      {myPost && (
                        <span className="inline-flex items-center gap-0.5 text-[10px] bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400 px-1.5 py-0.5 rounded-full font-medium">
                          <PenIcon size={9} />
                          投稿済み
                        </span>
                      )}
                      <span className="ml-auto text-[10px] text-gray-400 dark:text-gray-500">
                        {new Date(h.visitedAt).toLocaleDateString("ja-JP")}
                      </span>
                    </div>
                    <div className="mt-1.5 h-0.5 bg-gray-100 dark:bg-gray-700 rounded-full overflow-hidden" />
                  </div>
                </Link>
              );
            })}
          </div>
        )}
        <div className="px-3 py-1.5 border-t border-gray-100 dark:border-gray-800 text-[10px] text-gray-400 dark:text-gray-500">
          この端末のみで保存中
        </div>
      </SectionCard>

      {postHistory.length > 0 && (
        <SectionCard>
          <SectionHeader>
            <span className="inline-flex items-center gap-1">
              <PenIcon size={12} />
              投稿履歴
              <span className="font-normal text-gray-400 dark:text-gray-500">
                ({postHistory.length})
              </span>
            </span>
          </SectionHeader>
          <div>
            {postHistory.map((p, i) => (
              <Link
                key={p.key}
                to={`/${p.boardKey}/${p.threadKey}`}
                className={[
                  "block px-3 py-2.5 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors",
                  i < postHistory.length - 1 ? "border-b border-gray-100 dark:border-gray-800" : "",
                ].join(" ")}
              >
                <div className="text-sm font-medium text-gray-900 dark:text-gray-100 leading-snug truncate">
                  {p.threadTitle}
                </div>
                <div className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">
                  {p.boardKey} · {new Date(p.postedAt).toLocaleDateString("ja-JP")}
                </div>
              </Link>
            ))}
          </div>
        </SectionCard>
      )}
    </>
  );
}

/* ── Trending placeholder ────────────────────────────────── */
function TrendingPlaceholder() {
  return (
    <SectionCard>
      <SectionHeader>🔥 今日のトレンド</SectionHeader>
      <div className="px-3 py-6 text-center text-xs text-gray-400 dark:text-gray-500">準備中</div>
    </SectionCard>
  );
}

/* ── Sidebar: guest panel ────────────────────────────────── */
function GuestPanel({ availableUserRegistration }: { availableUserRegistration: boolean }) {
  return (
    <SectionCard>
      <div className="p-3 flex items-center gap-2.5">
        <div className="w-8 h-8 rounded-full bg-gray-100 dark:bg-gray-700 flex items-center justify-center flex-shrink-0">
          <UserIcon size={16} />
        </div>
        <div>
          <div className="text-sm font-semibold text-gray-800 dark:text-gray-200">ゲスト</div>
          <div className="text-[10px] text-gray-400 dark:text-gray-500 leading-snug">
            この端末にデータを保存中
          </div>
        </div>
      </div>
      <div className="px-3 pb-3 flex gap-1.5">
        <a
          href="/auth-code"
          className="flex-1 py-1.5 rounded-md bg-blue-600 text-white text-xs font-semibold text-center hover:bg-blue-700 transition-colors"
        >
          認証ページ
        </a>
        {availableUserRegistration && (
          <a
            href="/user"
            className="flex-1 py-1.5 rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-900 text-gray-700 dark:text-gray-300 text-xs text-center hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
          >
            ユーザーページ
          </a>
        )}
      </div>
    </SectionCard>
  );
}

/* ── Sidebar: compact favorites preview ─────────────────── */
function FavoritesPreview({ favorites }: { favorites: FavoriteEntry[] }) {
  const shown = favorites.slice(0, 3);
  return (
    <SectionCard>
      <SectionHeader
        action={
          <span className="text-blue-600 dark:text-blue-400 cursor-pointer text-[11px]">
            すべて
          </span>
        }
      >
        <span className="inline-flex items-center gap-1">
          <StarIcon size={12} />
          お気に入り
          <span className="font-normal text-gray-400 dark:text-gray-500">({favorites.length})</span>
        </span>
      </SectionHeader>
      {shown.length === 0 ? (
        <div className="px-3 py-4 text-xs text-gray-400 dark:text-gray-500 text-center">なし</div>
      ) : (
        shown.map((f, i) => (
          <Link
            key={f.key}
            to={`/${f.boardKey}/${f.threadKey}`}
            className={[
              "block px-2.5 py-1.5 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors",
              i < shown.length - 1 ? "border-b border-gray-100 dark:border-gray-800" : "",
            ].join(" ")}
          >
            <div className="text-xs text-gray-800 dark:text-gray-200 leading-snug line-clamp-2">
              {f.title}
            </div>
            <div className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">{f.boardKey}</div>
          </Link>
        ))
      )}
    </SectionCard>
  );
}

/* ── Sidebar: compact history preview ───────────────────── */
function HistoryPreview({ history }: { history: ReadHistoryEntry[] }) {
  const shown = history.slice(0, 3);
  return (
    <SectionCard>
      <SectionHeader
        action={
          <span className="text-blue-600 dark:text-blue-400 cursor-pointer text-[11px]">
            すべて
          </span>
        }
      >
        <span className="inline-flex items-center gap-1">
          <ClockIcon size={12} />
          閲覧履歴
        </span>
      </SectionHeader>
      {shown.length === 0 ? (
        <div className="px-3 py-4 text-xs text-gray-400 dark:text-gray-500 text-center">なし</div>
      ) : (
        shown.map((h, i) => (
          <Link
            key={h.key}
            to={`/${h.boardKey}/${h.threadKey}`}
            className={[
              "block px-2.5 py-1.5 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors",
              i < shown.length - 1 ? "border-b border-gray-100 dark:border-gray-800" : "",
            ].join(" ")}
          >
            <div className="text-xs text-gray-800 dark:text-gray-200 leading-snug truncate">
              {h.title}
            </div>
            <div className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">{h.boardKey}</div>
          </Link>
        ))
      )}
    </SectionCard>
  );
}

/* ── Sidebar: login CTA ──────────────────────────────────── */
function LoginCTA() {
  return (
    <div className="bg-gray-50 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md p-3">
      <div className="flex items-center gap-1.5 text-xs font-semibold text-gray-700 dark:text-gray-300 mb-2">
        <span className="w-1.5 h-1.5 rounded-full bg-yellow-600 inline-block" />
        ログインで追加される機能
      </div>
      <div className="text-[11px] text-gray-500 dark:text-gray-400 leading-relaxed space-y-0.5">
        <div>· 端末をまたいだ同期</div>
        <div>· 返信・更新の通知</div>
        <div>· 認証付きの投稿（規制回避）</div>
        <div>· おすすめスレッド</div>
      </div>
      <div className="mt-2 pt-2 border-t border-gray-200 dark:border-gray-700 text-[10px] text-gray-400 dark:text-gray-500 leading-snug">
        お気に入り・履歴は
        <strong className="text-gray-500 dark:text-gray-400"> ログインなしでも保存されます</strong>
        （この端末のみ）
      </div>
    </div>
  );
}

/* ── Right: notices ──────────────────────────────────────── */
function NoticesPanel({ notices }: { notices: NoticeListItem[] }) {
  if (notices.length === 0) return null;
  return (
    <SectionCard>
      <SectionHeader>📢 お知らせ</SectionHeader>
      {notices.map((n, i) => (
        <div
          key={n.slug}
          className={[
            "px-3 py-2 flex gap-2 items-start text-xs",
            i < notices.length - 1 ? "border-b border-gray-100 dark:border-gray-800" : "",
          ].join(" ")}
        >
          <span className="text-gray-400 dark:text-gray-500 whitespace-nowrap pt-0.5">
            {new Date(n.published_at).toLocaleDateString("ja-JP", {
              year: "numeric",
              month: "2-digit",
              day: "2-digit",
            })}
          </span>
          <Link
            to={`/notices/${n.slug}`}
            className="text-blue-600 dark:text-blue-400 leading-snug hover:underline"
          >
            {n.title}
          </Link>
        </div>
      ))}
      <div className="px-3 py-1.5">
        <Link
          to="/notices"
          className="text-[11px] text-blue-600 dark:text-blue-400 hover:underline"
        >
          もっと見る →
        </Link>
      </div>
    </SectionCard>
  );
}

/* ── Right: quick links ──────────────────────────────────── */
function QuickLinks({ availableUserRegistration }: { availableUserRegistration: boolean }) {
  const links = [
    ["🔐 認証ページ", "/auth-code"],
    ...(availableUserRegistration ? [["👤 ユーザーページ", "/user"]] : []),
    ["📄 利用規約", "/terms"],
  ] as const;

  return (
    <SectionCard>
      <SectionHeader>リンク</SectionHeader>
      {links.map(([label, href], i) => (
        <a
          key={href}
          href={href}
          className={[
            "flex items-center justify-between px-3 py-2 text-xs text-blue-600 dark:text-blue-400 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors",
            i < links.length - 1 ? "border-b border-gray-100 dark:border-gray-800" : "",
          ].join(" ")}
        >
          {label} <ChevronRight size={11} />
        </a>
      ))}
    </SectionCard>
  );
}

/* ── Right: stats placeholder ────────────────────────────── */
function StatsPlaceholder() {
  return (
    <SectionCard>
      <SectionHeader>📊 本日の統計</SectionHeader>
      <div className="px-3 py-4 text-center text-xs text-gray-400 dark:text-gray-500">準備中</div>
    </SectionCard>
  );
}

/* ── Mobile pill nav ─────────────────────────────────────── */
type MobSection = "boards" | "favorites" | "history" | "trending";

function MobilePillNav({
  active,
  onChange,
  favCount,
  histCount,
}: {
  active: MobSection;
  onChange: (s: MobSection) => void;
  favCount: number;
  histCount: number;
}) {
  const pills: Array<{ id: MobSection; label: string }> = [
    { id: "boards", label: "板一覧" },
    {
      id: "favorites",
      label: favCount > 0 ? `★ お気に入り (${favCount})` : "★ お気に入り",
    },
    { id: "history", label: histCount > 0 ? `履歴 (${histCount})` : "履歴" },
    { id: "trending", label: "トレンド" },
  ];

  return (
    <div className="flex gap-1.5 px-3 py-2 bg-white dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 overflow-x-auto">
      {pills.map((p) => (
        <button
          key={p.id}
          type="button"
          onClick={() => onChange(p.id)}
          className={[
            "px-3 py-1 rounded-full border text-xs font-medium whitespace-nowrap cursor-pointer transition-colors",
            active === p.id
              ? "bg-blue-600 border-blue-600 text-white"
              : "bg-white dark:bg-gray-900 border-gray-300 dark:border-gray-600 text-gray-500 dark:text-gray-400",
          ].join(" ")}
        >
          {p.label}
        </button>
      ))}
    </div>
  );
}

/* ══════════════════════════════════════════════════════════
   TOP PAGE
══════════════════════════════════════════════════════════ */
function TopPage({ loaderData: { eddistData, boards, notices } }: Route.ComponentProps) {
  const { history, favorites, postHistory } = useThreadHistory();
  const [activeTab, setActiveTab] = useState<MainTab>("boards");
  const [activeMobSection, setActiveMobSection] = useState<MobSection>("boards");
  const [boardDisplay, setBoardDisplay] = useState<"list" | "grid">("list");

  return (
    <div className="min-h-screen bg-white dark:bg-gray-950">
      <Meta bbsName={eddistData.bbsName} />

      {/* ── Header ── */}
      <header className="sticky top-0 z-50 flex items-center gap-3 h-[52px] px-4 bg-[#24292f] border-b border-[#444c56]">
        <div className="text-white font-bold text-[17px] tracking-tight flex-shrink-0">
          {eddistData.bbsName}
        </div>

        {/* Search (placeholder) */}
        <div className="hidden sm:flex flex-1 max-w-[260px] items-center gap-2 bg-white/10 border border-white/15 rounded-md px-2.5 py-1.5 text-[#9198a1] text-xs cursor-text">
          <SearchIcon />
          <span>スレッドを検索...</span>
          <span className="ml-auto text-[11px] bg-white/10 border border-white/15 rounded px-1 text-[#8b949e]">
            /
          </span>
        </div>

        {/* Nav */}
        <div className="flex items-center gap-2 ml-auto">
          <a
            href="/auth-code"
            className="hidden sm:block px-3 py-1 rounded-md border border-white/25 text-[#e6edf3] text-xs hover:bg-white/10 transition-colors"
          >
            認証
          </a>
          {eddistData.availableUserRegistration && (
            <a
              href="/user"
              className="px-3 py-1 rounded-md bg-blue-600 hover:bg-blue-700 text-white text-xs font-semibold transition-colors"
            >
              ユーザーページ
            </a>
          )}
        </div>
      </header>

      {/* ── Desktop layout (md+) ── */}
      <div className="hidden md:grid max-w-[1200px] mx-auto px-4 py-4 gap-4 [grid-template-columns:220px_1fr_220px]">
        {/* Left sidebar */}
        <div className="flex flex-col gap-3">
          <GuestPanel availableUserRegistration={eddistData.availableUserRegistration} />
          <FavoritesPreview favorites={favorites} />
          <HistoryPreview history={history} />
          <LoginCTA />
        </div>

        {/* Main content */}
        <div className="flex flex-col gap-3">
          <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md overflow-hidden shadow-sm">
            <TabBar
              active={activeTab}
              onChange={setActiveTab}
              favCount={favorites.length}
              histCount={history.length}
            />

            <div className="p-3">
              {activeTab === "boards" && (
                <BoardList
                  boards={boards}
                  display={boardDisplay}
                  onDisplayChange={setBoardDisplay}
                />
              )}
              {activeTab === "favorites" && <FavoritesList favorites={favorites} />}
              {activeTab === "history" && (
                <HistoryList history={history} postHistory={postHistory} />
              )}
            </div>
          </div>

          <TrendingPlaceholder />
        </div>

        {/* Right sidebar */}
        <div className="flex flex-col gap-3">
          <StatsPlaceholder />
          <QuickLinks availableUserRegistration={eddistData.availableUserRegistration} />
          <NoticesPanel notices={notices} />
        </div>
      </div>

      {/* ── Mobile layout (<md) ── */}
      <div className="md:hidden">
        {/* Search */}
        <div className="bg-gray-50 dark:bg-gray-800 px-3 py-2 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-full px-3.5 py-2 text-gray-400 dark:text-gray-500 text-sm">
            <SearchIcon />
            <span>スレッドを検索...</span>
          </div>
        </div>

        <MobilePillNav
          active={activeMobSection}
          onChange={setActiveMobSection}
          favCount={favorites.length}
          histCount={history.length}
        />

        <div className="px-3 py-3 flex flex-col gap-3">
          {activeMobSection === "boards" && (
            <BoardList boards={boards} display={boardDisplay} onDisplayChange={setBoardDisplay} />
          )}
          {activeMobSection === "favorites" && <FavoritesList favorites={favorites} />}
          {activeMobSection === "history" && (
            <HistoryList history={history} postHistory={postHistory} />
          )}
          {activeMobSection === "trending" && <TrendingPlaceholder />}
        </div>
      </div>

      <Footer />
    </div>
  );
}

export default TopPage;
