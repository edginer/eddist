export interface Notice {
  id: string;
  title: string;
  content: string;
  summary: string | null;
  created_at: string;
  updated_at: string;
  published_at: string;
  author_id: string | null;
}

export interface NoticeListItem {
  id: string;
  title: string;
  summary: string | null;
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

export async function fetchNoticeById({
  baseUrl,
  id,
}: {
  baseUrl: string;
  id: string;
}): Promise<Notice> {
  const response = await fetch(`${baseUrl}/api/notices/${id}`);
  if (!response.ok) {
    throw new Error("Failed to fetch notice");
  }
  return response.json();
}
