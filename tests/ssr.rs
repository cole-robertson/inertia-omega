//! Server-side rendering through `HttpGateway`, against stand-in SSR servers.

#![cfg(feature = "ssr")]

use std::fmt::{self, Write};
use std::sync::{Arc, Mutex};

use axum::routing::post;
use axum::{Json, Router};
use http::{HeaderMap, Method, StatusCode};
use inertia::ssr::HttpGateway;
use inertia::testing::AssertablePage;
use inertia::{Config, Inertia, Page, Request, props};
use serde_json::json;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

/// Serve `router` on a free port, returning its URL.
async fn serve(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());

    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

    url
}

/// Collects the fields of this crate's events while it's the default subscriber.
#[derive(Clone, Default)]
struct Logs(Arc<Mutex<String>>);

impl Logs {
    fn contents(&self) -> String {
        self.0.lock().unwrap().clone()
    }
}

impl Subscriber for Logs {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.target().starts_with("inertia")
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut logs = self.0.lock().unwrap();
        event.record(&mut Fields(&mut logs));
        logs.push('\n');
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

struct Fields<'a>(&'a mut String);

impl Visit for Fields<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let _ = write!(self.0, "{}={:?} ", field.name(), value);
    }
}

/// Inertia's SSR server answers a failed render with a description of the
/// error that names the page's URL, and a URL may carry a secret, such as a
/// password reset token.
#[tokio::test]
async fn ssr_failures_are_logged_without_the_page() {
    let url = serve(Router::new().route(
        "/render",
        post(|Json(page): Json<Page>| async move {
            let error = json!({ "error": "window is not defined", "component": page.component, "url": page.url });

            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }),
    ))
    .await;

    let logs = Logs::default();
    let _guard = tracing::subscriber::set_default(logs.clone());

    let config = Config::new().ssr(HttpGateway::new().url(url));
    let uri = "/reset-password?token=secret".parse().unwrap();
    let response = Inertia::new(config, Request::new(Method::GET, &uri, HeaderMap::new()))
        .render("ResetPassword", props! {})
        .into_http()
        .await;

    AssertablePage::from_body(response.body()).component("ResetPassword");
    let logs = logs.contents();
    assert!(logs.contains("Inertia SSR failed"), "{logs}");
    assert!(logs.contains("500"), "{logs}");
    assert!(!logs.contains("secret"), "{logs}");
}
