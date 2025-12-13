import { useEffect, useRef, useState, useCallback } from "react";

interface UsePullToRefreshOptions {
  onRefresh: () => Promise<void>;
  threshold?: number;
  direction?: "down" | "up";
  enabled?: boolean;
  scrollTarget?: "window" | "element";
}

interface PullToRefreshState {
  isPulling: boolean;
  pullDistance: number;
  isRefreshing: boolean;
}

// Helper to get scroll position based on target
const getScrollPosition = (
  target: "window" | "element",
  element?: HTMLElement | null
) => {
  if (target === "window") {
    return {
      scrollTop: window.scrollY || document.documentElement.scrollTop,
      scrollHeight: document.documentElement.scrollHeight,
      clientHeight: window.innerHeight,
    };
  } else {
    if (!element)
      return { scrollTop: 0, scrollHeight: 0, clientHeight: 0 };
    return {
      scrollTop: element.scrollTop,
      scrollHeight: element.scrollHeight,
      clientHeight: element.clientHeight,
    };
  }
};

export const usePullToRefresh = ({
  onRefresh,
  threshold = 80,
  direction = "down",
  enabled = true,
  scrollTarget = "window",
}: UsePullToRefreshOptions) => {
  const [state, setState] = useState<PullToRefreshState>({
    isPulling: false,
    pullDistance: 0,
    isRefreshing: false,
  });

  const touchStartY = useRef<number>(0);
  const scrollableRef = useRef<HTMLDivElement>(null);

  const handleTouchStart = useCallback(
    (e: Event) => {
      if (!enabled || state.isRefreshing) return;
      if (!(e instanceof TouchEvent)) return;

      const { scrollTop, scrollHeight, clientHeight } = getScrollPosition(
        scrollTarget,
        scrollableRef.current
      );

      // Check if at the correct edge for pull-to-refresh
      const atTop = scrollTop <= 1 && direction === "down"; // Allow 1px tolerance
      const atBottom =
        scrollTop + clientHeight >= scrollHeight - 5 && direction === "up";

      if (atTop || atBottom) {
        touchStartY.current = e.touches[0].clientY;
      }
    },
    [enabled, direction, state.isRefreshing, scrollTarget]
  );

  const handleTouchMove = useCallback(
    (e: Event) => {
      if (!enabled || state.isRefreshing || touchStartY.current === 0) return;
      if (!(e instanceof TouchEvent)) return;

      const { scrollTop, scrollHeight, clientHeight } = getScrollPosition(
        scrollTarget,
        scrollableRef.current
      );

      const currentY = e.touches[0].clientY;
      const diff = currentY - touchStartY.current;

      // Only activate pull-to-refresh if:
      // 1. Still at the edge
      // 2. Pulling in the correct direction
      const atTop = scrollTop <= 1;
      const atBottom = scrollTop + clientHeight >= scrollHeight - 5;

      if (direction === "down") {
        // Pull down from top: only if at top AND pulling down
        if (atTop && diff > 0) {
          e.preventDefault();
          setState((prev) => ({
            ...prev,
            isPulling: true,
            pullDistance: diff,
          }));
        } else {
          // Allow normal scrolling
          setState((prev) => ({
            ...prev,
            isPulling: false,
            pullDistance: 0,
          }));
        }
      } else {
        // Pull up from bottom: only if at bottom AND pulling up
        if (atBottom && diff < 0) {
          e.preventDefault();
          setState((prev) => ({
            ...prev,
            isPulling: true,
            pullDistance: -diff,
          }));
        } else {
          // Allow normal scrolling
          setState((prev) => ({
            ...prev,
            isPulling: false,
            pullDistance: 0,
          }));
        }
      }
    },
    [enabled, direction, state.isRefreshing, scrollTarget]
  );

  const handleTouchEnd = useCallback(async () => {
    if (!enabled || state.isRefreshing) {
      touchStartY.current = 0;
      setState((prev) => ({ ...prev, isPulling: false, pullDistance: 0 }));
      return;
    }

    if (state.pullDistance >= threshold) {
      setState((prev) => ({
        ...prev,
        isPulling: false,
        pullDistance: 0,
        isRefreshing: true,
      }));

      try {
        await onRefresh();
      } finally {
        setState((prev) => ({ ...prev, isRefreshing: false }));
      }
    } else {
      setState((prev) => ({ ...prev, isPulling: false, pullDistance: 0 }));
    }

    touchStartY.current = 0;
  }, [enabled, state.pullDistance, state.isRefreshing, threshold, onRefresh]);

  useEffect(() => {
    const target = scrollTarget === "window" ? window : scrollableRef.current;
    if (!target || !enabled) return;

    const options = { passive: false };
    target.addEventListener("touchstart", handleTouchStart, options);
    target.addEventListener("touchmove", handleTouchMove, options);
    target.addEventListener("touchend", handleTouchEnd);

    return () => {
      target.removeEventListener("touchstart", handleTouchStart);
      target.removeEventListener("touchmove", handleTouchMove);
      target.removeEventListener("touchend", handleTouchEnd);
    };
  }, [enabled, scrollTarget, handleTouchStart, handleTouchMove, handleTouchEnd]);

  return {
    ...state,
  };
};
