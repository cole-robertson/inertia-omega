//! The HTTP headers that make up the Inertia protocol.

use http::HeaderName;

/// Marks a request as an Inertia visit, and a response as an Inertia page.
pub const INERTIA: HeaderName = HeaderName::from_static("x-inertia");

/// The asset version the client is running, or the server's current version.
pub const VERSION: HeaderName = HeaderName::from_static("x-inertia-version");

/// The URL the client should perform a full page visit to.
pub const LOCATION: HeaderName = HeaderName::from_static("x-inertia-location");

/// The URL, including its fragment, the client should visit after a redirect.
pub const REDIRECT: HeaderName = HeaderName::from_static("x-inertia-redirect");

/// The component a partial reload is for.
pub const PARTIAL_COMPONENT: HeaderName = HeaderName::from_static("x-inertia-partial-component");

/// The comma-separated props a partial reload should include.
pub const PARTIAL_ONLY: HeaderName = HeaderName::from_static("x-inertia-partial-data");

/// The comma-separated props a partial reload should exclude.
pub const PARTIAL_EXCEPT: HeaderName = HeaderName::from_static("x-inertia-partial-except");

/// The comma-separated props whose client-side merge state should be reset.
pub const RESET: HeaderName = HeaderName::from_static("x-inertia-reset");

/// The error bag validation errors should be scoped to.
pub const ERROR_BAG: HeaderName = HeaderName::from_static("x-inertia-error-bag");

/// Whether an infinite scroll request wants to `append` or `prepend` its page.
pub const INFINITE_SCROLL_MERGE_INTENT: HeaderName = HeaderName::from_static("x-inertia-infinite-scroll-merge-intent");

/// The comma-separated once props the client already has.
pub const EXCEPT_ONCE_PROPS: HeaderName = HeaderName::from_static("x-inertia-except-once-props");

/// Set to `prefetch` when the client is prefetching a page.
pub const PURPOSE: HeaderName = HeaderName::from_static("purpose");
