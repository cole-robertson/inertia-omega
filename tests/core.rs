//! The framework-agnostic core, exercised without a web framework.

use std::time::{Duration, Instant};

use http::{HeaderMap, HeaderName, Method};
use inertia::session::ArraySession;
use inertia::testing::AssertablePage;
use inertia::{Config, Inertia, Page, Paginator, Props, Request, ValidationErrors, props};
use serde::Serialize;
use serde_json::json;

fn config() -> Config {
    Config::new()
        .version("1")
        .share(|_| props! { "appName" => "Demo", "auth.check" => true })
}

fn request(method: Method, headers: &[(&'static str, &'static str)]) -> Request {
    let headers: HeaderMap = headers
        .iter()
        .map(|(name, value)| (HeaderName::from_static(name), value.parse().unwrap()))
        .collect();

    Request::new(method, &"/users?page=1".parse().unwrap(), headers)
}

/// An Inertia visit with the given extra headers.
fn visit(headers: &[(&'static str, &'static str)]) -> Inertia {
    let mut all = vec![("x-inertia", "true"), ("x-inertia-version", "1")];
    all.extend_from_slice(headers);

    Inertia::new(config(), request(Method::GET, &all))
}

/// A partial reload of `Users` with the given extra headers.
fn reload(headers: &[(&'static str, &'static str)]) -> Inertia {
    let mut all = vec![("x-inertia-partial-component", "Users")];
    all.extend_from_slice(headers);

    visit(&all)
}

async fn resolve(response: inertia::Response) -> Page {
    response.into_page().await.unwrap()
}

fn users_props() -> Props {
    props! {
        "users" => ["Taylor", "Jess"],
        "auth.user.id" => 1,
        "auth.user.name" => "Taylor",
        "lazy" => inertia::lazy(|| async { "computed" }),
        "nested" => props! { "a" => 1, "b" => inertia::optional(|| async { 2 }) },
        "literal" => json!({ "x": 1, "y": { "z": 2 } }),
        "optional" => inertia::optional(|| async { "optional" }),
        "always" => inertia::always("always"),
    }
}

#[tokio::test]
async fn resolves_shared_and_page_props() {
    let page = resolve(visit(&[]).render("Users", users_props())).await;

    assert_eq!(
        serde_json::to_value(&page.props).unwrap(),
        json!({
            "errors": {},
            "appName": "Demo",
            "auth": { "check": true, "user": { "id": 1, "name": "Taylor" } },
            "users": ["Taylor", "Jess"],
            "lazy": "computed",
            "nested": { "a": 1 },
            "literal": { "x": 1, "y": { "z": 2 } },
            "always": "always",
        })
    );
    assert_eq!(page.url, "/users?page=1");
    assert_eq!(page.version, "1");
    assert_eq!(page.metadata.shared_props, ["errors", "appName", "auth"]);
}

#[tokio::test]
async fn renders_serializable_structs() {
    #[derive(Serialize)]
    struct UsersProps {
        users: Vec<&'static str>,
    }

    let page = resolve(
        visit(&[])
            .render("Users", UsersProps { users: vec!["Taylor"] })
            .with("count", 1),
    )
    .await;

    assert_eq!(page.props["users"], json!(["Taylor"]));
    assert_eq!(page.props["count"], 1);

    let error = visit(&[]).render("Users", [1, 2]).into_page().await.unwrap_err();
    assert!(error.to_string().contains("must serialize to a JSON object"));
}

#[tokio::test]
async fn partial_reloads_only_include_the_requested_props() {
    let page = resolve(reload(&[("x-inertia-partial-data", "users,optional")]).render("Users", users_props())).await;
    assert_eq!(
        serde_json::to_value(&page.props).unwrap(),
        json!({ "errors": {}, "users": ["Taylor", "Jess"], "optional": "optional", "always": "always" })
    );

    let page = resolve(
        reload(&[("x-inertia-partial-data", "auth.user.name,nested.b,literal.y")]).render("Users", users_props()),
    )
    .await;
    assert_eq!(
        serde_json::to_value(&page.props).unwrap(),
        json!({
            "errors": {},
            "auth": { "user": { "name": "Taylor" } },
            "nested": { "b": 2 },
            "literal": { "y": { "z": 2 } },
            "always": "always",
        })
    );
}

#[tokio::test]
async fn partial_reloads_exclude_the_excepted_props() {
    let page =
        resolve(reload(&[("x-inertia-partial-except", "auth,literal.x,errors")]).render("Users", users_props())).await;
    let page = AssertablePage::from_body(&serde_json::to_string(&page).unwrap());

    page.missing("auth")
        .has("errors")
        .equals("literal", json!({ "y": { "z": 2 } }))
        .equals("optional", "optional");
}

