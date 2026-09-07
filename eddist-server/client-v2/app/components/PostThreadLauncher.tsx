import { type ReactNode, useCallback } from "react";
import { useToast } from "~/contexts/ToastContext";
import { useDeferredModal } from "~/hooks/useDeferredModal";
import type PostThreadModal from "./PostThreadModal";
import { Button } from "./ui/Button";

const loadPostThreadModal = () => import("./PostThreadModal").then((module) => module.default);

interface PostThreadLauncherProps {
  children: ReactNode;
  className?: string;
  boardKey: string;
  refetchThreadList: () => Promise<unknown>;
}

export const PostThreadLauncher = ({
  children,
  className,
  boardKey,
  refetchThreadList,
}: PostThreadLauncherProps) => {
  const { showToast } = useToast();
  const handleModalLoadError = useCallback(() => {
    showToast("スレッド作成ダイアログの読み込みに失敗しました。再度お試しください。", "error");
  }, [showToast]);
  const { Modal, open, openModal, prefetchModal, setOpen } = useDeferredModal<
    typeof PostThreadModal
  >(loadPostThreadModal, { onOpenError: handleModalLoadError });

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
          refetchThreadList={refetchThreadList}
        />
      ) : null}
    </>
  );
};
