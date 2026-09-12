//! Erreurs applicatives, converties en réponses HTTP.
//!
//! Les codes et les statuts sont ceux de `@weave/contracts` : le site et
//! l'application iOS les lisent tels quels, et en changer un casserait les
//! deux sans que le compilateur Rust s'en aperçoive.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Unauthorized,
    Forbidden,
    NotFound,
    Validation,
    RateLimited,
    NoRequestsLeft,
    TooManyPlans,
    PlanClosed,
    AlreadyRequested,
    EntitlementRequired,
    Upstream,
    Internal,
}

impl Code {
    /// Le libellé rendu au client. Ce sont ceux des contrats partagés.
    pub fn as_str(self) -> &'static str {
        match self {
            Code::Unauthorized => "unauthorized",
            Code::Forbidden => "forbidden",
            Code::NotFound => "not_found",
            Code::Validation => "validation",
            Code::RateLimited => "rate_limited",
            Code::NoRequestsLeft => "no_requests_left",
            Code::TooManyPlans => "too_many_plans",
            Code::PlanClosed => "plan_closed",
            Code::AlreadyRequested => "already_requested",
            Code::EntitlementRequired => "entitlement_required",
            Code::Upstream => "upstream_unavailable",
            Code::Internal => "internal",
        }
    }

    pub fn status(self) -> StatusCode {
        match self {
            Code::Unauthorized => StatusCode::UNAUTHORIZED,
            Code::Forbidden => StatusCode::FORBIDDEN,
            Code::NotFound => StatusCode::NOT_FOUND,
            Code::Validation => StatusCode::UNPROCESSABLE_ENTITY,
            // Le quota épuisé est un 429 comme la limitation de débit : c'est
            // la même idée — revenez plus tard —, pas la même cause.
            Code::RateLimited | Code::NoRequestsLeft => StatusCode::TOO_MANY_REQUESTS,
            Code::TooManyPlans | Code::PlanClosed | Code::AlreadyRequested => StatusCode::CONFLICT,
            Code::EntitlementRequired => StatusCode::PAYMENT_REQUIRED,
            Code::Upstream => StatusCode::BAD_GATEWAY,
            Code::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug)]
pub struct AppError {
    pub code: Code,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl AppError {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn avec_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let mut corps = json!({ "error": self.code.as_str(), "message": self.message });
        if let Some(details) = self.details {
            corps["details"] = details;
        }
        (self.code.status(), Json(corps)).into_response()
    }
}

/// Une panne de base ou de cache ne doit jamais fuir au client : elle est
/// journalisée ici, et rendue comme une erreur interne.
impl From<sea_orm::DbErr> for AppError {
    fn from(erreur: sea_orm::DbErr) -> Self {
        tracing::error!(erreur = %erreur, "erreur de base de données");
        AppError::new(Code::Internal, "Une erreur interne est survenue.")
    }
}

impl From<redis::RedisError> for AppError {
    fn from(erreur: redis::RedisError) -> Self {
        tracing::error!(erreur = %erreur, "erreur du magasin clé-valeur");
        AppError::new(Code::Internal, "Une erreur interne est survenue.")
    }
}

pub fn non_autorise(message: &str) -> AppError {
    AppError::new(Code::Unauthorized, message)
}

pub fn introuvable(message: &str) -> AppError {
    AppError::new(Code::NotFound, message)
}

pub fn invalide(message: &str) -> AppError {
    AppError::new(Code::Validation, message)
}

pub fn trop_de_requetes(message: &str) -> AppError {
    AppError::new(Code::RateLimited, message)
}
