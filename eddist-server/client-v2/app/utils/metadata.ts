export const getCanonicalUrl = (
  publicBaseUrl: string,
  request: Request,
  pathname?: string,
  searchParams?: URLSearchParams,
): string => {
  const requestedUrl = new URL(request.url);
  const baseUrl = (publicBaseUrl || requestedUrl.origin).replace(/\/+$/, "");
  const url = new URL(pathname ?? requestedUrl.pathname, `${baseUrl}/`);

  url.pathname = url.pathname === "/" ? "/" : url.pathname.replace(/\/+$/, "");
  url.search = searchParams?.toString() ? `?${searchParams.toString()}` : "";
  url.hash = "";

  return url.toString();
};

export const toMetaText = (value: string, maxLength = 160): string => {
  const normalized = value
    .replace(/<br\s*\/?\s*>/gi, " ")
    .replace(/<[^>]*>/g, " ")
    .replace(/\s+/g, " ")
    .trim();

  if (normalized.length <= maxLength) return normalized;
  return `${normalized.slice(0, maxLength - 1)}…`;
};