#[tokio::test]
async fn partial_reloads_of_another_component_are_full_visits() {
    let page = resolve(reload(&[("x-inertia-partial-data", "users")]).render("Dashboard", users_props())).await;

    assert_eq!(page.props["appName"], "Demo");
    assert!(page.props.get("optional").is_none());
}

#[tokio::test]
async fn deferred_props_are_announced_then_loaded_by_group() {
    let props = || {
        props! {
            "users" => inertia::defer(|| async { ["Taylor"] }),
            "stats" => inertia::defer(|| async { 42 }).group("stats"),
            "broken" => inertia::try_lazy(|| async { Err::<u8, _>("boom") }).deferred().rescue(),
        }
    };

    let page = resolve(visit(&[]).render("Users", props())).await;
    assert!(page.props.get("users").is_none());
    assert_eq!(page.metadata.deferred_props["default"], ["users", "broken"]);
    assert_eq!(page.metadata.deferred_props["stats"], ["stats"]);

    let page = resolve(reload(&[("x-inertia-partial-data", "users,broken")]).render("Users", props())).await;
    assert_eq!(
        serde_json::to_value(&page.props).unwrap(),
        json!({ "errors": {}, "users": ["Taylor"] })
    );
    assert_eq!(page.metadata.rescued_props, ["broken"]);
    assert!(page.metadata.deferred_props.is_empty());
}

#[tokio::test]
async fn unrescued_failures_fail_the_render() {
    let response = visit(&[]).render(
        "Users",
        props! { "broken" => inertia::try_lazy(|| async { Err::<u8, _>("boom") }) },
    );

    assert_eq!(response.into_page().await.unwrap_err().to_string(), "boom");
}

#[tokio::test]
async fn merge_props_describe_how_to_merge() {
    let props = || {
        props! {
            "append" => inertia::merge([1, 2]),
            "prepend" => inertia::merge([1]).prepend(),
            "deep" => inertia::deep_merge(json!({ "a": 1 })),
            "matched" => inertia::merge(json!([{ "id": 1 }])).match_on("id"),
            "paths" => inertia::merge(json!({ "data": [], "meta": [] })).append_at("data").match_on("data.id").prepend_at("meta"),
            "deferredMerge" => inertia::defer(|| async { [1] }).merge(),
        }
    };

    let metadata = resolve(visit(&[]).render("Users", props())).await.metadata;
    assert_eq!(
        metadata.merge_props,
        ["append", "matched", "paths.data", "deferredMerge"]
    );
    assert_eq!(metadata.prepend_props, ["prepend", "paths.meta"]);
    assert_eq!(metadata.deep_merge_props, ["deep"]);
    assert_eq!(metadata.match_props_on, ["matched.id", "paths.data.id"]);

    let reset = reload(&[
        ("x-inertia-partial-data", "append,prepend"),
        ("x-inertia-reset", "prepend"),
    ]);
    let metadata = resolve(reset.render("Users", props())).await.metadata;
    assert_eq!(metadata.merge_props, ["append"]);
    assert!(metadata.prepend_props.is_empty());
}

#[tokio::test]
async fn once_props_are_skipped_when_the_client_has_them() {
    let props = || {
        props! {
            "plans" => inertia::once(|| async { ["basic"] }),
            "countries" => inertia::once(|| async { ["NL"] }).once_as("geo").until(Duration::from_secs(60)),
            "fresh" => inertia::once(|| async { 1 }).fresh(),
        }
    };

    let page = resolve(visit(&[]).render("Users", props())).await;
    assert_eq!(page.props["plans"], json!(["basic"]));
    assert_eq!(page.metadata.once_props["plans"].prop, "plans");
    assert_eq!(page.metadata.once_props["plans"].expires_at, None);
    assert_eq!(page.metadata.once_props["geo"].prop, "countries");
    assert!(page.metadata.once_props["geo"].expires_at.is_some());

    let page = resolve(visit(&[("x-inertia-except-once-props", "plans,geo,fresh")]).render("Users", props())).await;
    assert!(page.props.get("plans").is_none());
    assert!(page.props.get("countries").is_none());
    assert_eq!(page.props["fresh"], 1);
    assert!(
        page.metadata.once_props.contains_key("plans"),
        "the client keeps remembering them"
    );
}

