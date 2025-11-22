export interface Notice {
  slug: string;
  title: string;
  content: string;
  published_at: string;
}

export interface NoticeListItem {
  slug: string;
  title: string;
  published_at: string;
}

export interface NoticeListResponse {
  notices: NoticeListItem[];
  total: number;
  page: number;
  limit: number;
}

export async function fetchLatestNotices({
  baseUrl,
}: {
  baseUrl: string;
}): Promise<NoticeListItem[]> {
  const response = await fetch(`${baseUrl}/api/notices/latest`);
  if (!response.ok) {
    throw new Error("Failed to fetch latest notices");
  }
  return response.json();
}

export async function fetchNotices({
  baseUrl,
  page = 0,
  limit = 10,
}: {
  baseUrl: string;
  page?: number;
  limit?: number;
}): Promise<NoticeListResponse> {
  const response = await fetch(
    `${baseUrl}/api/notices?page=${page}&limit=${limit}`
  );
  if (!response.ok) {
    throw new Error("Failed to fetch notices");
  }
  return response.json();
}

export async function fetchNoticeBySlug({
  baseUrl,
  slug,
}: {
  baseUrl: string;
  slug: string;
}): Promise<Notice> {
  const response = await fetch(`${baseUrl}/api/notices/${slug}`);
  if (!response.ok) {
    throw new Error("Failed to fetch notice");
  }
  return response.json();
}
