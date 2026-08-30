import { assertEquals, assertRejects } from "@std/assert";
import { TokenCipher } from "./token-cipher.ts";

Deno.test("token encryption is bound to its session and token kind", async () => {
  const cipher = await TokenCipher.create(new Uint8Array(32));
  const encrypted = await cipher.encrypt("secret-token", "session-1:access");

  assertEquals(
    await cipher.decrypt(encrypted, "session-1:access"),
    "secret-token",
  );
  await assertRejects(
    () => cipher.decrypt(encrypted, "session-2:access"),
    Error,
    "Stored token decryption failed",
  );
  await assertRejects(
    () => cipher.decrypt(encrypted, "session-1:refresh"),
    Error,
    "Stored token decryption failed",
  );
});
