/**
 * Primitives cryptographiques.
 *
 * Tout provient de `node:crypto` et de WebCrypto : aucun paquet tiers n'est
 * requis pour le hachage des secrets, la signature ES256 des jetons APNs, ou la
 * génération d'aléa.
 */
import {
  createHash,
  createHmac,
  randomBytes,
  randomUUID,
  scrypt as scryptCallback,
  timingSafeEqual as timingSafeEqualBuffers,
} from "node:crypto";
import { promisify } from "node:util";

const scrypt = promisify(scryptCallback) as (
  secret: string,
  salt: Buffer,
  keylen: number,
  options: { N: number; r: number; p: number; maxmem: number },
) => Promise<Buffer>;

/** SHA-256 hexadécimal, utilisé pour les index sans exposition de la valeur. */
export function sha256Hex(value: string): string {
  return createHash("sha256").update(value).digest("hex");
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

/**
 * Hachage d'un secret court — en pratique le code de connexion à six chiffres,
 * qui vit dix minutes.
 *
 * scrypt plutôt qu'Argon2id : il est intégré à `node:crypto`, là où Argon2
 * demanderait un module natif à compiler. Pour un secret à six chiffres dont la
 * durée de vie se compte en minutes et dont les tentatives sont déjà limitées
 * à cinq, la différence entre les deux fonctions ne protège rien de plus.
 *
 * Les paramètres suivent la recommandation de l'OWASP pour scrypt
 * (N = 2^17, r = 8, p = 1). Ils sont inscrits dans l'empreinte : les changer
 * plus tard n'invalidera pas les empreintes déjà produites.
 */
const SCRYPT = { N: 1 << 17, r: 8, p: 1, keylen: 32 } as const;

export async function hashSecret(secret: string): Promise<string> {
  const salt = randomBytes(16);
  const derive = await scrypt(secret, salt, SCRYPT.keylen, {
    N: SCRYPT.N,
    r: SCRYPT.r,
    p: SCRYPT.p,
    // Par défaut, Node plafonne la mémoire à 32 Mio et refuse N = 2^17.
    maxmem: 256 * 1024 * 1024,
  });
  return [
    "scrypt",
    SCRYPT.N,
    SCRYPT.r,
    SCRYPT.p,
    salt.toString("base64url"),
    derive.toString("base64url"),
  ].join("$");
}

export async function verifySecret(secret: string, hash: string): Promise<boolean> {
  try {
    const [schema, n, r, p, saltEncode, attenduEncode] = hash.split("$");
    if (schema !== "scrypt") return false;

    const attendu = Buffer.from(attenduEncode!, "base64url");
    const obtenu = await scrypt(secret, Buffer.from(saltEncode!, "base64url"), attendu.length, {
      N: Number(n),
      r: Number(r),
      p: Number(p),
      maxmem: 256 * 1024 * 1024,
    });

    return obtenu.length === attendu.length && timingSafeEqualBuffers(obtenu, attendu);
  } catch {
    return false;
  }
}

/** Comparaison à temps constant de deux chaînes. */
export function timingSafeEqual(a: string, b: string): boolean {
  const left = Buffer.from(a);
  const right = Buffer.from(b);
  if (left.length !== right.length) return false;
  return timingSafeEqualBuffers(left, right);
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
  const signature = createHmac("sha256", secret).update(payload).digest("base64url");
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
  const expected = createHmac("sha256", secret)
    .update(`${objectKey}:${expires}:${blur}`)
    .digest("base64url");
  return timingSafeEqual(expected, signature);
}
