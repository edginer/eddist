import Encoding from "encoding-japanese";

interface ResponseInput {
  name: string;
  mail: string;
  body: string;
  boardKey: string;
  threadKey: string;
}

interface ThreadCreateInput {
  title: string;
  name: string;
  mail: string;
  body: string;
  boardKey: string;
}

type PostResponseResult = PostResponseSuccess | PostResponseFailure;

interface PostResponseSuccess {
  success: true;
}

interface PostResponseFailure {
  success: false;
  error: PostResponseFailureAuthCode | PostResponseFailureUnknown;
}

interface PostResponseFailureAuthCode {
  kind: "auth-code";
  authCode: string;
}

interface PostResponseFailureUnknown {
  kind: "unknown";
  errorHtml: string;
}

const convertToSjisText = (text: string): string => {
  const resultArray = [];

  for (let i = 0; i < text.length; i++) {
    const codePoint = text.codePointAt(i);
    if (codePoint == null) {
      throw new Error("Invalid code point");
    }

    // Move to the next index if the code point is a surrogate pair
    if (codePoint > 0xffff) i++;

    const char = String.fromCodePoint(codePoint);

    const encodedChar = Encoding.convert(char, {
      to: "SJIS",
      from: "UNICODE",
      type: "array",
    });

    // Check if encoding succeeded (non-Shift-JIS characters are usually replaced by '?')
    if (encodedChar.length === 1 && encodedChar[0] === 63) {
      const numericRef = `&#${codePoint};`;
      const encodedRef = Encoding.convert(numericRef, {
        to: "SJIS",
        from: "UNICODE",
        type: "array",
      });
      resultArray.push(...encodedRef);
    } else {
      resultArray.push(...encodedChar);
    }
  }

  return Encoding.urlEncode(new Uint8Array(resultArray));
};

const convertToUtf8Text = (text: ArrayBuffer): string => {
  const utf8 = Encoding.convert(new Uint8Array(text), {
    to: "UNICODE",
    from: "SJIS",
  });
  return Encoding.codeToString(utf8);
};

const extractAuthCodeWhenUnauthenticated = (text: string): string => {
  const extractedAuthCode = text.match(/'(\d{6})'/);
  const authCode = extractedAuthCode?.[1];
  if (authCode == null) {
    throw new Error("Unknown response from the server");
  }
  return authCode;
};

export const postResponse = async ({
  name,
  mail,
  body,
  boardKey,
  threadKey,
}: ResponseInput): Promise<PostResponseResult> => {
  const params = {
    submit: convertToSjisText("書き込む"),
    mail: convertToSjisText(mail),
    FROM: convertToSjisText(name),
    MESSAGE: convertToSjisText(body),
    bbs: boardKey,
    key: threadKey,
  };

  const res = await fetch(`/test/bbs.cgi`, {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body:
      "submit=" +
      params.submit +
      "&mail=" +
      (params.mail ?? "") +
      "&FROM=" +
      (params.FROM ?? "") +
      "&MESSAGE=" +
      params.MESSAGE +
      "&bbs=" +
      params.bbs +
      "&key=" +
      params.key,
  });

  return await afterPost(res);
};

export const postThread = async ({
  title,
  name,
  mail,
  body,
  boardKey,
}: ThreadCreateInput): Promise<PostResponseResult> => {
  const params = {
    submit: convertToSjisText("新規スレッド作成"),
    mail: convertToSjisText(mail),
    FROM: convertToSjisText(name),
    MESSAGE: convertToSjisText(body),
    bbs: boardKey,
    subject: convertToSjisText(title),
  };

  const res = await fetch(`/test/bbs.cgi`, {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body:
      "submit=" +
      params.submit +
      "&mail=" +
      (params.mail ?? "") +
      "&FROM=" +
      (params.FROM ?? "") +
      "&MESSAGE=" +
      params.MESSAGE +
      "&bbs=" +
      params.bbs +
      "&subject=" +
      params.subject,
  });

  return await afterPost(res);
};

const afterPost = async (res: Response): Promise<PostResponseResult> => {
  const bytes = await res.arrayBuffer();
  const text = convertToUtf8Text(bytes);

  if (!res.ok) {
    if (text.includes("error_code")) {
      const doc = new DOMParser().parseFromString(text, "text/html");
      return {
        success: false,
        error: {
          kind: "unknown",
          errorHtml: doc.body.innerHTML,
        },
      };
    }
    throw new Error(`Failed to post a response: ${res.statusText}`);
  }

  if (text.includes("error_code")) {
    if (text.includes("E-Unauthenticated")) {
      const authCode = extractAuthCodeWhenUnauthenticated(text);
      return { success: false, error: { kind: "auth-code", authCode } };
    } else {
      const doc = new DOMParser().parseFromString(text, "text/html");
      return {
        success: false,
        error: {
          kind: "unknown",
          errorHtml: doc.body.innerHTML,
        },
      };
    }
  }

  return { success: true };
};
