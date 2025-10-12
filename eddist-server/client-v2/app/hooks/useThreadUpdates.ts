import { useEffect, useRef } from "react";

interface UseThreadUpdatesOptions {
  /**
   * Called when receiving an update notification from the server
   */
  onUpdate?: () => void;

  /**
   * Called when the WebSocket connection is established
   */
  onOpen?: () => void;

  /**
   * Called when the WebSocket connection is closed
   */
  onClose?: () => void;

  /**
   * Called when an error occurs
   */
  onError?: (error: Event) => void;

  /**
   * Whether to automatically reconnect when the connection is closed
   * @default true
   */
  autoReconnect?: boolean;

  /**
   * Reconnection delay in milliseconds
   * @default 3000
   */
  reconnectDelay?: number;

  /**
   * Maximum number of reconnection attempts
   * @default 5
   */
  maxReconnectAttempts?: number;
}

/**
 * Custom hook to subscribe to real-time thread updates via WebSocket
 *
 * @param boardKey - The board key to subscribe to
 * @param threadNumber - The thread number to subscribe to
 * @param options - Configuration options
 *
 * @example
 * ```tsx
 * useThreadUpdates(boardKey, threadNumber, {
 *   onUpdate: () => {
 *     // Refetch thread data when update is received
 *     mutate();
 *   }
 * });
 * ```
 */
export const useThreadUpdates = (
  boardKey: string | undefined,
  threadNumber: string | undefined,
  options: UseThreadUpdatesOptions = {}
) => {
  const {
    onUpdate,
    onOpen,
    onClose,
    onError,
    autoReconnect = true,
    reconnectDelay = 3000,
    maxReconnectAttempts = 5,
  } = options;

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const shouldReconnectRef = useRef(true);

  useEffect(() => {
    // Don't connect if boardKey or threadNumber is missing
    if (!boardKey || !threadNumber) {
      return;
    }

    shouldReconnectRef.current = true;
    reconnectAttemptsRef.current = 0;

    const connect = () => {
      try {
        // Construct WebSocket URL
        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const wsUrl = `${protocol}//${window.location.host}/ws?board_key=${encodeURIComponent(
          boardKey
        )}&thread_number=${encodeURIComponent(threadNumber)}`;

        console.log(`[WebSocket] Connecting to ${wsUrl}`);
        const ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
          console.log(
            `[WebSocket] Connected to thread ${threadNumber} on board ${boardKey}`
          );
          reconnectAttemptsRef.current = 0; // Reset reconnection attempts on successful connection
          onOpen?.();
        };

        ws.onmessage = (event) => {
          console.log(
            `[WebSocket] Received update notification for thread ${threadNumber} on board ${boardKey}:`,
            event.data
          );
          // Empty message or any message means "thread updated, please refetch"
          onUpdate?.();
        };

        ws.onerror = (error) => {
          console.error(`[WebSocket] Error:`, error);
          onError?.(error);
        };

        ws.onclose = (event) => {
          console.log(
            `[WebSocket] Connection closed (code: ${event.code}, reason: ${event.reason})`
          );
          wsRef.current = null;
          onClose?.();

          // Attempt to reconnect if enabled and within max attempts
          if (
            shouldReconnectRef.current &&
            autoReconnect &&
            reconnectAttemptsRef.current < maxReconnectAttempts
          ) {
            reconnectAttemptsRef.current += 1;
            console.log(
              `[WebSocket] Reconnecting in ${reconnectDelay}ms (attempt ${reconnectAttemptsRef.current}/${maxReconnectAttempts})`
            );

            reconnectTimeoutRef.current = setTimeout(() => {
              if (shouldReconnectRef.current) {
                connect();
              }
            }, reconnectDelay);
          } else if (
            reconnectAttemptsRef.current >= maxReconnectAttempts
          ) {
            console.error(
              `[WebSocket] Max reconnection attempts (${maxReconnectAttempts}) reached. Giving up.`
            );
          }
        };
      } catch (error) {
        console.error(`[WebSocket] Failed to create WebSocket:`, error);
      }
    };

    connect();

    // Cleanup function
    return () => {
      console.log(`[WebSocket] Cleaning up connection`);
      shouldReconnectRef.current = false;

      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
        reconnectTimeoutRef.current = null;
      }

      if (wsRef.current) {
        // Close with code 1000 (normal closure)
        wsRef.current.close(1000, "Component unmounted");
        wsRef.current = null;
      }
    };
  }, [
    boardKey,
    threadNumber,
    autoReconnect,
    reconnectDelay,
    maxReconnectAttempts,
    onUpdate,
    onOpen,
    onClose,
    onError,
  ]);

  return wsRef.current;
};
