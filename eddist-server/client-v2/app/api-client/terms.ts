export interface Terms {
  content: string;
  updated_at: string;
}

export async function fetchTerms({
  baseUrl,
}: {
  baseUrl: string;
}): Promise<Terms> {
  const response = await fetch(`${baseUrl}/api/terms`);
  if (!response.ok) {
    throw new Error("Failed to fetch terms");
  }
  return response.json();
}
