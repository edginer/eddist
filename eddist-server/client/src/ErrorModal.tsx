import { Button, Modal } from "flowbite-react";
import { FaExclamation } from "react-icons/fa";

interface ErrorModalProps {
  openErrorModal: boolean;
  setOpenErrorModal: React.Dispatch<React.SetStateAction<boolean>>;
  errorMessage: string;
}

const ErrorModal = (props: ErrorModalProps) => {
  return (
    <Modal
      show={props.openErrorModal}
      size="md"
      onClose={() => props.setOpenErrorModal(false)}
      popup
    >
      <Modal.Header />
      <Modal.Body>
        <div className="text-center">
          <FaExclamation className="mx-auto mb-4 h-14 w-14 text-gray-400 dark:text-gray-200" />
          <span
            dangerouslySetInnerHTML={{ __html: props.errorMessage }}
            className="mb-5 p-2 text-lg font-normal text-gray-500 dark:text-gray-400"
          ></span>
          <div className="flex justify-center gap-4">
            <Button
              color="failure"
              onClick={() => {
                props.setOpenErrorModal(false);
              }}
            >
              閉じる
            </Button>
          </div>
        </div>
      </Modal.Body>
    </Modal>
  );
};

export default ErrorModal;
