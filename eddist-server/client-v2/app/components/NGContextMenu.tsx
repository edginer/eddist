import { useEffect, useRef } from "react";
import { useNGWords, type NGCategory } from "~/contexts/NGWordsContext";
import { useToast } from "~/contexts/ToastContext";

interface NGContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  options: {
    label: string;
    value: string;
    category: NGCategory;
    isResponse?: boolean;
  }[];
}

export const NGContextMenu = ({
  x,
  y,
  onClose,
  options,
}: NGContextMenuProps) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const { addRule } = useNGWords();
  const { showToast } = useToast();

  // Truncate long text for display
  const truncateText = (text: string, maxLength: number = 100): string => {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "...";
  };

  // Build flat menu items
  const menuItems = options.flatMap((option) => {
    if (option.isResponse) {
      // For responses, show two options: collapsed and hidden
      return [
        {
          label: `${option.label}: ${truncateText(option.value, 30)}`,
          description: "NGに追加 (折りたたむ)",
          onClick: () =>
            handleAddToNG(option.value, option.category, "collapsed"),
        },
        {
          label: `${option.label}: ${truncateText(option.value, 30)}`,
          description: "NGに追加 (非表示)",
          onClick: () => handleAddToNG(option.value, option.category, "hidden"),
        },
      ];
    } else {
      // For non-responses, show one option
      return [
        {
          label: `${option.label}: ${truncateText(option.value, 30)}`,
          description: "NGに追加",
          onClick: () =>
            handleAddToNG(option.value, option.category, undefined),
        },
      ];
    }
  });

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent | TouchEvent) => {
      const target = e.target as Node;
      const clickedInsideMenu =
        menuRef.current && menuRef.current.contains(target);

      if (!clickedInsideMenu) {
        onClose();
      }
    };

    // Add delay to prevent immediate closing
    setTimeout(() => {
      document.addEventListener("mousedown", handleClickOutside);
      document.addEventListener("touchstart", handleClickOutside);
    }, 100);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("touchstart", handleClickOutside);
    };
  }, [onClose]);

  // Close on escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [onClose]);

  // Adjust position to keep menu in viewport
  const adjustedPosition = (() => {
    const menuWidth = 280;
    const menuHeight = menuItems.length * 56 + 16;

    let adjustedX = x;
    let adjustedY = y;

    if (x + menuWidth > window.innerWidth) {
      adjustedX = window.innerWidth - menuWidth - 10;
    }

    if (y + menuHeight > window.innerHeight) {
      adjustedY = window.innerHeight - menuHeight - 10;
    }

    return { x: Math.max(10, adjustedX), y: Math.max(10, adjustedY) };
  })();

  const handleAddToNG = (
    value: string,
    category: NGCategory,
    hideMode?: "hidden" | "collapsed"
  ) => {
    try {
      addRule(category, {
        pattern: value,
        matchType: "partial",
        enabled: true,
        ...(hideMode ? { hideMode } : {}),
      });

      // Visual feedback
      showToast(`NGワードに追加: ${value}`, "success");
    } catch (error) {
      console.error("[NGContextMenu] Error adding rule:", error);
      showToast("NGワードの追加に失敗しました", "error");
    }

    onClose();
  };

  return (
    <div
      ref={menuRef}
      className="fixed bg-white rounded shadow-lg border border-gray-200 py-1 z-9999 min-w-60 max-w-[280px]"
      style={{
        left: `${adjustedPosition.x}px`,
        top: `${adjustedPosition.y}px`,
      }}
    >
      {menuItems.map((item, idx) => (
        <button
          key={idx}
          type="button"
          onClick={item.onClick}
          className="w-full text-left px-4 py-3 hover:bg-gray-100 flex items-center justify-between border-b border-gray-100 last:border-b-0"
        >
          <div className="flex-1 min-w-0">
            <div className="text-sm text-gray-900 wrap-break-word">
              {item.label}
            </div>
            <div className="text-xs text-gray-500 mt-1">{item.description}</div>
          </div>
        </button>
      ))}
    </div>
  );
};
