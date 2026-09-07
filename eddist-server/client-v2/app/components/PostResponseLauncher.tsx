import { type ReactNode, useCallback } from "react";
import { useToast } from "~/contexts/ToastContext";
import { useDeferredModal } from "~/hooks/useDeferredModal";
import type PostResponseModal from "./PostResponseModal";
import { Button } from "./ui/Button";

const loadPostResponseModal = () => import("./PostResponseModal").then((module) => module.default);

interface PostResponseLauncherProps {
  children: ReactNode;
  className?: string;
  boardKey: string;
  threadKey: string;
  refetchThread: () => Promise<unknown>;
}

export const PostResponseLauncher = ({
  children,
  className,
  boardKey,
  threadKey,
  refetchThread,
}: PostResponseLauncherProps) => {
  const { showToast } = useToast();
  const handleModalLoadError = useCallback(() => {
    showToast("書き込みダイアログの読み込みに失敗しました。再度お試しください。", "error");
  }, [showToast]);
  const { Modal, open, openModal, prefetchModal, setOpen } = useDeferredModal<
    typeof PostResponseModal
  >(loadPostResponseModal, { onOpenError: handleModalLoadError });

  return (
    <>
      <Button
        onPointerEnter={prefetchModal}
        onFocus={prefetchModal}
        onClick={openModal}
        className={className}
      >
        {children}
      </Button>
      {Modal ? (
        <Modal
          open={open}
          setOpen={setOpen}
          boardKey={boardKey}
          threadKey={threadKey}
          refetchThread={refetchThread}
        />
      ) : null}
    </>
  );
};
