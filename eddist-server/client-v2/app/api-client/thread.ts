export interface Response {
  name: string;
  mail: string;
  date: string;
  authorId: string;
  body: string;
  id: number;
  authorIdAppearBeforeCount: number;
}

export const fetchThread = async (boardKey: string, threadKey: string) => {
  const res = await fetch(
    `${
      import.meta.env.SSR
        ? import.meta.env.VITE_EDDIST_SERVER_URL
        : import.meta.env.VITE_SSR_BASE_URL
    }/${boardKey}/dat/${threadKey}.dat`,
    {
      headers: {
        "Content-Type": "text/plain; charset=shift_jis",
      },
      redirect:
        // in server side, follow redirect
        // in client side, do not follow redirect
        import.meta.env.SSR ? "follow" : "manual",
    }
  );
  const sjisText = await res.blob();
  const arrayBuffer = await sjisText.arrayBuffer();
  const text = new TextDecoder("shift_jis").decode(arrayBuffer);
  return convertThreadTextToResponseList(text);
};

const convertThreadTextToResponseList = (text: string) => {
  const lines = text.split("\n").filter((x) => x !== "");
  let threadTitle = "";

  const idMap = new Map<string, [Response, number][]>();
  const authorIdAppearBeforeCountMap = new Map<string, number>();

  const responses = lines.map((line, idx) => {
    const lineRegex = /^(.*)<>(.*)<>(.*) ID:(.*)<>(.*)<>(.*)$/;
    const match = line.match(lineRegex);
    if (match == null) {
      // あぼーん<>あぼーん<><> あぼーん<> てす
      const aboneRegex = /^(.*)<>(.*)<><> あぼーん<>(.*)$/;
      const aboneMatch = line.match(aboneRegex);
      if (aboneMatch == null) {
        throw new Error(`Invalid response line: ${line}`);
      }

      if (idx === 0) {
        threadTitle = aboneMatch[3];
      }

      return {
        name: aboneMatch[1],
        mail: "",
        date: "",
        authorId: "",
        body: "あぼーん",
        id: idx + 1,
        authorIdAppearBeforeCount: 0,
      };
    }
    const name = match[1];
    const mail = match[2];
    const date = match[3];
    const authorId = match[4];
    const body = match[5];
    if (idx === 0) {
      threadTitle = match[6];
    }

    if (authorIdAppearBeforeCountMap.has(authorId)) {
      const count = authorIdAppearBeforeCountMap.get(authorId)!;
      authorIdAppearBeforeCountMap.set(authorId, count + 1);
    } else {
      authorIdAppearBeforeCountMap.set(authorId, 1);
    }

    const response = {
      name,
      mail,
      date,
      authorId,
      body,
      id: idx + 1,
      authorIdAppearBeforeCount: authorIdAppearBeforeCountMap.get(authorId)!,
    };

    if (!idMap.has(authorId)) {
      idMap.set(authorId, []);
    }
    idMap.get(authorId)?.push([response, idx]);

    return response;
  });

  return {
    threadName: threadTitle,
    responses: responses satisfies Response[],
    authorIdMap: idMap,
  };
};
