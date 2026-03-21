import { FaMoon, FaSun } from "react-icons/fa";
import { useTheme } from "~/contexts/ThemeContext";

interface ThemeToggleButtonProps {
  className?: string;
  iconClassName?: string;
}

const HEADER_BTN_CLASS =
  "px-3 py-2 lg:px-4 lg:py-2 text-sm lg:text-base rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors flex items-center";

export function ThemeToggleButton({
  className = HEADER_BTN_CLASS,
  iconClassName = "w-4 h-4",
}: ThemeToggleButtonProps) {
  const { isDark, toggleTheme } = useTheme();
  return (
    <button
      type="button"
      onClick={toggleTheme}
      className={className}
      title={isDark ? "ライトモードに切り替え" : "ダークモードに切り替え"}
    >
      {isDark ? (
        <FaSun className={iconClassName} />
      ) : (
        <FaMoon className={iconClassName} />
      )}
    </button>
  );
}
