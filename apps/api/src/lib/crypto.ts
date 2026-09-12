/**
 * Primitives cryptographiques.
 *
 * Tout provient de Bun ou de WebCrypto : aucun paquet tiers n'est requis pour
 * le hachage des mots de passe (Argon2id intégré à Bun), la signature ES256 des
 * jetons APNs, ou la génération d'aléa.
 */
import { randomBytes, randomUUID } from "node:crypto";

/** SHA-256 hexadécimal, utilisé pour les index sans exposition de la valeur. */
export function sha256Hex(value: string): string {
  return new Bun.CryptoHasher("sha256").update(value).digest("hex");
}

/** Normalise un e-mail avant hachage : minuscules, espaces retirés. */
export function normalizeEmail(email: string): string {
  return email.trim().toLowerCase();
}

export function emailHash(email: string): string {
  return sha256Hex(normalizeEmail(email));
}

/** Jeton opaque adapté aux URL, 256 bits d'entropie. */
export function opaqueToken(bytes = 32): string {
  return randomBytes(bytes).toString("base64url");
}

export function uuid(): string {
  return randomUUID();
}

/** Code de connexion à six chiffres, tiré uniformément. */
export function otpCode(): string {
  const buffer = randomBytes(4);
  const value = buffer.readUInt32BE(0) % 1_000_000;
  return value.toString().padStart(6, "0");
}

/** Hachage d'un secret court (code OTP, jeton de rafraîchissement). */
export async function hashSecret(secret: string): Promise<string> {
  return Bun.password.hash(secret, { algorithm: "argon2id", memoryCost: 19_456, timeCost: 2 });
}

export async function verifySecret(secret: string, hash: string): Promise<boolean> {
  try {
    return await Bun.password.verify(secret, hash);
  } catch {
    return false;
  }
}

/** Comparaison à temps constant de deux chaînes. */
export function timingSafeEqual(a: string, b: string): boolean {
  const left = Buffer.from(a);
  const right = Buffer.from(b);
  if (left.length !== right.length) return false;
  return require("node:crypto").timingSafeEqual(left, right);
}

/* ------------------------------------------------------------------ */
/* URL signées pour les médias                                         */
/* ------------------------------------------------------------------ */

/**
 * Signe l'accès à un objet média. Les photos et enregistrements vocaux ne sont
 * jamais servis par une URL devinable : chaque lecture passe par une signature
 * à durée limitée, liée au niveau de révélation demandé.
 */
export function signMediaUrl(
  baseUrl: string,
  secret: string,
  objectKey: string,
  options: { expiresInSeconds: number; blur?: number },
): string {
  const expires = Math.floor(Date.now() / 1000) + options.expiresInSeconds;
  const blur = options.blur ?? 0;
  const payload = `${objectKey}:${expires}:${blur}`;
  const signature = new Bun.CryptoHasher("sha256", secret).update(payload).digest("base64url");
  const url = new URL(`${baseUrl.replace(/\/$/, "")}/${encodeURIComponent(objectKey)}`);
  url.searchParams.set("exp", String(expires));
  url.searchParams.set("blur", String(blur));
  url.searchParams.set("sig", signature);
  return url.toString();
}

/** Vérifie une URL signée. Renvoie `false` si expirée ou altérée. */
export function verifyMediaSignature(
  secret: string,
  objectKey: string,
  expires: number,
  blur: number,
  signature: string,
): boolean {
  if (expires * 1000 < Date.now()) return false;
  const expected = new Bun.CryptoHasher("sha256", secret)
    .update(`${objectKey}:${expires}:${blur}`)
    .digest("base64url");
  return timingSafeEqual(expected, signature);
}
