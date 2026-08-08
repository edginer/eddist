import type { ButtonHTMLAttributes } from "react";
import { twMerge } from "tailwind-merge";

const BASE =
  "flex items-center justify-center rounded-lg text-center font-medium text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800";
const SIZES = { sm: "text-sm px-3 py-1.5", md: "text-sm px-4 py-2" } as const;

export const Button = ({
  size = "md",
  color: _color,
  type = "button",
  className,
  children,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { size?: "sm" | "md"; color?: string }) => (
  <button type={type} className={twMerge(BASE, SIZES[size], className)} {...rest}>
    {children}
  </button>
);
