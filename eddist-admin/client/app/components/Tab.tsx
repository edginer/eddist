import React, { useEffect, useState } from "react";
import { twJoin } from "tailwind-merge";

interface ItemProps<TKey> {
  tabKey: TKey;
  children: React.ReactNode;
  tabLabel: React.ReactNode;
  id?: string;
}

interface Props<TKey extends string> {
  tabItems: ItemProps<TKey>[];
  onSelectedTabChange?: (tabKey: TKey) => void;
}

const Tab = <TKey extends string>({
  tabItems,
  onSelectedTabChange,
}: Props<TKey>) => {
  if (tabItems.length === 0) {
    throw new Error("Tab must have at least one child");
  }

  const [selectedTabKey, setSelectedTabKey] = useState<TKey>(
    tabItems[0].tabKey
  );
  useEffect(() => {
    if (onSelectedTabChange) {
      onSelectedTabChange(selectedTabKey);
    }
  }, [selectedTabKey, onSelectedTabChange]);

  return (
    <div className="sm:px-4 pt-4">
      <div className="mb-4 border-b border-gray-200 dark:border-gray-700">
        <ul
          className="flex flex-wrap -mb-px text-sm font-medium text-center"
          id="default-tab"
          role="tablist"
        >
          {tabItems.map((tabItem) => {
            return (
              <li className="mr-2" role="presentation" key={tabItem.tabKey}>
                <button
                  className={twJoin(
                    "inline-block p-4 border-b-2 rounded-t-lg",
                    selectedTabKey === tabItem.tabKey
                      ? "border-blue-600"
                      : "border-gray-200 hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300"
                  )}
                  id={tabItem.id ?? tabItem.tabKey}
                  type="button"
                  role="tab"
                  aria-selected="false"
                  onClick={() => setSelectedTabKey(tabItem.tabKey)}
                >
                  {tabItem.tabLabel}
                </button>
              </li>
            );
          })}
        </ul>
      </div>
      <div className="md:px-4">
        {tabItems.map((tabItem) => {
          if (tabItem.tabKey === selectedTabKey) {
            return tabItem.children;
          }
          return null;
        })}
      </div>
    </div>
  );
};

export default Tab;
