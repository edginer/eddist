import { useSuspenseQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

interface Board {
  name: string;
  board_key: string;
  default_name: string;
}

function TopPage() {
  const { data: boards } = useSuspenseQuery({
    queryKey: ["boards"],
    queryFn: () => fetch("/api/boards").then((res) => res.json()),
  });

  return (
    <div>
      <header>
        <h1 className="text-3xl lg:text-5xl">エッヂ掲示板</h1>
      </header>
      <section className="py-4 pt-8">
        <h2 className="text-2xl lg:text-4xl">板一覧</h2>
        <ul className="text-left list-disc list-inside pl-4 py-2 lg:text-lg">
          {boards.map((board: Board) => (
            <li key={board.board_key}>
              <Link to={`/${board.board_key}/`}>{board.name}</Link>
            </li>
          ))}
        </ul>
      </section>
      <section>
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
      <section className="py-4 pt-4">
        <h2 className="text-2xl lg:text-4xl">利用規約</h2>
        <p className="text-left py-2 lg:text-lg">
          <a href="/terms" className="text-blue-500">
            利用規約はこちら
          </a>
        </p>
      </section>
    </div>
  );
}

export default TopPage;
