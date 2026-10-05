use std::fmt;

use ::axum::extract::{FromRequestParts, OptionalFromRequestParts};
use ::axum::response::{IntoResponse, Response};
use http::StatusCode;
use http::request::Parts;

use crate::inertia::Inertia;

/// Extracts the request's [`Inertia`] handle, installed by the
/// [`InertiaLayer`](super::InertiaLayer). Works in middleware too, which is
/// the place to share per-request props such as the current user.
impl<S: Send + Sync> FromRequestParts<S> for Inertia {
    type Rejection = MissingInertiaLayer;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Inertia>().cloned().ok_or(MissingInertiaLayer)
    }
}

/// Extracts the request's [`Inertia`] handle when the route has an
/// [`InertiaLayer`](super::InertiaLayer).
impl<S: Send + Sync> OptionalFromRequestParts<S> for Inertia {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts.extensions.get::<Inertia>().cloned())
    }
}

/// The [`Inertia`] extractor was used on a route without an [`InertiaLayer`](super::InertiaLayer).
#[derive(Debug, Clone, Copy)]
pub struct MissingInertiaLayer;

impl fmt::Display for MissingInertiaLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the Inertia extractor requires an `InertiaLayer` on the router")
    }
}

impl std::error::Error for MissingInertiaLayer {}

impl IntoResponse for MissingInertiaLayer {
    fn into_response(self) -> Response {
        tracing::error!("{self}");

        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}
