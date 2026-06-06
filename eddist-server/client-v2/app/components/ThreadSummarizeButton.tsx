import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { FaMagic, FaSync } from "react-icons/fa";
import { fetchThread } from "~/api-client/thread";

interface Props {
  boardKey: string;
  threadId: number;
  threadTitle: string;
}

type State = "idle" | "loading" | "done" | "error";

const stripHtml = (html: string) =>
  html
    .replace(/<br\s*\/?>/gi, " ")
    .replace(/<[^>]+>/g, "")
    .replace(/&gt;/g, ">")
    .replace(/&lt;/g, "<")
    .replace(/&amp;/g, "&")
    .replace(/&nbsp;/g, " ")
    .trim();

const fitToQuota = async (summarizer: Summarizer, responses: string[]): Promise<string> => {
  const full = responses.join("\n\n");
  const usage = await summarizer.measureInputUsage(full);
  if (usage <= summarizer.inputQuota) return full;

  const ratio = summarizer.inputQuota / usage;
  let count = Math.max(1, Math.floor(responses.length * ratio * 0.9));
  let text = responses.slice(0, count).join("\n\n");

  const usage2 = await summarizer.measureInputUsage(text);
  if (usage2 > summarizer.inputQuota && count > 1) {
    count = Math.max(1, Math.floor(count * (summarizer.inputQuota / usage2) * 0.9));
    text = responses.slice(0, count).join("\n\n");
  }

  return text;
};

export const ThreadSummarizeButton = ({ boardKey, threadId, threadTitle }: Props) => {
  const [state, setState] = useState<State>("idle");
  const [summary, setSummary] = useState<string | null>(null);
  const [showPopover, setShowPopover] = useState(false);
  const [popoverStyle, setPopoverStyle] = useState<React.CSSProperties>({});
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!showPopover) return;
    const close = (e: MouseEvent) => {
      if (!buttonRef.current?.contains(e.target as Node)) {
        setShowPopover(false);
      }
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [showPopover]);

  const computePopoverStyle = (): React.CSSProperties => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (!rect) return {};
    const popoverWidth = 320;
    const left = Math.max(
      8,
      Math.min(rect.left + window.scrollX, window.innerWidth - popoverWidth - 8),
    );
    if (rect.top > 220) {
      return {
        position: "absolute",
        top: rect.top + window.scrollY - 8,
        left,
        width: popoverWidth,
        transform: "translateY(-100%)",
      };
    }
    return {
      position: "absolute",
      top: rect.bottom + window.scrollY + 8,
      left,
      width: popoverWidth,
    };
  };

  const handleClick = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    if (state === "done" || state === "error") {
      setPopoverStyle(computePopoverStyle());
      setShowPopover((v) => !v);
      return;
    }

    setPopoverStyle(computePopoverStyle());
    setShowPopover(true);
    setState("loading");

    try {
      const thread = await fetchThread(boardKey, String(threadId));
      const plainTitle = stripHtml(threadTitle);
      const responses = thread.responses
        .slice(0, 50)
        .map((r) => stripHtml(r.bodyParts.map((p) => p.text).join("")))
        .filter(Boolean);

      const availability = await Summarizer.availability();
      if (availability === "unavailable") throw new Error("Summarizer unavailable");

      const summarizer = await Summarizer.create({
        type: "key-points",
        format: "plain-text",
        length: "short",
        expectedInputLanguages: ["ja"],
        sharedContext: `日本語の掲示板スレッド「${plainTitle}」`,
      });
      const fittedText = await fitToQuota(summarizer, responses);
      const result = await summarizer.summarize(fittedText);
      summarizer.destroy();

      setSummary(result);
      setState("done");
    } catch (err) {
      console.error("Summarize error:", err);
      setState("error");
      setSummary("要約に失敗しました");
    }
  };

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        onClick={handleClick}
        title="要約"
        disabled={state === "loading"}
        className={`p-1.5 rounded text-xs transition-colors ${
          state === "loading"
            ? "text-gray-400 cursor-wait"
            : "text-purple-500 hover:text-purple-700 hover:bg-purple-100 dark:hover:bg-purple-900"
        } ${showPopover ? "bg-purple-100 dark:bg-purple-900" : ""}`}
      >
        {state === "loading" ? (
          <FaSync className="w-3 h-3 animate-spin" />
        ) : (
          <FaMagic className="w-3 h-3" />
        )}
      </button>

      {showPopover &&
        typeof document !== "undefined" &&
        createPortal(
          <div
            style={popoverStyle}
            className="z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-600 rounded-lg shadow-lg p-3 text-sm text-gray-800 dark:text-gray-200"
          >
            {state === "loading" ? (
              <div className="flex items-center gap-2 text-gray-500 dark:text-gray-400">
                <FaSync className="animate-spin w-3 h-3 shrink-0" />
                <span>要約中...</span>
              </div>
            ) : (
              <>
                <p className="whitespace-pre-wrap leading-relaxed">{summary}</p>
                <p className="mt-2 text-xs text-gray-400 dark:text-gray-500">
                  内容の正確性は保証しません
                </p>
              </>
            )}
          </div>,
          document.body,
        )}
    </>
  );
};
