/** Erreurs applicatives, converties en réponses HTTP par le serveur. */
import { ERROR_CODES, type ApiError, type ErrorCode } from "@weave/contracts";

const STATUS: Record<ErrorCode, number> = {
  [ERROR_CODES.UNAUTHORIZED]: 401,
  [ERROR_CODES.FORBIDDEN]: 403,
  [ERROR_CODES.NOT_FOUND]: 404,
  [ERROR_CODES.VALIDATION]: 422,
  [ERROR_CODES.RATE_LIMITED]: 429,
  [ERROR_CODES.LOOM_FULL]: 409,
  [ERROR_CODES.THREAD_GONE]: 410,
  [ERROR_CODES.ENTITLEMENT_REQUIRED]: 402,
  [ERROR_CODES.ALREADY_EXTENDED]: 409,
  [ERROR_CODES.UPSTREAM]: 502,
  [ERROR_CODES.INTERNAL]: 500,
};

export class AppError extends Error {
  readonly code: ErrorCode;
  readonly status: number;
  readonly details: unknown;

  constructor(code: ErrorCode, message: string, details?: unknown) {
    super(message);
    this.name = "AppError";
    this.code = code;
    this.status = STATUS[code];
    this.details = details;
  }

  toJSON(): ApiError {
    return this.details === undefined
      ? { error: this.code, message: this.message }
      : { error: this.code, message: this.message, details: this.details };
  }
}

export const unauthorized = (message = "Authentification requise.") =>
  new AppError(ERROR_CODES.UNAUTHORIZED, message);

export const forbidden = (message = "Action non autorisée.") =>
  new AppError(ERROR_CODES.FORBIDDEN, message);

export const notFound = (message = "Ressource introuvable.") =>
  new AppError(ERROR_CODES.NOT_FOUND, message);

export const invalid = (message: string, details?: unknown) =>
  new AppError(ERROR_CODES.VALIDATION, message, details);

export const rateLimited = (message = "Trop de requêtes. Réessayez dans un instant.") =>
  new AppError(ERROR_CODES.RATE_LIMITED, message);

export const loomFull = () =>
  new AppError(
    ERROR_CODES.LOOM_FULL,
    "Votre métier est complet. Dénouez un fil pour faire de la place.",
  );

export const threadGone = () =>
  new AppError(ERROR_CODES.THREAD_GONE, "Ce fil s'est dénoué : il n'existe plus.");

export const entitlementRequired = (message: string, details?: unknown) =>
  new AppError(ERROR_CODES.ENTITLEMENT_REQUIRED, message, details);
