export interface Thread {
  title: string;
  id: number;
  responseCount: number;
  authorId?: string;
}

const convertSubjectTextToThreadList = (text: string): Thread[] => {
  const lines = text.split("\n");
  const threadList = lines
    .map((line) => {
      const lineRegex = /^(\d{9,10}\.dat)<>(.*) \((\d{1,5})\)$/;
      const lineRegexWithId =
        /^(\d{9,10}\.dat)<>(.*) \[(.{4,13})★\] \((\d{1,5})\)$/;
      const match = line.match(lineRegexWithId);
      if (match == null) {
        const match2 = line.match(lineRegex);
        if (match2 == null) {
          return undefined;
        }

        const id = parseInt(match2[1].split(".")[0]);
        const title = match2[2];
        const responseCount = parseInt(match2[3]);

        return {
          title,
          id,
          responseCount,
          authorId: undefined,
        };
      }
      const id = parseInt(match[1].split(".")[0]);
      const title = match[2];
      const authorId = match[3];
      const responseCount = parseInt(match[4]);

      return {
        title,
        id,
        responseCount,
        authorId,
      };
    })
    .filter((thread) => thread != null) as Thread[];
  return threadList;
};

export const fetchThreadList = async (boardKey: string) => {
  const res = await fetch(
    `${import.meta.env.VITE_SSR_BASE_URL}/${boardKey}/subject.txt`,
    {
      headers: {
        "Content-Type": "text/plain; charset=shift_jis",
      },
    }
  );
  const sjisText = await res.blob();
  const arrayBuffer = await sjisText.arrayBuffer();
  const text = new TextDecoder("shift_jis").decode(arrayBuffer);

  return convertSubjectTextToThreadList(text);
};
