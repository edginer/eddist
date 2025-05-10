import ErrorModal from "./ErrorModal";
import AuthCodeModal from "./AuthCodeModal";
import {
  Button,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  Textarea,
  TextInput,
} from "flowbite-react";
import { postResponse } from "./utils";
import { useState } from "react";
import { useForm } from "react-hook-form";

interface PostResponseModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
  boardKey: string;
  threadKey: string;
  refetchThread: () => Promise<unknown>;
}

const PostResponseModal = (props: PostResponseModalProps) => {
  const { register, handleSubmit } = useForm();

  const [openAuthCodeModal, setOpenAuthCodeModal] = useState(false);
  const [authCode, setAuthCode] = useState("");
  const [errorModal, serErrorModal] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  return (
    <Modal show={props.open} size="5xl" onClose={() => props.setOpen(false)}>
      <AuthCodeModal
        openAuthCodeModal={openAuthCodeModal}
        setOpenAuthCodeModal={setOpenAuthCodeModal}
        authCode={authCode}
      />
      <ErrorModal
        openErrorModal={errorModal}
        setOpenErrorModal={serErrorModal}
        errorMessage={errorMessage}
      />
      <ModalHeader className="border-gray-200 dark:border-gray-600">
        <h3 className="lg:text-2xl">書き込み</h3>
      </ModalHeader>
      <ModalBody>
        <form
          onSubmit={handleSubmit(async (data) => {
            const result = await postResponse({
              name: data.name,
              mail: data.mail,
              body: data.body,
              boardKey: props.boardKey,
              threadKey: props.threadKey,
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
              return false;
            }

            props.setOpen(false);
            await props.refetchThread();
          })}
        >
          <div className="space-y-6">
            <div className="flex justify-between">
              <div className="flex-grow mr-2">
                <div className="mb-2 block">
                  <Label htmlFor="modal-name">名前</Label>
                </div>
                <TextInput
                  id="modal-name"
                  placeholder="名前..."
                  {...register("name")}
                />
              </div>
              <div className="flex-grow ml-2">
                <div className="mb-2 block">
                  <Label htmlFor="modal-email">メール</Label>
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
                <Label>本文</Label>
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
      </ModalBody>
    </Modal>
  );
};

export default PostResponseModal;
