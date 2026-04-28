import { useEffect, useRef, useState } from "react";

type UseWebSocketResult<T> = {
  lastMessage: T | null;
  readyState: number;
};

const BACKOFF_MS = [1000, 2000, 4000];

export function useWebSocket<T>(
  url: string,
  options?: { onMessage?: (data: T) => void },
): UseWebSocketResult<T> {
  const [lastMessage, setLastMessage] = useState<T | null>(null);
  const [readyState, setReadyState] = useState<number>(WebSocket.CLOSED);
  const reconnectAttemptRef = useRef(0);
  const reconnectTimerRef = useRef<number | null>(null);
  const socketRef = useRef<WebSocket | null>(null);
  const closedByUnmountRef = useRef(false);
  const onMessageRef = useRef(options?.onMessage);

  // Keep the callback ref up to date
  useEffect(() => {
    onMessageRef.current = options?.onMessage;
  }, [options?.onMessage]);

  useEffect(() => {
    if (!url) {
      setReadyState(WebSocket.CLOSED);
      return;
    }

    closedByUnmountRef.current = false;

    const connect = () => {
      const socket = new WebSocket(url);
      socketRef.current = socket;
      setReadyState(socket.readyState);

      socket.onopen = () => {
        reconnectAttemptRef.current = 0;
        setReadyState(WebSocket.OPEN);
      };

      socket.onmessage = (event: MessageEvent<string>) => {
        try {
          const data = JSON.parse(event.data) as T;
          setLastMessage(data);
          onMessageRef.current?.(data);
        } catch {
          // Ignore malformed messages to keep the hook resilient.
        }
      };

      socket.onerror = () => {
        setReadyState(WebSocket.CLOSING);
      };

      socket.onclose = () => {
        setReadyState(WebSocket.CLOSED);
        if (closedByUnmountRef.current) return;

        const attempt = reconnectAttemptRef.current;
        const backoff =
          BACKOFF_MS[Math.min(attempt, BACKOFF_MS.length - 1)] ?? BACKOFF_MS[0];
        reconnectAttemptRef.current += 1;
        reconnectTimerRef.current = window.setTimeout(connect, backoff);
      };
    };

    connect();

    return () => {
      closedByUnmountRef.current = true;
      if (reconnectTimerRef.current != null) {
        window.clearTimeout(reconnectTimerRef.current);
      }
      if (socketRef.current) {
        socketRef.current.close();
      }
    };
  }, [url]);

  return { lastMessage, readyState };
}
