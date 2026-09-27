export const THREAD_LIST_SKELETON_ATTR = "data-thread-list-skeleton";
// If hydration never runs (e.g. stale chunks after a deploy), reveal the SSR'd list anyway.
export const THREAD_LIST_SKELETON_FALLBACK_MS = 5000;

interface NgRuleLike {
  pattern: string;
  matchType: "partial" | "regex";
  enabled: boolean;
}

export const matchesNgRule = (
  text: string | undefined,
  rule: NgRuleLike,
  toRegExp: (pattern: string) => RegExp | null,
): boolean => {
  if (!rule.enabled || !text) return false;
  if (rule.matchType === "regex") {
    const regex = toRegExp(rule.pattern);
    return regex ? regex.test(text) : false;
  }
  return text.toLowerCase().includes(rule.pattern.toLowerCase());
};

export interface SkeletonCheckData {
  attr: string;
  storageKey: string;
  hasUnsafe: boolean;
  hasShared: boolean;
  fallbackMs: number;
  rows: [string, string | null][];
}

const applyThreadListSkeleton = (data: SkeletonCheckData, matches: typeof matchesNgRule) => {
  const flag = () => {
    const root = document.documentElement;
    root.setAttribute(data.attr, "");
    setTimeout(() => root.removeAttribute(data.attr), data.fallbackMs);
  };
  try {
    if (data.hasUnsafe && /(?:^|;\s*)safe_mode=off(?:;|$)/.test(document.cookie)) {
      flag();
      return;
    }
    const stored = localStorage.getItem(data.storageKey);
    if (!stored) return;
    let config: {
      version?: number;
      thread?: { authorIds?: NgRuleLike[]; titles?: NgRuleLike[] };
      response?: unknown;
      sharedNg?: { enabled?: boolean };
    };
    try {
      config = JSON.parse(stored);
    } catch {
      return; // NGWordsProvider falls back to the default config, which filters nothing extra.
    }
    if (!config?.version || !config.thread || !config.response) return;
    if (data.hasShared && config.sharedNg?.enabled === false) {
      flag();
      return;
    }
    const toRegExp = (pattern: string) => {
      try {
        return new RegExp(pattern, "i");
      } catch {
        return null;
      }
    };
    const authorIdRules = config.thread.authorIds ?? [];
    const titleRules = config.thread.titles ?? [];
    const hit = data.rows.some(
      ([title, authorId]) =>
        authorIdRules.some((rule) => matches(authorId ?? undefined, rule, toRegExp)) ||
        titleRules.some((rule) => matches(title, rule, toRegExp)),
    );
    if (hit) flag();
  } catch {
    flag();
  }
};

// JSON inside <script>: escape "<" so user-supplied titles cannot close the element.
const toScriptJson = (value: unknown) =>
  JSON.stringify(value)
    .replace(/</g, "\\u003c")
    .replace(/\u2028/g, "\\u2028")
    .replace(/\u2029/g, "\\u2029");

export const buildThreadListSkeletonScript = (data: SkeletonCheckData) =>
  `(${applyThreadListSkeleton.toString()})(${toScriptJson(data)},${matchesNgRule.toString()})`;
