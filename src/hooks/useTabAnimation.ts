import { useState, useRef, useEffect } from "react";
import { TAB_ORDER } from "../constants";

export function useTabAnimation() {
  const [activeTab, setActiveTab] = useState('all');
  const [renderTab, setRenderTab] = useState('all');
  const [listStyle, setListStyle] = useState({
    opacity: 1,
    transform: 'translateX(0px)',
    transition: 'none'
  });
  const tabSwitchTimerRef = useRef<number | null>(null);

  const handleTabChange = (id: string) => {
    const newIndex = TAB_ORDER.indexOf(id);
    const oldIndex = TAB_ORDER.indexOf(activeTab);
    const direction = newIndex > oldIndex ? -1 : 1;

    setActiveTab(id);

    if (id !== renderTab) {
      setListStyle({
        opacity: 0,
        transform: `translateX(${direction * -30}px)`,
        transition: 'all 0.2s ease-in'
      });

      if (tabSwitchTimerRef.current) {
        window.clearTimeout(tabSwitchTimerRef.current);
      }

      tabSwitchTimerRef.current = window.setTimeout(() => {
        setRenderTab(id);
        setListStyle({
          opacity: 0,
          transform: `translateX(${direction * 30}px)`,
          transition: 'none'
        });

        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            setListStyle({
              opacity: 1,
              transform: 'translateX(0px)',
              transition: 'all 0.25s cubic-bezier(0.2, 0.8, 0.2, 1)'
            });
          });
        });

        tabSwitchTimerRef.current = null;
      }, 200);
    }
  };

  useEffect(() => {
    return () => {
      if (tabSwitchTimerRef.current) {
        window.clearTimeout(tabSwitchTimerRef.current);
        tabSwitchTimerRef.current = null;
      }
    };
  }, []);

  return { activeTab, renderTab, listStyle, handleTabChange };
}
