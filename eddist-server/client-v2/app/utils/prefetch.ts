type ModuleLoader = () => Promise<unknown>;

const warm = (load: ModuleLoader) => {
  load().catch(() => {});
};

// Modal chunks are otherwise fetched during the click that opens them, so the
// Suspense fallback stalls the interaction. Idle time keeps this off the TBT path.
export const schedulePrefetch = (loaders: readonly ModuleLoader[]) => {
  const run = () => {
    for (const load of loaders) warm(load);
  };

  if (typeof window.requestIdleCallback === "function") {
    const id = window.requestIdleCallback(run, { timeout: 2000 });
    return () => window.cancelIdleCallback(id);
  }

  const id = window.setTimeout(run, 1500);
  return () => window.clearTimeout(id);
};

// Covers taps landing before the idle pass: pointerdown/focus precede click.
export const intentPrefetch = (load: ModuleLoader) => ({
  onPointerEnter: () => warm(load),
  onPointerDown: () => warm(load),
  onFocus: () => warm(load),
});
