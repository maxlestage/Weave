/** Codes d'erreur applicatifs, stables et partagés par tous les clients. */
export const ERROR_CODES = {
  UNAUTHORIZED: "unauthorized",
  FORBIDDEN: "forbidden",
  NOT_FOUND: "not_found",
  VALIDATION: "validation",
  RATE_LIMITED: "rate_limited",
  /** Le quota de demandes du jour est épuisé. */
  NO_REQUESTS_LEFT: "no_requests_left",
  /** Trois plans sont déjà ouverts. */
  TOO_MANY_PLANS: "too_many_plans",
  /** Le plan est complet, passé ou annulé. */
  PLAN_CLOSED: "plan_closed",
  /** Une demande a déjà été envoyée pour ce plan. */
  ALREADY_REQUESTED: "already_requested",
  /** Le palier ou les crédits ne permettent pas cette action. */
  ENTITLEMENT_REQUIRED: "entitlement_required",
  UPSTREAM: "upstream_unavailable",
  INTERNAL: "internal",
} as const;

export type ErrorCode = (typeof ERROR_CODES)[keyof typeof ERROR_CODES];

export interface ApiError {
  readonly error: ErrorCode;
  readonly message: string;
  readonly details?: unknown;
}
