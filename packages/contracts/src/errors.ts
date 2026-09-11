/** Codes d'erreur applicatifs, stables et partagés par tous les clients. */
export const ERROR_CODES = {
  UNAUTHORIZED: "unauthorized",
  FORBIDDEN: "forbidden",
  NOT_FOUND: "not_found",
  VALIDATION: "validation",
  RATE_LIMITED: "rate_limited",
  /** Le métier est plein : trois fils sont déjà actifs. */
  LOOM_FULL: "loom_full",
  /** Le fil est dénoué : il n'existe plus en cache. */
  THREAD_GONE: "thread_gone",
  /** Le palier ou les crédits ne permettent pas cette action. */
  ENTITLEMENT_REQUIRED: "entitlement_required",
  /** Prolonge déjà utilisée sur ce fil. */
  ALREADY_EXTENDED: "already_extended",
  UPSTREAM: "upstream_unavailable",
  INTERNAL: "internal",
} as const;

export type ErrorCode = (typeof ERROR_CODES)[keyof typeof ERROR_CODES];

export interface ApiError {
  readonly error: ErrorCode;
  readonly message: string;
  readonly details?: unknown;
}
