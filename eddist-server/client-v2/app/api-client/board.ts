export const fetchBoards = async () => {
  return await fetch(`${import.meta.env.VITE_SSR_BASE_URL}/api/boards`).then(
    (res) => res.json() as Promise<Board[]>
  );
};

export interface Board {
  name: string;
  board_key: string;
  default_name: string;
}
