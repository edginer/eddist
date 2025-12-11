import { useState, useRef } from "react";
import { twMerge } from "tailwind-merge";

export interface Tab {
  id: string;
  title: string;
  content: React.ReactNode;
}

interface TabsProps {
  tabs: Tab[];
  defaultTab?: string;
}

export const Tabs = ({ tabs, defaultTab }: TabsProps) => {
  const [activeTab, setActiveTab] = useState(defaultTab ?? tabs[0]?.id);
  const tabRefs = useRef<Map<string, HTMLButtonElement>>(new Map());

  const handleKeyDown = (e: React.KeyboardEvent, currentIndex: number) => {
    let newIndex = currentIndex;

    if (e.key === "ArrowRight") {
      newIndex = (currentIndex + 1) % tabs.length;
      e.preventDefault();
    } else if (e.key === "ArrowLeft") {
      newIndex = (currentIndex - 1 + tabs.length) % tabs.length;
      e.preventDefault();
    } else if (e.key === "Home") {
      newIndex = 0;
      e.preventDefault();
    } else if (e.key === "End") {
      newIndex = tabs.length - 1;
      e.preventDefault();
    } else {
      return;
    }

    const newTab = tabs[newIndex];
    setActiveTab(newTab.id);
    tabRefs.current.get(newTab.id)?.focus();
  };

  return (
    <div>
      {/* Tab headers */}
      <div className="flex border-b border-gray-300" role="tablist">
        {tabs.map((tab, index) => {
          const isActive = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              ref={(el) => {
                if (el) tabRefs.current.set(tab.id, el);
              }}
              role="tab"
              aria-selected={isActive}
              aria-controls={`tabpanel-${tab.id}`}
              id={`tab-${tab.id}`}
              tabIndex={isActive ? 0 : -1}
              type="button"
              onClick={() => setActiveTab(tab.id)}
              onKeyDown={(e) => handleKeyDown(e, index)}
              className={twMerge(
                "px-4 py-2 font-medium transition-colors",
                isActive
                  ? "border-b-2 border-blue-600 text-blue-600"
                  : "text-gray-600 hover:text-gray-800"
              )}
            >
              {tab.title}
            </button>
          );
        })}
      </div>

      {/* Tab content */}
      {tabs.map((tab) => {
        const isActive = activeTab === tab.id;
        return (
          <div
            key={tab.id}
            role="tabpanel"
            id={`tabpanel-${tab.id}`}
            aria-labelledby={`tab-${tab.id}`}
            hidden={!isActive}
            className="py-4"
          >
            {isActive && tab.content}
          </div>
        );
      })}
    </div>
  );
};
