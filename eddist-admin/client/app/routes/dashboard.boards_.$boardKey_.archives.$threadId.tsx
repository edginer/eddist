import { Link, useParams } from "react-router";
import { getArchivedResponses, getArchivedThread, useDeleteAuthedToken } from "~/hooks/queries";
import Breadcrumb from "../components/Breadcrumb";
import ResponseList from "../components/ResponseList";

const Page = () => {
  const params = useParams();
  if (params.boardKey == null || params.threadId == null) {
    throw new Error("Page not found");
  }

  const { data: responses } = getArchivedResponses({
    params: {
      path: {
        board_key: params.boardKey,
        thread_id: Number(params.threadId),
      },
    },
  });
  const { data: thread } = getArchivedThread({
    params: {
      path: {
        board_key: params.boardKey,
        thread_id: Number(params.threadId),
      },
    },
  });

  const deleteAuthedTokenMutation = useDeleteAuthedToken();

  return (
    <div className="mx-auto w-full max-w-7xl p-4 sm:p-6 lg:p-8">
      <h1 className="break-words text-2xl font-bold text-gray-900 sm:text-3xl">
        Thread: {thread?.title} ({thread?.thread_number})
      </h1>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <Breadcrumb>
          <Link to="/dashboard/boards" className="text-gray-500 hover:text-gray-700">
            Boards
          </Link>
          <Link
            to={`/dashboard/boards/${params.boardKey}`}
            className="text-gray-500 hover:text-gray-700"
          >
            {params.boardKey}
          </Link>
          <span className="text-gray-500" aria-current="page">
            {thread?.title}
          </span>
        </Breadcrumb>
        <Link className="my-2 underline underline-offset-1 sm:mr-4" to={"./dat"}>
          Go to archive dat page
        </Link>
      </div>

      <ResponseList
        onClickDeleteAuthedToken={(token) => {
          deleteAuthedTokenMutation.mutate({ authedTokenId: token, usingOriginIp: false });
        }}
        onClickDeleteAuthedTokensAssociatedWithIp={(token) => {
          deleteAuthedTokenMutation.mutate({ authedTokenId: token, usingOriginIp: true });
        }}
        responses={responses?.filter((r) => r != null) ?? []}
      />
    </div>
  );
};

export default Page;
