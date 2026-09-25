// The thread list SSR output is shared through the CDN cache, so it is rendered for the
// default reader: safe mode on, shared NG on, no personal NG rules. Readers whose own
// settings would change the first rows after hydration (a layout shift) are flagged by an
// inline script before first paint, and a skeleton is shown until the client list is ready.

export const THREAD_LIST_SKELETON_ATTR = "data-thread-list-skeleton";

interface NgRuleLike {
  pattern: string;
  matchType: "partial" | "regex";
  enabled: boolean;
}

// Serialized with Function#toString into the inline script: must stay self-contained.
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
  // Opting out of safe mode / shared NG would add rows the SSR output left out.
  hasUnsafe: boolean;
  hasShared: boolean;
  // [title, authorId] of the SSR'd rows; a personal NG rule matching one would remove it.
  rows: [string, string | null][];
}

// Serialized with Function#toString into the inline script: must stay self-contained.
const applyThreadListSkeleton = (data: SkeletonCheckData, matches: typeof matchesNgRule) => {
  const flag = () => document.documentElement.setAttribute(data.attr, "");
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
