import { customAlphabet } from "nanoid";
const createNanoid = customAlphabet(
  "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
);

export function createUserId() {
  return `usr_${createNanoid(12)}`;
}
