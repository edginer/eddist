import { useState, useCallback, useRef } from "react";

interface ContextMenuState {
  x: number;
  y: number;
  isOpen: boolean;
}

export const useContextMenu = () => {
  const [menuState, setMenuState] = useState<ContextMenuState>({
    x: 0,
    y: 0,
    isOpen: false,
  });

  const longPressTimerRef = useRef<NodeJS.Timeout | null>(null);
  const touchStartPosRef = useRef<{ x: number; y: number } | null>(null);

  const openMenu = useCallback((x: number, y: number) => {
    setMenuState({ x, y, isOpen: true });
  }, []);

  const closeMenu = useCallback(() => {
    setMenuState((prev) => ({ ...prev, isOpen: false }));
  }, []);

  // Desktop: right-click handler
  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      openMenu(e.clientX, e.clientY);
    },
    [openMenu]
  );

  // Mobile: long-press handlers
  const handleTouchStart = useCallback(
    (e: React.TouchEvent) => {
      const touch = e.touches[0];
      touchStartPosRef.current = { x: touch.clientX, y: touch.clientY };

      // Clear any existing timer
      if (longPressTimerRef.current) {
        clearTimeout(longPressTimerRef.current);
      }

      // Start long-press timer (500ms)
      longPressTimerRef.current = setTimeout(() => {
        if (touchStartPosRef.current) {
          // Trigger haptic feedback on mobile if available
          if (navigator.vibrate) {
            navigator.vibrate(50);
          }

          openMenu(touchStartPosRef.current.x, touchStartPosRef.current.y);
          e.preventDefault();
        }
      }, 500);
    },
    [openMenu]
  );

  const handleTouchMove = useCallback((e: React.TouchEvent) => {
    // Cancel long-press if finger moves too much
    if (touchStartPosRef.current && longPressTimerRef.current) {
      const touch = e.touches[0];
      const dx = touch.clientX - touchStartPosRef.current.x;
      const dy = touch.clientY - touchStartPosRef.current.y;
      const distance = Math.sqrt(dx * dx + dy * dy);

      // Cancel if moved more than 10px
      if (distance > 10) {
        clearTimeout(longPressTimerRef.current);
        longPressTimerRef.current = null;
        touchStartPosRef.current = null;
      }
    }
  }, []);

  const handleTouchEnd = useCallback(() => {
    // Clear long-press timer if touch ends before timeout
    if (longPressTimerRef.current) {
      clearTimeout(longPressTimerRef.current);
      longPressTimerRef.current = null;
    }
    touchStartPosRef.current = null;
  }, []);

  const handleTouchCancel = useCallback(() => {
    // Clear long-press timer if touch is cancelled
    if (longPressTimerRef.current) {
      clearTimeout(longPressTimerRef.current);
      longPressTimerRef.current = null;
    }
    touchStartPosRef.current = null;
  }, []);

  return {
    menuState,
    closeMenu,
    contextMenuHandlers: {
      onContextMenu: handleContextMenu,
      onTouchStart: handleTouchStart,
      onTouchMove: handleTouchMove,
      onTouchEnd: handleTouchEnd,
      onTouchCancel: handleTouchCancel,
    },
  };
};
