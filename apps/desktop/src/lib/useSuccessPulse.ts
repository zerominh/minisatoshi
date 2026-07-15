import { useCallback, useEffect, useRef, useState } from "react";

/** Brief success key for button flash (class `btn-ok`). */
export function useSuccessPulse(durationMs = 1600) {
  const [pulse, setPulse] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (timer.current) clearTimeout(timer.current);
    };
  }, []);

  const flash = useCallback(
    (key: string) => {
      setPulse(key);
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => setPulse(null), durationMs);
    },
    [durationMs],
  );

  const is = useCallback((key: string) => pulse === key, [pulse]);

  return { pulse, flash, is };
}
