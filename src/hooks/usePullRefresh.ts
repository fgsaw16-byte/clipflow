import { useState, useRef } from "react";

interface UsePullRefreshParams {
  forceSync: () => Promise<void>;
}

export function usePullRefresh({ forceSync }: UsePullRefreshParams) {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const pullStartYRef = useRef<number | null>(null);
  const pullDistanceRef = useRef(0);
  const pullIndicatorRef = useRef<HTMLDivElement>(null);
  const pullIconRef = useRef<HTMLDivElement>(null);
  const pullTextRef = useRef<HTMLSpanElement>(null);
  const PULL_THRESHOLD = 60;

  const updatePullUI = (dist: number, dragging: boolean) => {
    pullDistanceRef.current = dist;
    const el = pullIndicatorRef.current;
    const icon = pullIconRef.current;
    const text = pullTextRef.current;
    if (!el) return;
    const t = dragging ? 'none' : 'height 0.3s ease, opacity 0.3s ease';
    el.style.height = `${dist}px`;
    el.style.opacity = `${Math.min(dist / PULL_THRESHOLD, 1)}`;
    el.style.transition = t;
    if (icon) {
      icon.style.transform = `rotate(${(dist / PULL_THRESHOLD) * 360}deg)`;
      icon.style.transition = dragging ? 'none' : 'transform 0.3s ease';
    }
    if (text) {
      text.textContent = dist >= PULL_THRESHOLD ? '松开刷新' : '下拉刷新';
    }
  };

  const handlePullMouseDown = (e: React.MouseEvent, listRef: React.RefObject<HTMLDivElement | null>) => {
    if (listRef.current && listRef.current.scrollTop <= 0) {
      pullStartYRef.current = e.clientY;
    }
  };

  const handlePullMouseMove = (e: React.MouseEvent, listRef: React.RefObject<HTMLDivElement | null>) => {
    if (pullStartYRef.current === null || isRefreshing) return;
    const delta = e.clientY - pullStartYRef.current;
    if (delta > 0 && listRef.current && listRef.current.scrollTop <= 0) {
      e.preventDefault();
      updatePullUI(Math.min(delta * 0.4, 100), true);
    } else {
      updatePullUI(0, true);
    }
  };

  const handlePullMouseUp = () => {
    if (pullStartYRef.current !== null && pullDistanceRef.current >= PULL_THRESHOLD && !isRefreshing) {
      setIsRefreshing(true);
      updatePullUI(PULL_THRESHOLD, false);
      forceSync().finally(() => {
        setIsRefreshing(false);
        updatePullUI(0, false);
      });
    } else {
      updatePullUI(0, false);
    }
    pullStartYRef.current = null;
  };

  const handlePullMouseLeave = () => {
    if (!isRefreshing) updatePullUI(0, false);
    pullStartYRef.current = null;
  };

  return {
    isRefreshing,
    pullIndicatorRef,
    pullIconRef,
    pullTextRef,
    handlePullMouseDown,
    handlePullMouseMove,
    handlePullMouseUp,
    handlePullMouseLeave,
  };
}
