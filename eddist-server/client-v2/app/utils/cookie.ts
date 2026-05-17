export const parseCookie = (cookieHeader: string, name: string): string | undefined => {
  const entry = cookieHeader
    .split(";")
    .map((s) => s.trim())
    .find((s) => s.startsWith(`${name}=`));
  return entry?.slice(name.length + 1);
};
