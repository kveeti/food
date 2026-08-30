function encodeBase64Url(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes))
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/, "");
}

function decodeBase64Url(value: string): Uint8Array<ArrayBuffer> {
  const base64 = value.replaceAll("-", "+").replaceAll("_", "/");
  const padded = base64.padEnd(Math.ceil(base64.length / 4) * 4, "=");
  const decoded = atob(padded);
  const bytes = new Uint8Array(decoded.length);
  for (let index = 0; index < decoded.length; index++) {
    bytes[index] = decoded.charCodeAt(index);
  }
  return bytes;
}

export class TokenCipher {
  private constructor(private readonly key: CryptoKey) {}

  static async create(key: Uint8Array): Promise<TokenCipher> {
    if (key.length !== 32) {
      throw new Error("Token encryption key must contain 32 bytes");
    }
    const keyBytes = new Uint8Array(key.length);
    keyBytes.set(key);
    return new TokenCipher(
      await crypto.subtle.importKey(
        "raw",
        keyBytes,
        "AES-GCM",
        false,
        ["encrypt", "decrypt"],
      ),
    );
  }

  async encrypt(value: string, associatedData: string): Promise<string> {
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const ciphertext = new Uint8Array(
      await crypto.subtle.encrypt(
        {
          name: "AES-GCM",
          iv,
          additionalData: new TextEncoder().encode(associatedData),
        },
        this.key,
        new TextEncoder().encode(value),
      ),
    );
    const encrypted = new Uint8Array(iv.length + ciphertext.length);
    encrypted.set(iv);
    encrypted.set(ciphertext, iv.length);
    return encodeBase64Url(encrypted);
  }

  async decrypt(value: string, associatedData: string): Promise<string> {
    try {
      const encrypted = decodeBase64Url(value);
      if (encrypted.length <= 12) {
        throw new Error("Encrypted token is too short");
      }
      const plaintext = await crypto.subtle.decrypt(
        {
          name: "AES-GCM",
          iv: encrypted.subarray(0, 12),
          additionalData: new TextEncoder().encode(associatedData),
        },
        this.key,
        encrypted.subarray(12),
      );
      return new TextDecoder().decode(plaintext);
    } catch (error) {
      throw new Error(
        "Stored token decryption failed because its key or encrypted value does not match",
        { cause: error },
      );
    }
  }
}
