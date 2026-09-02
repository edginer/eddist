import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { EMPTY_SHARED_NG_LIST, type SharedNgList } from "~/api-client/ng_id";
import type { Response } from "~/api-client/thread";
import type { Thread } from "~/api-client/thread_list";

const STORAGE_KEY = "eddist:ng-words:config";
const DEBOUNCE_DELAY = 300;

export interface NGRule {
  id: string;
  pattern: string;
  matchType: "partial" | "regex";
  enabled: boolean;
  hideMode?: "hidden" | "collapsed";
  // Set only for rules added from a context menu and synchronized with the server.
  sharedBoardKey?: string;
}

// How the reader consumes the board-wide list assembled from other readers'
// contributions. The threshold that puts a value on that list lives on the server.
export interface SharedNGSettings {
  enabled: boolean;
  hideMode: "hidden" | "collapsed";
}

export interface NGWordsConfig {
  version: number;
  thread: {
    authorIds: NGRule[];
    titles: NGRule[];
  };
  response: {
    authorIds: NGRule[];
    names: NGRule[];
    bodies: NGRule[];
  };
  sharedNg: SharedNGSettings;
}

export type NGCategory =
  | "thread.authorIds"
  | "thread.titles"
  | "response.authorIds"
  | "response.names"
  | "response.bodies";

interface FilterResult {
  filtered: boolean;
  hideMode: "hidden" | "collapsed" | null;
  source: "user" | "shared" | null;
}

interface NGWordsContextValue {
  config: NGWordsConfig;
  sharedNgList: SharedNgList;
  setSharedNgList: (list: SharedNgList) => void;
  updateSharedNgSettings: (updates: Partial<SharedNGSettings>) => void;
  addRule: (category: NGCategory, rule: Omit<NGRule, "id">) => void;
  updateRule: (category: NGCategory, ruleId: string, updates: Partial<Omit<NGRule, "id">>) => void;
  removeRule: (category: NGCategory, ruleId: string) => void;
  toggleRule: (category: NGCategory, ruleId: string) => void;
  clearAllRules: () => void;
  shouldFilterThread: (thread: Thread) => boolean;
  shouldFilterResponse: (response: Response) => FilterResult;
}

const getDefaultSharedNgSettings = (): SharedNGSettings => ({
  enabled: true,
  hideMode: "collapsed",
});

const getDefaultConfig = (): NGWordsConfig => ({
  version: 1,
  thread: {
    authorIds: [],
    titles: [],
  },
  response: {
    authorIds: [],
    names: [],
    bodies: [],
  },
  sharedNg: getDefaultSharedNgSettings(),
});

type Scope = "thread" | "response";
type CategoryConfig = { scope: Scope; field: string };

const CATEGORY_MAP: Record<NGCategory, CategoryConfig> = {
  "thread.authorIds": { scope: "thread", field: "authorIds" },
  "thread.titles": { scope: "thread", field: "titles" },
  "response.authorIds": { scope: "response", field: "authorIds" },
  "response.names": { scope: "response", field: "names" },
  "response.bodies": { scope: "response", field: "bodies" },
};

/**
 * Generic updater for nested NGWordsConfig structure.
 * Takes a category and a transform function, returns a new config.
 */
const updateRulesByCategory = (
  prev: NGWordsConfig,
  category: NGCategory,
  transform: (rules: NGRule[]) => NGRule[],
): NGWordsConfig => {
  const { scope, field } = CATEGORY_MAP[category];
  const scopeConfig = prev[scope];

  return {
    ...prev,
    [scope]: {
      ...scopeConfig,
      [field]: transform(scopeConfig[field as keyof typeof scopeConfig] as NGRule[]),
    },
  };
};

const isBrowser = typeof window !== "undefined";

const loadConfig = (): NGWordsConfig => {
  if (!isBrowser) return getDefaultConfig();

  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return getDefaultConfig();

    const parsed = JSON.parse(stored);

    if (!parsed.version || !parsed.thread || !parsed.response) {
      console.error("Invalid NG config structure, resetting");
      return getDefaultConfig();
    }

    return { ...parsed, sharedNg: { ...getDefaultSharedNgSettings(), ...parsed.sharedNg } };
  } catch (error) {
    console.error("Failed to parse NG config, resetting:", error);
    return getDefaultConfig();
  }
};

