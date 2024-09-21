import { useQuery as useSuspenseQuery } from "@tanstack/react-query";
import { Button, Label, Modal, Textarea, TextInput } from "flowbite-react";
import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { FaArrowLeft } from "react-icons/fa";
import { useForm } from "react-hook-form";
import { twMerge } from "tailwind-merge";
import { postResponse } from "./utils";
import AuthCodeModal from "./AuthCodeModal";

interface Response {
  name: string;
  mail: string;
  date: string;
  authorId: string;
  body: string;
  id: number;
}

const convertThreadTextToResponseList = (text: string) => {
  const lines = text.split("\n").filter((x) => x !== "");
  let threadTitle = "";
  const responses = lines.map((line, idx) => {
    const lineRegex = /^(.*)<>(.*)<>(.*) ID:(.*)<>(.*)<>(.*)$/;
    const match = line.match(lineRegex);
    if (match == null) {
      // あぼーん<>あぼーん<><> あぼーん<> てす
      const aboneRegex = /^(.*)<>(.*)<><> あぼーん<>(.*)$/;
      const aboneMatch = line.match(aboneRegex);
      if (aboneMatch == null) {
        throw new Error(`Invalid response line: ${line}`);
      }

      if (idx === 0) {
        threadTitle = aboneMatch[3];
      }

      return {
        name: aboneMatch[1],
        mail: "",
        date: "",
        authorId: "",
        body: "あぼーん",
        id: idx + 1,
      };
    }
    const name = match[1];
    const mail = match[2];
    const date = match[3];
    const authorId = match[4];
    const body = match[5];
    if (idx === 0) {
      threadTitle = match[6];
    }

    return {
      name,
      mail,
      date,
      authorId,
      body,
      id: idx + 1,
    };
  });

  return {
    threadName: threadTitle,
    responses: responses satisfies Response[],
  };
};

const ThreadPage = () => {
  const params = useParams();
  const { register, handleSubmit } = useForm();

  const { data: boards } = useSuspenseQuery({
    queryKey: ["boards"],
    queryFn: () => fetch("/api/boards").then((res) => res.json()),
  });

  const { data: posts, refetch } = useSuspenseQuery({
    queryKey: ["thread", params.boardKey, params.threadKey],
    queryFn: async () => {
      const res = await fetch(
        `/${params.boardKey}/dat/${params.threadKey}.dat`,
        {
          headers: {
            "Content-Type": "text/plain; charset=shift_jis",
          },
          redirect: "manual",
        }
      );
      const sjisText = await res.blob();
      const arrayBuffer = await sjisText.arrayBuffer();
      const text = new TextDecoder("shift_jis").decode(arrayBuffer);
      return convertThreadTextToResponseList(text);
    },
  });

  const [creatingResponse, setCreatingResponse] = useState(false);
  const [openAuthCodeModal, setOpenAuthCodeModal] = useState(false);
  const [authCode, setAuthCode] = useState("");

  return (
    <div>
      <Modal
        show={creatingResponse}
        size="5xl"
        onClose={() => setCreatingResponse(false)}
      >
        <AuthCodeModal
          openAuthCodeModal={openAuthCodeModal}
          setOpenAuthCodeModal={setOpenAuthCodeModal}
          authCode={authCode}
        />

        <Modal.Header>
          <h3 className="lg:text-2xl">書き込み</h3>
        </Modal.Header>
        <Modal.Body>
          <form
            onSubmit={handleSubmit(async (data) => {
              const result = await postResponse({
                name: data.name,
                mail: data.email,
                body: data.body,
                boardKey: params.boardKey!,
                threadKey: params.threadKey!,
              });

              if (!result.success) {
                setAuthCode(result.authCode);
                setOpenAuthCodeModal(true);
                return false;
              }

              setCreatingResponse(false);
              await refetch();
            })}
          >
            <div className="space-y-6">
              <div className="flex justify-between">
                <div className="flex-grow mr-2">
                  <div className="mb-2 block">
                    <Label htmlFor="modal-name" value="名前" />
                  </div>
                  <TextInput
                    id="modal-name"
                    placeholder="名前..."
                    {...register("name")}
                  />
                </div>
                <div className="flex-grow ml-2">
                  <div className="mb-2 block">
                    <Label htmlFor="modal-email" value="メール" />
                  </div>
                  <TextInput
                    id="modal-email"
                    placeholder="メール..."
                    {...register("mail")}
                  />
                </div>
              </div>
              <div>
                <div className="mb-2 block">
                  <Label value="本文" />
                </div>
                <Textarea
                  placeholder="本文..."
                  required
                  rows={8}
                  {...register("body", { required: true })}
                />
              </div>

              <div className="w-full">
                <Button type="submit">書き込む</Button>
              </div>
            </div>
          </form>
        </Modal.Body>
      </Modal>
      <header className="flex justify-between items-center">
        <Link to={`/${params.boardKey}`}>
          <FaArrowLeft className="mx-2 mr-4 w-6 h-6" />
        </Link>
        <h1 className="text-3xl lg:text-5xl flex-grow">
          {
            boards?.find(
              (board: { board_key: string }) =>
                board.board_key === params.boardKey
            )?.name
          }
        </h1>
        <Button
          onClick={() => setCreatingResponse(true)}
          className={twMerge(
            "px-2 mx-4",
            params.boardKey || params.threadKey || "hidden"
          )}
        >
          書き込み
        </Button>
      </header>
      <div className=" mx-auto bg-white border border-gray-300 rounded-lg shadow-md mt-4">
        <div className="p-4 bg-gray-100 border-b border-gray-300">
          <div className="text-lg">{posts?.threadName}</div>
        </div>
        <div>
          {posts?.responses.map((post) => (
            <div key={post.id} className="border-b border-gray-300 p-4">
              <div className="text-sm text-gray-500">
                {post.id}. {post.name} {post.date} ID: {post.authorId}
              </div>
              <div
                className="text-gray-800 mt-2"
                dangerouslySetInnerHTML={{ __html: post.body }}
              ></div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default ThreadPage;
