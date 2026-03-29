import { createContext, type ReactNode, useCallback, useContext, useEffect, useState } from "react";

const STORAGE_KEY = "eddist:ui:settings";

const isBrowser = typeof window !== "undefined";

interface UISettings {
  showHistoryButtons: boolean;
  enableReadHistory: boolean;
  enableFavorites: boolean;
  enablePostHistory: boolean;
}

const defaultSettings = (): UISettings => ({
  showHistoryButtons: true,
  enableReadHistory: true,
  enableFavorites: true,
  enablePostHistory: true,
});

const loadSettings = (): UISettings => {
  if (!isBrowser) return defaultSettings();
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return defaultSettings();
    return { ...defaultSettings(), ...JSON.parse(stored) };
  } catch {
    return defaultSettings();
  }
};

interface UISettingsContextValue {
  settings: UISettings;
  setSetting: <K extends keyof UISettings>(key: K, value: UISettings[K]) => void;
}

const UISettingsContext = createContext<UISettingsContextValue | null>(null);

export const UISettingsProvider = ({ children }: { children: ReactNode }) => {
  const [settings, setSettings] = useState<UISettings>(loadSettings);

  useEffect(() => {
    if (!isBrowser) return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch {
      // ignore
    }
  }, [settings]);

  const setSetting = useCallback(<K extends keyof UISettings>(key: K, value: UISettings[K]) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
  }, []);

  return (
    <UISettingsContext.Provider value={{ settings, setSetting }}>
      {children}
    </UISettingsContext.Provider>
  );
};

export const useUISettings = (): UISettingsContextValue => {
  const context = useContext(UISettingsContext);
  if (!context) throw new Error("useUISettings must be used within UISettingsProvider");
  return context;
};