#[tokio::test]
async fn scroll_props_carry_pagination_metadata() {
    let props = || {
        props! {
            "users" => inertia::scroll(Paginator::from_items(1..=25, 10, 2)),
            "later" => inertia::scroll_with(|| async { Paginator::from_items(1..=5, 10, 1) }).deferred(),
        }
    };

    let page = resolve(visit(&[]).render("Users", props())).await;
    assert_eq!(
        page.props["users"]["data"],
        json!([11, 12, 13, 14, 15, 16, 17, 18, 19, 20])
    );
    assert_eq!(
        serde_json::to_value(&page.metadata.scroll_props["users"]).unwrap(),
        json!({ "pageName": "page", "previousPage": 1, "nextPage": 3, "currentPage": 2, "reset": false })
    );
    assert_eq!(page.metadata.merge_props, ["users.data", "later"]);
    assert_eq!(page.metadata.deferred_props["default"], ["later"]);

    let prepend = reload(&[
        ("x-inertia-partial-data", "later"),
        ("x-inertia-infinite-scroll-merge-intent", "prepend"),
    ]);
    let page = resolve(prepend.render("Users", props())).await;
    assert_eq!(page.metadata.prepend_props, ["later.data"]);
    assert_eq!(page.metadata.scroll_props["later"].metadata.next_page, None);
}

#[tokio::test]
async fn sibling_callbacks_resolve_concurrently() {
    let slow = |value: u8| {
        inertia::lazy(move || async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            value
        })
    };

    let started = Instant::now();
    let page = resolve(visit(&[]).render("Users", props! { "a" => slow(1), "b" => slow(2), "c" => slow(3) })).await;

    assert_eq!(page.props["c"], 3);
    assert!(
        started.elapsed() < Duration::from_millis(350),
        "took {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn big_integers_can_be_preserved() {
    let page = resolve(
        visit(&[])
            .render("Users", props! { "id" => u64::MAX, "small" => 1 })
            .preserve_big_integers(true),
    )
    .await;

    assert_eq!(page.props["id"], json!({ "$bigint": "18446744073709551615" }));
    assert_eq!(page.props["small"], 1);
    assert!(page.preserve_big_integers);
}

#[tokio::test]
async fn flash_data_and_errors_are_delivered_to_the_next_render() {
    let session = ArraySession::new();
    let post = Inertia::with_session(
        config(),
        request(Method::POST, &[("x-inertia", "true")]),
        session.clone(),
    );

    post.flash("message", "Saved!")
        .with_errors(ValidationErrors::new().with("name", "Required."));
    post.clear_history();
    post.commit().await;

    let get = || {
        Inertia::with_session(
            config(),
            request(Method::GET, &[("x-inertia", "true")]),
            session.clone(),
        )
    };

    let page = resolve(get().render("Users", ())).await;
    assert_eq!(page.props["errors"], json!({ "name": "Required." }));
    assert_eq!(page.flash["message"], "Saved!");
    assert!(page.clear_history);

    let page = resolve(get().render("Users", ())).await;
    assert_eq!(page.props["errors"], json!({}));
    assert!(page.flash.is_empty());
    assert!(!page.clear_history);
}

#[tokio::test]
async fn flash_data_reaches_a_render_in_the_same_request() {
    let inertia = visit(&[]);
    inertia.flash("toast", "Hi").encrypt_history(true);

    let page = resolve(inertia.render("Users", ()).flash("other", 1)).await;

    assert_eq!(
        serde_json::to_value(&page.flash).unwrap(),
        json!({ "toast": "Hi", "other": 1 })
    );
    assert!(page.encrypt_history);
}

#[tokio::test]
async fn first_visits_render_the_root_view() {
    let config =
        config().root_view(|view: &inertia::View<'_>| format!("<title>{}</title>{}", view.page.component, view.body));
    let inertia = Inertia::new(config, request(Method::GET, &[]));

    let response = inertia
        .render("Users", props! { "html" => "</script><b>&" })
        .into_http()
        .await;
    let body = response.body();

    assert_eq!(response.headers()["content-type"], "text/html; charset=utf-8");
    assert!(body.starts_with("<title>Users</title>"));
    assert!(body.contains(r#"<div id="app"></div>"#));
    assert!(!body.contains("</script><b>"));
    AssertablePage::from_body(body).equals("html", "</script><b>&");
}

#[cfg(feature = "validator")]
#[test]
fn converts_validator_errors() {
    use validator::Validate;

    #[derive(Validate)]
    struct NewUser {
        #[validate(length(min = 3, message = "The name must be at least 3 characters."))]
        name: String,
        #[validate(email)]
        email: String,
    }

    let errors: ValidationErrors = NewUser {
        name: "Al".into(),
        email: "nope".into(),
    }
    .validate()
    .unwrap_err()
    .into();

    assert_eq!(errors.first("name"), Some("The name must be at least 3 characters."));
    assert_eq!(errors.first("email"), Some("email"));
}

#[cfg(feature = "garde")]
#[test]
fn converts_garde_reports() {
    use garde::Validate;

    #[derive(Validate)]
    struct NewUser {
        #[garde(length(min = 3))]
        name: String,
    }

    let errors: ValidationErrors = NewUser { name: "Al".into() }.validate().unwrap_err().into();

    assert!(errors.has("name"));
}
