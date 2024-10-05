import { Link, useParams } from "@remix-run/react";
import { Button, Modal } from "flowbite-react";
import { useState } from "react";
import { toast } from "react-toastify";
import Breadcrumb from "~/components/Breadcrumb";
import DatArchiveResponseList from "~/components/DatArchiveResponseList";
import { useDeleteAuthedToken } from "~/hooks/deleteAuthedToken";
import {
  deleteDatArchivedResponse,
  getDatAdminArchivedThread,
  getDatArcvhiedThread,
  updateDatArchivedResponse,
} from "~/hooks/queries";

const Page = () => {
  const params = useParams();
  if (params.boardKey == null || params.threadId == null) {
    throw new Error("Page not found");
  }

  const archivedThread = getDatArcvhiedThread({
    params: {
      path: {
        board_key: params.boardKey,
        thread_number: Number(params.threadId),
      },
    },
  });

  const adminArchivedThread = getDatAdminArchivedThread({
    params: {
      path: {
        board_key: params.boardKey,
        thread_number: Number(params.threadId),
      },
    },
  });
  const thread = adminArchivedThread.data;

  const [selectedResponsesOrder, setSelectedResponsesOrder] = useState<
    number[]
  >([]);
  const [onEditResponseOrder, setOnEditResponseOrder] = useState<
    number | undefined
  >();

  const [editRespState, setEditRespState] = useState<
    | {
        author_name: string;
        body: string;
        email: string;
        res_order: number;
      }
    | undefined
  >();

  const deleteAuthedToken = useDeleteAuthedToken();

  const deleteResponse = (resOrder: number) =>
    deleteDatArchivedResponse({
      params: {
        path: {
          board_key: params.boardKey!!,
          thread_number: Number(params.threadId),
          res_order: resOrder,
        },
      },
    });

  const updateResp = (
    author_name: string,
    body: string,
    email: string,
    res_order: number
  ) =>
    updateDatArchivedResponse({
      params: {
        path: {
          board_key: params.boardKey!!,
          thread_number: Number(params.threadId),
        },
      },
      body: [
        {
          author_name,
          email,
          body,
          res_order,
        },
      ],
    });

  return (
    <>
      <Modal
        show={onEditResponseOrder != null}
        onClose={() => setOnEditResponseOrder(undefined)}
      >
        <Modal.Header>Edit Response</Modal.Header>
        <Modal.Body>
          <div className="flex flex-row">
            <div className="flex flex-col">
              <label htmlFor="name">Name</label>
              <input
                type="text"
                id="name"
                className="border border-gray-300 rounded-md px-2 py-1"
                value={editRespState?.author_name ?? ""}
                onChange={(e) => {
                  if (editRespState) {
                    setEditRespState({
                      ...editRespState,
                      author_name: e.target.value,
                    });
                  }
                }}
              />
            </div>
            <div className="flex flex-col">
              <label htmlFor="mail">Mail</label>
              <input
                type="text"
                id="mail"
                className="border border-gray-300 rounded-md px-2 py-1"
                value={editRespState?.email ?? ""}
                onChange={(e) => {
                  if (editRespState) {
                    setEditRespState({
                      ...editRespState,
                      email: e.target.value,
                    });
                  }
                }}
              />
            </div>
          </div>

          <div className="flex flex-col">
            <label htmlFor="body">Body</label>
            <textarea
              id="body"
              className="border border-gray-300 rounded-md px-2 py-1"
              value={editRespState?.body}
              onChange={(e) => {
                if (editRespState) {
                  setEditRespState({
                    ...editRespState,
                    body: e.target.value,
                  });
                }
              }}
            />
          </div>
        </Modal.Body>
        <Modal.Footer>
          <Button
            onClick={() => {
              setOnEditResponseOrder(undefined);
              setEditRespState(undefined);
            }}
          >
            Close
          </Button>
          <Button
            onClick={async () => {
              if (editRespState) {
                const { mutate } = updateResp(
                  editRespState.author_name,
                  editRespState.body,
                  editRespState.email,
                  onEditResponseOrder!!
                );
                await mutate();
                setEditRespState(undefined);
                setOnEditResponseOrder(undefined);
              } else {
                toast.error("Failed to update response: invalid state");
              }
            }}
          >
            Save
          </Button>
        </Modal.Footer>
      </Modal>

      <div className="p-4">
        <h1 className="text-3xl font-bold">
          Thread: {thread!.title} ({params.threadId})
        </h1>
        <Breadcrumb>
          <Link
            to="/dashboard/boards"
            className="text-gray-500 hover:text-gray-700"
          >
            Boards
          </Link>
          <Link
            to={`/dashboard/boards/${params.boardKey}`}
            className="text-gray-500 hover:text-gray-700"
          >
            {params.boardKey}
          </Link>
          <Link
            to={`/dashboard/boards/${params.boardKey}/archives/${params.threadId}`}
            className="text-gray-500 hover:text-gray-700"
          >
            {adminArchivedThread!!.data!!.title}
          </Link>
          <span className="text-gray-500" aria-current="page">
            (dat archive)
          </span>
        </Breadcrumb>
        <DatArchiveResponseList
          responses={archivedThread.data?.responses}
          adminResponses={adminArchivedThread.data?.responses!!}
          selectedResponsesOrder={selectedResponsesOrder}
          setSelectedResponsesOrder={setSelectedResponsesOrder}
          onClickDeleteAuthedToken={(token) => deleteAuthedToken(token, false)}
          onClickDeleteAuthedTokensAssociatedWithIp={(token) =>
            deleteAuthedToken(token, true)
          }
          onClickEditResponse={(idx) => {
            setOnEditResponseOrder(idx);
            setEditRespState({
              author_name: adminArchivedThread.data?.responses[idx].name!!,
              body: adminArchivedThread.data?.responses[idx].body!!,
              email: adminArchivedThread.data?.responses[idx].mail!!,
              res_order: idx,
            });
          }}
          onClieckAbon={(idx) => {
            deleteResponse(idx);
          }}
        />
      </div>
    </>
  );
};

export default Page;
