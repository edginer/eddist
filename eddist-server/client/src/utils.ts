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
  authCode: string;
}

const convertToSjisText = (text: string): string => {
  const sjis = Encoding.convert(Encoding.stringToCode(text), {
    to: "SJIS",
    from: "UNICODE",
  });
  return Encoding.urlEncode(sjis);
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
  if (!res.ok) {
    throw new Error(`Failed to post a response: ${res.statusText}`);
  }

  const text = await res.text();
  if (text.includes("error_code") && text.includes("E-Unauthenticated")) {
    const authCode = extractAuthCodeWhenUnauthenticated(text);
    return { success: false, authCode };
  }

  return { success: true };
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
  if (!res.ok) {
    throw new Error(`Failed to post a response: ${res.statusText}`);
  }

  const text = await res.text();
  if (text.includes("error_code") && text.includes("E-Unauthenticated")) {
    const authCode = extractAuthCodeWhenUnauthenticated(text);
    return { success: false, authCode };
  }

  return { success: true };
};
