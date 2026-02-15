export interface Response {
  name: string;
  mail: string;
  date: string;
  authorId: string;
  bodyParts: BodyAnchorPart[];
  id: number;
  refs?: number[];
  authorIdAppearBeforeCount: number;
}

export interface BodyAnchorPart {
  text: string;
  isMatch: boolean;
  type: 'anchor' | 'url' | 'text';
}

export const fetchThread = async (
  boardKey: string,
  threadKey: string,
  options?:
    | {
        baseUrl: string;
      }
    | undefined
) => {
  const res = await fetch(
    `${
      (import.meta.env.SSR && options?.baseUrl) || ""
    }/${boardKey}/dat/${threadKey}.dat`,
    {
      headers: {
        "Content-Type": "text/plain; charset=shift_jis",
      },
      redirect:
        // in server side, follow redirect
        // in client side, do not follow redirect
        // import.meta.env.SSR ? "follow" : "manual",
        // TODO: for now, always manual
        "manual",
    }
  );
  const sjisText = await res.blob();
  const arrayBuffer = await sjisText.arrayBuffer();
  const text = new TextDecoder("shift_jis").decode(arrayBuffer);
  return {
    ...convertThreadTextToResponseList(text),
    redirected: res.redirected,
  };
};

const convertThreadTextToResponseList = (text: string) => {
  const lines = text.split("\n").filter((x) => x !== "");
  let threadTitle = "";

  const idMap = new Map<string, [Response, number][]>();
  const authorIdAppearBeforeCountMap = new Map<string, number>();
  const referredMap = new Map<number, number[]>();

  const responses: Response[] = lines.map((line, idx) => {
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
        bodyParts: [{ text: "あぼーん", isMatch: false, type: 'text' }],
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

    const [bodyParts, refs] = buildAnchorPartedBody(body);
    for (const ref of refs) {
      if (!referredMap.has(ref)) {
        referredMap.set(ref, []);
      }
      referredMap.get(ref)?.push(idx + 1);
    }

    const response = {
      name,
      mail,
      date,
      authorId,
      id: idx + 1,
      authorIdAppearBeforeCount: authorIdAppearBeforeCountMap.get(authorId)!,
      bodyParts,
    };

    if (!idMap.has(authorId)) {
      idMap.set(authorId, []);
    }
    idMap.get(authorId)?.push([response, idx]);

    return response;
  });

  for (const [refId, referredIds] of referredMap) {
    const response = responses[refId - 1];
    if (response) {
      response.refs = referredIds;
    }
  }

  return {
    threadName: threadTitle,
    responses: responses satisfies Response[],
    authorIdMap: idMap,
  };
};

const buildAnchorPartedBody = (body: string): [BodyAnchorPart[], number[]] => {
  const refs = [];
  const parts: BodyAnchorPart[] = [];

  // Combined regex to match both anchors and URLs
  // Match: >>digits OR http(s)://...
  const combinedRegex = /(&gt;&gt;(\d{1,4}))|(https?:\/\/[^\s<>"]+)/g;
  let lastIndex = 0;
  let match;

  while ((match = combinedRegex.exec(body)) !== null) {
    const { index } = match;

    // Add text before this match
    if (index > lastIndex) {
      const textBefore = body.slice(lastIndex, index);
      parts.push({ text: textBefore, isMatch: false, type: 'text' });
    }

    // Check if it's an anchor (>>123) or URL
    if (match[1]) {
      // It's an anchor match
      parts.push({
        text: match[1].replaceAll("&gt;", ">"),
        isMatch: true,
        type: 'anchor'
      });
      refs.push(parseInt(match[2]));
    } else if (match[3]) {
      // It's a URL match
      parts.push({
        text: match[3],
        isMatch: false,
        type: 'url'
      });
    }

    lastIndex = index + match[0].length;
  }

  // Add remaining text
  if (lastIndex < body.length) {
    parts.push({ text: body.slice(lastIndex), isMatch: false, type: 'text' });
  }

  return [parts, refs];
};
