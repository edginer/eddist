import { useSuspenseQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

interface Board {
  name: string;
  board_key: string;
  default_name: string;
}

declare global {
  interface Window {
    eddistData: {
      bbsName: string;
      availableUserRegistration: boolean;
    };
  }
}

function TopPage() {
  const { data: boards } = useSuspenseQuery({
    queryKey: ["boards"],
    queryFn: () => fetch("/api/boards").then((res) => res.json()),
  });

  return (
    <div className="min-h-[calc(100vh-1rem)] lg:min-h-[calc(100vh-4rem)] flex flex-col">
      <article className="flex-1">
        <header>
          <h1 className="text-3xl lg:text-5xl">
            {window.eddistData?.bbsName ?? "エッヂ掲示板"}
          </h1>
        </header>
        <section className="py-4 pt-8">
          <h2 className="text-2xl lg:text-4xl">板一覧</h2>
          <ul className="text-left list-disc list-inside pl-4 py-2 lg:text-lg">
            {boards.map((board: Board) => (
              <li key={board.board_key}>
                <Link to={`/${board.board_key}/`} className="text-blue-500">
                  {board.name}
                </Link>
              </li>
            ))}
          </ul>
        </section>
        <section className="py-4 pt-8">
          <h2 className="text-2xl lg:text-4xl">認証ページ</h2>
          <p className="text-left py-2 lg:text-lg">
            認証ページへのリンクはこちら
          </p>
          <a
            href="/auth-code"
            className="text-blue-500 text-left py-2 lg:text-lg"
          >
            認証ページ
          </a>
        </section>
        {window.eddistData?.availableUserRegistration && (
          <section>
            <h2 className="text-2xl lg:text-4xl">ユーザーページ・ログイン</h2>
            <a
              href="/user"
              className="text-blue-500 text-left py-2 pt-4 lg:text-lg"
            >
              ユーザーページ
            </a>
            <p className="text-left py-2 lg:text-lg">
              ユーザー登録を行うには書き込みを行う必要があります
            </p>
          </section>
        )}
        <section className="py-4 pt-4">
          <h2 className="text-2xl lg:text-4xl">利用規約</h2>
          <p className="text-left py-2 lg:text-lg">
            <a href="/terms" className="text-blue-500">
              利用規約・問い合わせ先はこちら
            </a>
          </p>
        </section>
      </article>
      <footer
        id="footer"
        className="py-2 text-center bg-white border-t border-gray-300"
      >
        <p className="text-xs text-gray-500">
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
        </p>
      </footer>
    </div>
  );
}

export default TopPage;
