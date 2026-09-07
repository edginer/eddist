import { type ReactNode, useCallback } from "react";
import { useToast } from "~/contexts/ToastContext";
import { useDeferredModal } from "~/hooks/useDeferredModal";
import type { NGWordsSettingsModal } from "./NGWordsSettingsModal";

const loadNGWordsSettingsModal = () =>
  import("./NGWordsSettingsModal").then((module) => module.NGWordsSettingsModal);

interface NGSettingsLauncherProps {
  children: ReactNode;
  className?: string;
  enableSafeMode: boolean;
  summarizerSupported?: boolean;
  summarizeEnabled?: boolean;
  onSummarizeEnabledChange?: (enabled: boolean) => void;
  title?: string;
}

export const NGSettingsLauncher = ({
  children,
  className,
  title,
  enableSafeMode,
  summarizerSupported,
  summarizeEnabled,
  onSummarizeEnabledChange,
}: NGSettingsLauncherProps) => {
  const { showToast } = useToast();
  const handleModalLoadError = useCallback(() => {
    showToast("設定ダイアログの読み込みに失敗しました。再度お試しください。", "error");
  }, [showToast]);
  const { Modal, open, openModal, prefetchModal, setOpen } = useDeferredModal<
    typeof NGWordsSettingsModal
  >(loadNGWordsSettingsModal, { onOpenError: handleModalLoadError });

  return (
    <>
      <button
        type="button"
        className={className}
        title={title}
        onPointerEnter={prefetchModal}
        onFocus={prefetchModal}
        onClick={openModal}
      >
        {children}
      </button>
      {Modal ? (
        <Modal
          open={open}
          setOpen={setOpen}
          enableSafeMode={enableSafeMode}
          summarizerSupported={summarizerSupported}
          summarizeEnabled={summarizeEnabled}
          onSummarizeEnabledChange={onSummarizeEnabledChange}
        />
      ) : null}
    </>
  );
};
