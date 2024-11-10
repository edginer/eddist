import { Button, Label, Modal, Textarea, TextInput } from "flowbite-react";
import { useState } from "react";
import ErrorModal from "./ErrorModal";
import AuthCodeModal from "./AuthCodeModal";
import { postThread } from "./utils";
import { useForm } from "react-hook-form";

interface PostThreadModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
  boardKey: string;
  refetchThreadList: () => Promise<unknown>;
}

const PostThreadModal = (props: PostThreadModalProps) => {
  const [openAuthCodeModal, setOpenAuthCodeModal] = useState(false);
  const [authCode, setAuthCode] = useState("");
  const [errorModal, serErrorModal] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  const { register, handleSubmit } = useForm();

  return (
    <Modal show={props.open} size="5xl" onClose={() => props.setOpen(false)}>
      <ErrorModal
        openErrorModal={errorModal}
        setOpenErrorModal={serErrorModal}
        errorMessage={errorMessage}
      />
      <AuthCodeModal
        openAuthCodeModal={openAuthCodeModal}
        setOpenAuthCodeModal={setOpenAuthCodeModal}
        authCode={authCode}
      />
      <Modal.Header>
        <h3 className="lg:text-2xl">スレッド作成</h3>
      </Modal.Header>
      <Modal.Body>
        <form
          onSubmit={handleSubmit(async (data) => {
            const result = await postThread({
              title: data.title,
              name: data.name,
              mail: data.mail,
              body: data.body,
              boardKey: props.boardKey,
            });
            if (!result.success) {
              switch (result.error.kind) {
                case "auth-code":
                  setAuthCode(result.error.authCode);
                  setOpenAuthCodeModal(true);
                  return;
                case "unknown":
                  serErrorModal(true);
                  setErrorMessage(result.error.errorHtml);
                  return;
                default:
                  break;
              }
            }
            props.setOpen(false);
            await props.refetchThreadList();
          })}
        >
          <div className="space-y-6">
            <div>
              <div className="mb-2 block">
                <Label htmlFor="modal-thread-name" value="スレッド名" />
              </div>
              <TextInput
                id="modal-thread-name"
                placeholder="スレッド名..."
                required
                {...register("title", { required: true })}
              />
            </div>
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
  );
};

export default PostThreadModal;
