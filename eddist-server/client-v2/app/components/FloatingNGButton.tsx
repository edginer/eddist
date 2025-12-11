import { useEffect, useState } from "react";
import { Button } from "flowbite-react";
import { FaBan } from "react-icons/fa";
import { useNGWords } from "~/contexts/NGWordsContext";
import { useToast } from "~/contexts/ToastContext";

interface SelectionInfo {
  text: string;
  rect: DOMRect;
  target: HTMLElement;
}

export const FloatingNGButton = () => {
  const [selection, setSelection] = useState<SelectionInfo | null>(null);
  const { addRule } = useNGWords();
  const { showToast } = useToast();

  useEffect(() => {
    const handleSelectionChange = () => {
      const sel = window.getSelection();
      if (!sel || sel.rangeCount === 0) {
        setSelection(null);
        return;
      }

      const selectedText = sel.toString().trim();
      if (!selectedText) {
        setSelection(null);
        return;
      }

      // Get the range and its bounding rectangle
      const range = sel.getRangeAt(0);
      const rect = range.getBoundingClientRect();

      // Find the target element (the element that should be filtered)
      const targetElement = range.commonAncestorContainer.parentElement;
      if (!targetElement) {
        setSelection(null);
        return;
      }

      // Check if the selection is within a body text element (data-ng-target="body")
      const filterableElement = targetElement.closest(
        '[data-ng-target="body"]'
      );
      if (!filterableElement) {
        setSelection(null);
        return;
      }

      setSelection({
        text: selectedText,
        rect,
        target: filterableElement as HTMLElement,
      });
    };

    // Listen for selection changes
    document.addEventListener("selectionchange", handleSelectionChange);

    // Also listen for touchend to update position after selection is made
    document.addEventListener("touchend", () => {
      setTimeout(handleSelectionChange, 100);
    });

    return () => {
      document.removeEventListener("selectionchange", handleSelectionChange);
      document.removeEventListener("touchend", handleSelectionChange);
    };
  }, []);

  const handleClick = () => {
    if (selection) {
      try {
        // Directly add to NG words with default settings
        addRule("response.bodies", {
          pattern: selection.text,
          matchType: "partial",
          enabled: true,
          hideMode: "collapsed",
        });

        showToast(`NGワードに追加: ${selection.text}`, "success");
      } catch (error) {
        console.error("[FloatingNGButton] Error adding rule:", error);
        showToast("NGワードの追加に失敗しました", "error");
      }

      setSelection(null);
    }
  };

  if (!selection) {
    return null;
  }

  // Position the button above the selection
  const buttonStyle = {
    position: "fixed" as const,
    left: `${selection.rect.left + selection.rect.width / 2}px`,
    top: `${selection.rect.top - 50}px`, // 50px above selection
    transform: "translateX(-50%)",
    zIndex: 10000,
  };

  // If button would be off-screen at top, show it below the selection
  if (selection.rect.top < 60) {
    buttonStyle.top = `${selection.rect.bottom + 10}px`;
  }

  return (
    <div style={buttonStyle}>
      <Button
        size="sm"
        color="blue"
        onClick={handleClick}
        className="shadow-lg"
      >
        <FaBan className="mr-1" />
        NGに追加
      </Button>
    </div>
  );
};
