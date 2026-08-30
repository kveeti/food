import { action, query, redirect } from "@solidjs/router";
import { deleteCookie, getCookie } from "@solidjs/start/http";
import { config } from "./config";
import { authCookieName } from "./cookies";
import { deleteSession, userForToken } from "./data/session";

const loginUrl = new URL("/auth/login", config.appUrl).href;

export const getCurrentUser = query(async () => {
  "use server";
  const user = await userForToken(getCookie(authCookieName));
  if (!user) throw redirect(loginUrl);
  return { 
    id: user.id,
    publicId: user.publicId,
    email: user.email,
    timezone: "Europe/Helsinki"
  };
}, "current-user");

export const logout = action(async () => {
  "use server";
  await deleteSession(getCookie(authCookieName));
  deleteCookie(authCookieName, { path: "/" });
  throw redirect(loginUrl);
});

