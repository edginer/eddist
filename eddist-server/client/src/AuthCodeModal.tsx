import { Button, Modal } from "flowbite-react";
import React from "react";
import { FaExclamation } from "react-icons/fa";

interface AuthCodeModalProps {
  openAuthCodeModal: boolean;
  setOpenAuthCodeModal: React.Dispatch<React.SetStateAction<boolean>>;
  authCode: string;
}

const AuthCodeModal = (props: AuthCodeModalProps) => {
  return (
    <Modal
      show={props.openAuthCodeModal}
      size="md"
      onClose={() => props.setOpenAuthCodeModal(false)}
      popup
    >
      <Modal.Header />
      <Modal.Body>
        <div className="text-center">
          <FaExclamation className="mx-auto mb-4 h-14 w-14 text-gray-400 dark:text-gray-200" />
          <span className="mb-5 p-2 text-lg font-normal text-gray-500 dark:text-gray-400">
            <p>
              書き込みを行うには、認証コード'{props.authCode}
              'を用いて認証を行う必要があります。
            </p>
            <p>認証ページに移動しますか？</p>
          </span>
          <div className="flex justify-center gap-4">
            <Button
              color="failure"
              onClick={() => {
                props.setOpenAuthCodeModal(false);
                window.open("/auth-code", "_blank");
              }}
            >
              はい
            </Button>
            <Button
              color="gray"
              onClick={() => props.setOpenAuthCodeModal(false)}
            >
              キャンセル
            </Button>
          </div>
        </div>
      </Modal.Body>
    </Modal>
  );
};

export default AuthCodeModal;
