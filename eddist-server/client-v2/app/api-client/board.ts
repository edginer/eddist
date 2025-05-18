export const fetchBoards: (options?: {
  baseUrl: string;
}) => Promise<Board[]> = async (options) => {
  console.log(import.meta);
  console.log(options);
  return await fetch(
    `${(import.meta.env.SSR && options?.baseUrl) || ""}/api/boards`
  ).then((res) => res.json() satisfies Promise<Board[]>);
};

export interface Board {
  name: string;
  board_key: string;
  default_name: string;
}
