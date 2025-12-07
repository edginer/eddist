import React from "react";

/**
 * Simple markdown parser supporting:
 * - Headers (h1-h6)
 * - Unordered lists (-, *, +)
 * - Ordered lists (1., 2., etc.)
 * - Horizontal rules (---, ***, ___)
 * - Links: [text](url) and plain URLs
 * - Bold: **text** or __text__
 */
export function parseMarkdown(content: string): React.ReactNode[] {
  const lines = content.split("\n");
  const elements: React.ReactNode[] = [];
  let listStack: Array<{ type: "ul" | "ol"; items: React.ReactNode[] }> = [];
  let currentParagraph: string[] = [];
  let key = 0;

  const flushParagraph = () => {
    if (currentParagraph.length > 0) {
      elements.push(
        <p key={`p-${key++}`} className="text-gray-700 leading-relaxed mb-3">
          {parseInline(currentParagraph.join("\n"))}
        </p>
      );
      currentParagraph = [];
    }
  };

  const flushList = () => {
    if (listStack.length > 0) {
      const list = listStack[listStack.length - 1];
      if (list.type === "ul") {
        elements.push(
          <ul
            key={`ul-${key++}`}
            className="list-disc list-inside text-gray-700 mb-3 space-y-1"
          >
            {list.items}
          </ul>
        );
      } else {
        elements.push(
          <ol
            key={`ol-${key++}`}
            className="list-decimal list-inside text-gray-700 mb-3 space-y-1"
          >
            {list.items}
          </ol>
        );
      }
      listStack = [];
    }
  };

  // Header styles mapping
  const headerStyles: Record<number, string> = {
    1: "text-3xl font-bold text-gray-900 mb-4 mt-6",
    2: "text-xl font-semibold text-gray-900 mb-3 mt-4",
    3: "text-lg font-semibold text-gray-900 mb-3 mt-4",
    4: "text-base font-semibold text-gray-900 mb-2 mt-3",
    5: "text-sm font-medium text-gray-900 mb-2 mt-3",
    6: "text-sm font-medium text-gray-900 mb-2 mt-2",
  };

  for (const line of lines) {
    // Headers
    const headerMatch = line.match(/^(#{1,6})\s+(.+)$/);
    if (headerMatch) {
      flushParagraph();
      flushList();
      const level = headerMatch[1].length as 1 | 2 | 3 | 4 | 5 | 6;
      const text = headerMatch[2];
      const Tag = `h${level}` as keyof JSX.IntrinsicElements;
      elements.push(
        <Tag key={`h-${key++}`} className={headerStyles[level]}>
          {parseInline(text)}
        </Tag>
      );
      continue;
    }

    // Unordered list
    const ulMatch = line.match(/^[-*+]\s+(.+)$/);
    if (ulMatch) {
      flushParagraph();
      if (
        listStack.length === 0 ||
        listStack[listStack.length - 1].type !== "ul"
      ) {
        flushList();
        listStack.push({ type: "ul", items: [] });
      }
      listStack[listStack.length - 1].items.push(
        <li key={`li-${key++}`}>{parseInline(ulMatch[1])}</li>
      );
      continue;
    }

    // Ordered list
    const olMatch = line.match(/^\d+\.\s+(.+)$/);
    if (olMatch) {
      flushParagraph();
      if (
        listStack.length === 0 ||
        listStack[listStack.length - 1].type !== "ol"
      ) {
        flushList();
        listStack.push({ type: "ol", items: [] });
      }
      listStack[listStack.length - 1].items.push(
        <li key={`li-${key++}`}>{parseInline(olMatch[1])}</li>
      );
      continue;
    }

    // Horizontal rule (---, ***, or ___)
    if (/^(-{3,}|\*{3,}|_{3,})\s*$/.test(line.trim())) {
      flushParagraph();
      flushList();
      elements.push(<hr key={`hr-${key++}`} className="my-8" />);
      continue;
    }

    // Empty line
    if (line.trim() === "") {
      flushParagraph();
      flushList();
      continue;
    }

    // Regular paragraph line
    flushList();
    currentParagraph.push(line);
  }

  flushParagraph();
  flushList();

  return elements;
}

function parseInline(text: string): React.ReactNode[] {
  const elements: React.ReactNode[] = [];
  let remaining = text;
  let key = 0;

  while (remaining.length > 0) {
    // Bold: **text** or __text__
    const boldMatch = remaining.match(/^(\*\*|__)(.+?)\1/);
    if (boldMatch) {
      elements.push(
        <strong key={`b-${key++}`} className="font-bold">
          {boldMatch[2]}
        </strong>
      );
      remaining = remaining.slice(boldMatch[0].length);
      continue;
    }

    // Link with text: [text](url)
    const linkMatch = remaining.match(/^\[([^\]]+)\]\(([^)]+)\)/);
    if (linkMatch) {
      elements.push(
        <a
          key={`a-${key++}`}
          href={linkMatch[2]}
          className="text-blue-500 underline"
          target="_blank"
          rel="noopener noreferrer"
        >
          {linkMatch[1]}
        </a>
      );
      remaining = remaining.slice(linkMatch[0].length);
      continue;
    }

    // Plain URL: http(s)://...
    const urlMatch = remaining.match(/^(https?:\/\/[^\s]+)/);
    if (urlMatch) {
      elements.push(
        <a
          key={`a-${key++}`}
          href={urlMatch[1]}
          className="text-blue-500 underline"
          target="_blank"
          rel="noopener noreferrer"
        >
          {urlMatch[1]}
        </a>
      );
      remaining = remaining.slice(urlMatch[0].length);
      continue;
    }

    // Regular text
    const nextSpecial = remaining.search(/(\*\*|__|https?:\/\/|\[)/);
    if (nextSpecial === -1) {
      elements.push(remaining);
      remaining = "";
    } else if (nextSpecial === 0) {
      // No match found, consume one character to avoid infinite loop
      elements.push(remaining[0]);
      remaining = remaining.slice(1);
    } else {
      elements.push(remaining.slice(0, nextSpecial));
      remaining = remaining.slice(nextSpecial);
    }
  }

  return elements;
}
