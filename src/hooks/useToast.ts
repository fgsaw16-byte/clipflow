import { useState, useRef, useEffect } from "react";

export function useToast() {
  const [toastMsg, setToastMsg] = useState("");
  const toastTimerRef = useRef<number | null>(null);

  const showToast = (message: string) => {
    setToastMsg(message);
    if (toastTimerRef.current) window.clearTimeout(toastTimerRef.current);
    toastTimerRef.current = window.setTimeout(() => setToastMsg(""), 1800);
  };

  useEffect(() => {
    return () => {
      if (toastTimerRef.current) window.clearTimeout(toastTimerRef.current);
    };
  }, []);

  return { toastMsg, showToast };
}