const saveConfig = (config: NGWordsConfig): void => {
  if (!isBrowser) return;

  try {
    const configString = JSON.stringify(config);
    localStorage.setItem(STORAGE_KEY, configString);
  } catch (error) {
    if (error instanceof Error && error.name === "QuotaExceededError") {
      console.error("localStorage quota exceeded");
    } else {
      console.error("Failed to save NG config:", error);
    }
  }
};

const NGWordsContext = createContext<NGWordsContextValue | null>(null);

export const NGWordsProvider = ({ children }: { children: ReactNode }) => {
  const [config, setConfig] = useState<NGWordsConfig>(loadConfig);
  const [sharedNgList, setSharedNgListState] = useState<SharedNgList>(EMPTY_SHARED_NG_LIST);
  const isExternalUpdateRef = useRef(false);
  const isInitialMountRef = useRef(true);
  const regexCache = useRef<Map<string, RegExp | null>>(new Map());

  // Listen for storage changes from other tabs/windows
  useEffect(() => {
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === STORAGE_KEY && e.newValue) {
        try {
          const newConfig = JSON.parse(e.newValue);
          isExternalUpdateRef.current = true;
          setConfig({
            ...newConfig,
            sharedNg: { ...getDefaultSharedNgSettings(), ...newConfig.sharedNg },
          });
        } catch (error) {
          console.error("[NGWordsProvider] Failed to parse storage change:", error);
        }
      }
    };

    if (isBrowser) {
      window.addEventListener("storage", handleStorageChange);
    }

    return () => {
      if (isBrowser) {
        window.removeEventListener("storage", handleStorageChange);
      }
    };
  }, []);

  // Save to localStorage (but not if the change came from external source or initial mount)
  useEffect(() => {
    // Skip saving on initial mount (when config is loaded from localStorage)
    if (isInitialMountRef.current) {
      isInitialMountRef.current = false;
      return;
    }

    // Skip saving if this was an external update
    if (isExternalUpdateRef.current) {
      isExternalUpdateRef.current = false;
      return;
    }

    const timeoutId = setTimeout(() => {
      saveConfig(config);
    }, DEBOUNCE_DELAY);

    return () => {
      clearTimeout(timeoutId);
    };
  }, [config]);

  // Clear regex cache when config changes
  // biome-ignore lint/correctness/useExhaustiveDependencies: config is intentionally watched to trigger cache clear on change
  useEffect(() => {
    regexCache.current.clear();
  }, [config]);

  const addRule = useCallback((category: NGCategory, rule: Omit<NGRule, "id">) => {
    const newRule: NGRule = {
      ...rule,
      id: isBrowser && crypto.randomUUID ? crypto.randomUUID() : `${Date.now()}-${Math.random()}`,
    };

    setConfig((prev) =>
      updateRulesByCategory(prev, category, (rules) => {
        const existing = rules.find(
          (r) => r.pattern === newRule.pattern && r.matchType === newRule.matchType,
        );
        if (!existing) {
          return [...rules, newRule];
        }
        if (!newRule.sharedBoardKey || existing.sharedBoardKey) {
          return rules;
        }
        return rules.map((r) =>
          r.id === existing.id ? { ...r, sharedBoardKey: newRule.sharedBoardKey } : r,
        );
      }),
    );
  }, []);

  const updateRule = useCallback(
    (category: NGCategory, ruleId: string, updates: Partial<Omit<NGRule, "id">>) => {
      setConfig((prev) =>
        updateRulesByCategory(prev, category, (rules) =>
          rules.map((r) => (r.id === ruleId ? { ...r, ...updates } : r)),
        ),
      );
    },
    [],
  );

  const removeRule = useCallback((category: NGCategory, ruleId: string) => {
    setConfig((prev) =>
      updateRulesByCategory(prev, category, (rules) => rules.filter((r) => r.id !== ruleId)),
    );
  }, []);

  const toggleRule = useCallback((category: NGCategory, ruleId: string) => {
    setConfig((prev) =>
      updateRulesByCategory(prev, category, (rules) =>
        rules.map((r) => (r.id === ruleId ? { ...r, enabled: !r.enabled } : r)),
      ),
    );
  }, []);

  const clearAllRules = useCallback(() => {
    setConfig((prev) => ({ ...getDefaultConfig(), sharedNg: prev.sharedNg }));
  }, []);

  const updateSharedNgSettings = useCallback((updates: Partial<SharedNGSettings>) => {
    setConfig((prev) => ({ ...prev, sharedNg: { ...prev.sharedNg, ...updates } }));
  }, []);

  // Board pages push the list they loaded on every render pass; bail out when it is
  // unchanged so navigating within one board does not re-render every response.
  const setSharedNgList = useCallback((list: SharedNgList) => {
    setSharedNgListState((prev) =>
      prev.ngIds.length === list.ngIds.length &&
      prev.threadMetadents.length === list.threadMetadents.length &&
      prev.ngIds.every((id, i) => id === list.ngIds[i]) &&
      prev.threadMetadents.every((m, i) => m === list.threadMetadents[i])
        ? prev
        : list,
    );
  }, []);

  // Shared entries are whole author IDs / metadents, so they match exactly rather
  // than as substrings the way user-authored partial rules do.
  const sharedNgIdSet = useMemo(() => new Set(sharedNgList.ngIds), [sharedNgList.ngIds]);
  const sharedMetadentSet = useMemo(
    () => new Set(sharedNgList.threadMetadents),
    [sharedNgList.threadMetadents],
  );

  // matchesRule with regex caching for performance
  const matchesRule = useCallback((text: string, rule: NGRule): boolean => {
    if (!rule.enabled || !text) return false;

    if (rule.matchType === "regex") {
      // Check cache first
      let regex = regexCache.current.get(rule.pattern);

      if (regex === undefined) {
        try {
          regex = new RegExp(rule.pattern, "i");
        } catch {
          regex = null; // Mark as invalid
        }
        regexCache.current.set(rule.pattern, regex);
      }

      if (!regex) return false;
      return regex.test(text);
    } else {
      // Partial match optimization: use toLowerCase() once
      const lowerText = text.toLowerCase();
      const lowerPattern = rule.pattern.toLowerCase();
      return lowerText.includes(lowerPattern);
    }
  }, []);

  const shouldFilterThread = useCallback(
    (thread: Thread): boolean => {
      if (thread.authorId && config.thread.authorIds.length > 0) {
        for (const rule of config.thread.authorIds) {
          if (matchesRule(thread.authorId, rule)) {
            return true;
          }
        }
      }

      if (config.thread.titles.length > 0) {
        for (const rule of config.thread.titles) {
          if (matchesRule(thread.title, rule)) {
            return true;
          }
        }
      }

      if (config.sharedNg.enabled && thread.authorId && sharedMetadentSet.has(thread.authorId)) {
        return true;
      }

      return false;
    },
    [config, matchesRule, sharedMetadentSet],
  );

  const shouldFilterResponse = useCallback(
    (response: Response): FilterResult => {
      if (config.response.authorIds.length > 0) {
        for (const rule of config.response.authorIds) {
          if (matchesRule(response.authorId, rule)) {
            return {
              filtered: true,
              hideMode: rule.hideMode || "collapsed",
              source: "user",
            };
          }
        }
      }

      if (config.response.names.length > 0) {
        for (const rule of config.response.names) {
          if (matchesRule(response.name, rule)) {
            return {
              filtered: true,
              hideMode: rule.hideMode || "collapsed",
              source: "user",
            };
          }
        }
      }

      if (config.response.bodies.length > 0) {
        const bodyText = response.bodyParts.map((part) => part.text).join("");
        for (const rule of config.response.bodies) {
          if (matchesRule(bodyText, rule)) {
            return {
              filtered: true,
              hideMode: rule.hideMode || "collapsed",
              source: "user",
            };
          }
        }
      }

      // Checked after the reader's own rules so an explicit rule keeps its hideMode.
      if (config.sharedNg.enabled && sharedNgIdSet.has(response.authorId)) {
        return {
          filtered: true,
          hideMode: config.sharedNg.hideMode,
          source: "shared",
        };
      }

      return {
        filtered: false,
        hideMode: null,
        source: null,
      };
    },
    [config, matchesRule, sharedNgIdSet],
  );

  const value: NGWordsContextValue = {
    config,
    sharedNgList,
    setSharedNgList,
    updateSharedNgSettings,
    addRule,
    updateRule,
    removeRule,
    toggleRule,
    clearAllRules,
    shouldFilterThread,
    shouldFilterResponse,
  };

  return <NGWordsContext.Provider value={value}>{children}</NGWordsContext.Provider>;
};

export const useNGWords = (): NGWordsContextValue => {
  const context = useContext(NGWordsContext);
  if (!context) {
    throw new Error("useNGWords must be used within NGWordsProvider");
  }
  return context;
};
