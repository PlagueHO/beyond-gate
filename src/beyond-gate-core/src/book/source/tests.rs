//! Original HTML and actual loopback checks for the production book source path.

use super::*;
use crate::{
    auth::session::fixture as auth,
    library::fixture::{self, Scenario},
};

/// Invents a first-party-shaped root without using a real book or account snapshot.
fn root() -> Url {
    Url::parse("https://www.dndbeyond.com/sources/dnd/invented-1").expect("synthetic root")
}

/// Supplies one exact canonical location around original markup.
fn html(url: &Url, body: &str) -> Vec<u8> {
    let mut url = url.clone();
    url.set_fragment(None);
    format!("<html><head><link rel=\"canonical\" href=\"{url}\"></head><body>{body}</body></html>")
        .into_bytes()
}

/// Wraps invented anchors in the measured outline container.
fn navigation(body: &str) -> Result<BookOutline, BookError> {
    parse_outline(
        &html(
            &root(),
            &format!("<div class=\"compendium-toc-full-text\">{body}</div>"),
        ),
        &root(),
        "1",
    )
}

/// Selects an invented heading through its navigation URL.
fn section(body: &str) -> Result<String, BookError> {
    let url = root().join("invented-1/opening#first").expect("URL");
    parse_section(
        &html(
            &url,
            &format!(
                "<article class=\"p-article-content\" id=\"article\"><h1>Chapter</h1>{body}</article>"
            ),
        ),
        &url,
    )
}

/// Derives only a known synthetic ID; normal callers obtain IDs from an outline.
fn first_id(book: &str) -> String {
    format!(
        "s-{}",
        digest(&[
            book,
            &format!("{WEBSITE}/sources/dnd/invented-{book}/opening#first"),
            "0"
        ])
    )
}

/// Preserves duplicate navigation roles and canonicalizes only the same-book legacy alias.
#[test]
fn navigation_roles_aliases_and_parent_links_are_stable() {
    let outline = navigation(
        "<a href=\"/sources/invented-1/opening\">Chapter</a>\
        <a href=\"/sources/invented-1/opening#%66irst\">First</a>\
        <a href=\"/sources/dnd/invented-1/opening\">Chapter again</a>\
        <a href=\"/sources/dnd/invented-1/opening#first\">First again</a>",
    )
    .expect("outline");
    let entries = outline.entries();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[1].id(), first_id("1"));
    assert_ne!(entries[0].id(), entries[2].id());
    assert_ne!(entries[1].id(), entries[3].id());
    assert_eq!(entries[1].parent_id(), Some(entries[0].id()));
    assert_eq!(entries[3].parent_id(), Some(entries[2].id()));
    assert_eq!(entries[1].source_url(), entries[3].source_url());
    assert!(!format!("{outline:?}").contains("Chapter"));
    let nested = navigation("<ul><li><a href=\"/sources/invented-1/opening\">Chapter</a><ul>\
        <li><a href=\"/sources/invented-1/opening#first\">First</a><ul>\
        <li><a href=\"/sources/invented-1/opening#nested\">Nested</a></li></ul></li></ul></li></ul>").expect("nested hierarchy");
    assert_eq!(
        nested.entries()[2].parent_id(),
        Some(nested.entries()[1].id())
    );
}

/// Enforces the raw cap before auxiliary filtering or duplicate role handling.
#[test]
fn raw_navigation_bounds_include_auxiliary_links_and_duplicates() {
    for extra in [999, 1000] {
        let links = format!(
            "<a href=\"/sources/invented-1/opening\">Chapter</a>{}",
            "<a href=\"https://example.com/help\">Help</a>".repeat(extra)
        );
        assert_eq!(navigation(&links).is_ok(), extra == 999);
    }
    for count in [1000, 1001] {
        assert_eq!(
            navigation(&"<a href=\"/sources/invented-1/opening\">Chapter</a>".repeat(count))
                .is_ok(),
            count == 1000
        );
    }
    assert!(matches!(
        navigation("<a href=\"/sources/invented-1/opening#first\">Orphan</a>"),
        Err(BookError::InvalidOutline)
    ));
}

/// Rejects foreign books, encoded routes, redirects-as-links, controls, and unsafe schemes.
#[test]
fn unsafe_links_and_fragments_never_become_fetch_targets() {
    for href in [
        "http://www.dndbeyond.com/sources/dnd/invented-1/opening",
        "https://foreign.invalid/sources/dnd/invented-1/opening",
        "https://user@www.dndbeyond.com/sources/dnd/invented-1/opening",
        "/sources/dnd/another/opening",
        "/sources/dnd/invented-1/opening?key=value",
        "/sources/dnd/invented-1/%2e/opening",
        "/sources/dnd/invented-1/../opening",
        "/sources/dnd/invented-1/opening#bad%",
        "/sources/dnd/invented-1/opening#%ff",
        "/sources/dnd/invented-1/opening#%00",
        "javascript:alert(1)",
        "",
    ] {
        assert!(target(&root(), href).is_err(), "{href}");
    }
    assert_eq!(fragment("%E7%95%8C").expect("Unicode fragment"), "\u{754c}");
    assert_eq!(fragment("%2520").expect("decode once"), "%20");
}

/// Rejects resolved marketplace links as purchase markers instead of returning chapter text.
#[test]
fn marketplace_purchase_links_return_access_unknown() {
    for href in [
        "/marketplace",
        "/marketplace/items/invented",
        "../../../marketplace/items/invented",
        "https://www.dndbeyond.com/marketplace/items/invented",
        "//www.dndbeyond.com/marketplace/items/invented",
        "https://marketplace.dndbeyond.com/items/invented",
        "//marketplace.dndbeyond.com/items/invented",
    ] {
        let body = format!("<h2 id=\"first\">Opening</h2><a href=\"{href}\">Purchase</a>");
        assert!(
            matches!(section(&body), Err(BookError::AccessUnknown)),
            "{href}"
        );
    }
}

/// Keeps ordinary links whose paths, queries, or similar host names mention marketplace URLs.
#[test]
fn marketplace_near_miss_links_preserve_section_text() {
    for href in [
        "/marketplace-guide",
        "/sources/dnd/invented-1/marketplace",
        "https://www.dndbeyond.com/marketplace-guide",
        "https://www.dndbeyond.com/help?next=/marketplace/items",
        "https://example.invalid/marketplace/items",
        "https://example.invalid/marketplace.dndbeyond.com/items",
        "https://example.invalid/help?next=https://marketplace.dndbeyond.com/items",
        "https://marketplace.dndbeyond.com.example.invalid/items",
        "https://marketplace.dndbeyond.com@example.invalid/items",
        "#marketplace.dndbeyond.com",
    ] {
        let body = format!("<h2 id=\"first\">Opening</h2><a href=\"{href}\">Useful link</a>");
        assert_eq!(
            section(&body).expect(href),
            "Opening\nUseful link\n",
            "{href}"
        );
    }
}

/// Preserves useful text, list order, table cells, and sidebars without adjacent topics.
#[test]
fn selected_text_preserves_structure_without_controls_or_adjacent_text() {
    let text = section(
        "<h2 id=\"first\">Opening</h2><p>Hello <em>world</em>.</p>\
        <ol start=\"3\"><li>Three</li><li value=\"7\">Seven</li></ol>\
        <aside><h3>Sidebar</h3><p>Keep this note.</p></aside>\
        <table><tr><th>Label</th><th>Value</th></tr><tr><td>One</td><td>Two</td></tr></table>\
        <pre>keep  spaces\n  indentation</pre><p>A<br>B</p>\
        <script>private script</script><nav>private navigation</nav>\
        <p hidden>private warning</p><p style=\"display: none\">private style</p>\
        <h2 id=\"second\">Adjacent</h2><p>private adjacent</p>",
    )
    .expect("selected text");
    for preserved in [
        "Hello world.",
        "3. Three",
        "7. Seven",
        "Sidebar\nKeep this note.",
        "Label\tValue\nOne\tTwo",
        "keep  spaces\n  indentation",
        "A\nB",
    ] {
        assert!(text.contains(preserved), "{text:?}");
    }
    assert!(!text.contains("private"));
    assert!(!text.contains("Adjacent"));
    assert!(!text.contains("Chapter"));
}

/// Applies the validation depth cap to text nodes at the exact renderer boundary.
#[test]
fn validation_bounds_accept_depth_256_and_reject_depth_257() {
    for depth in [256, 257] {
        let body = format!(
            "<h2 id=\"first\">One</h2>{}text{}",
            "<div>".repeat(depth - 1),
            "</div>".repeat(depth - 1)
        );
        if depth == 256 {
            assert_eq!(section(&body).expect("bounded depth"), "One\ntext\n");
        } else {
            assert!(matches!(section(&body), Err(BookError::LimitExceeded)));
        }
    }
}

/// Counts the article and every element, including content outside the selected range.
#[test]
fn validation_bounds_accept_100000_elements_and_reject_100001() {
    for count in [100_000, 100_001] {
        let body = format!(
            "<h2 id=\"first\">One</h2><h2>Next</h2>{}",
            "<br>".repeat(count - 4)
        );
        if count == 100_000 {
            assert_eq!(section(&body).expect("bounded element count"), "One\n");
        } else {
            assert!(matches!(section(&body), Err(BookError::LimitExceeded)));
        }
    }
}

/// Caps outer wrappers so heading visibility cannot repeatedly walk unbounded ancestors.
#[test]
fn validation_bounds_include_outer_article_ancestors() {
    let url = root().join("opening#first").expect("synthetic URL");
    for wrappers in [253, 254] {
        let body = format!(
            "{}<article class=\"p-article-content\"><h1>Chapter</h1>\
            <h2 id=\"first\">One</h2></article>{}",
            "<div>".repeat(wrappers),
            "</div>".repeat(wrappers)
        );
        let result = parse_section(&html(&url, &body), &url);
        if wrappers == 253 {
            assert_eq!(result.expect("256 outer ancestors"), "One\n");
        } else {
            assert!(matches!(result, Err(BookError::LimitExceeded)));
        }
    }
}

/// Rejects a large, balanced deep article before hidden headings can start ancestor walks.
#[test]
fn validation_bounds_reject_balanced_deep_headings_before_visibility() {
    let body = format!(
        "{}{}",
        "<div><h2 hidden>Hidden</h2>".repeat(4_000),
        "</div>".repeat(4_000)
    );
    assert!(matches!(section(&body), Err(BookError::LimitExceeded)));
}

/// Preserves visibility and selected descendant handling within the validation bounds.
#[test]
fn validation_bounds_preserve_hidden_ancestors_and_heading_selection() {
    assert_eq!(
        section("<div><h2><span id=\"first\">One</span></h2><p>Text</p></div>")
            .expect("visible selected descendant"),
        "One\nText\n"
    );
    for attribute in ["hidden", "aria-hidden=\"true\"", "style=\"display: none\""] {
        let body = format!("<div {attribute}><h2 id=\"first\">Hidden</h2></div>");
        assert!(matches!(section(&body), Err(BookError::AccessUnknown)));
    }
    let url = root().join("opening#first").expect("synthetic URL");
    assert!(matches!(
        parse_section(
            &html(
                &url,
                "<div hidden><article class=\"p-article-content\">\
                <h1>Chapter</h1><h2 id=\"first\">Hidden</h2></article></div>"
            ),
            &url
        ),
        Err(BookError::AccessUnknown)
    ));
}

/// Fails closed for ambiguous IDs, missing targets, hidden headings, and unsupported targets.
#[test]
fn section_structure_and_target_failures_are_explicit() {
    assert_eq!(
        section("<h2 id=\"first\"></h2><h2 id=\"second\">Next topic</h2>")
            .expect("valid empty section"),
        ""
    );
    for body in [
        "<h2 id=\"first\">One</h2><h3 id=\"first\">Duplicate</h3>",
        "<h2 id=\"first\" hidden>Hidden</h2>",
        "<h2 id=\"other\">Missing</h2>",
        "<p id=\"first\">Not a heading</p>",
        "<h2 id=\"first\">One</h2><a href=\"https://marketplace.dndbeyond.com/item\">Purchase</a>",
        "<h2 id=\"first\">One</h2><p>Bad \u{1b} text</p>",
    ] {
        assert!(section(body).is_err());
    }
    let deep = format!(
        "<h2 id=\"first\">One</h2>{}text{}",
        "<div>".repeat(257),
        "</div>".repeat(257)
    );
    assert!(matches!(section(&deep), Err(BookError::LimitExceeded)));
    let wrong = html(
        &root().join("invented-1/other").expect("URL"),
        "<div></div>",
    );
    assert!(matches!(
        document(&wrong, &root()),
        Err(BookError::MalformedSource)
    ));
    assert!(matches!(
        document(
            &html(&root(), "<form id=\"challenge-form\"></form>"),
            &root()
        ),
        Err(BookError::AccessUnknown)
    ));
}

/// Exercises the actual metadata, cookie HTML, canonical parsing, and explicit continuation path.
#[tokio::test]
async fn fresh_reads_repeat_access_and_navigation_without_exchanging_for_html() {
    let session = fixture::session(Scenario::Normal).expect("local services");
    let outline = outline(&session, "1").await.expect("fresh outline");
    assert_eq!(fixture::request_count(&session), 4);
    assert_eq!(auth::request_count(&session), 1);
    let id = outline.entries()[1].id();
    let request = SectionRequest {
        book_id: "1",
        section_id: id,
        continuation: None,
    };
    let first = read(&session, request).await.expect("first chunk");
    assert!(first.text().len() <= 4096);
    assert_eq!(fixture::request_count(&session), 9);
    let cursor = first.next_cursor().expect("explicit continuation");
    let next = read(
        &session,
        SectionRequest {
            continuation: Some(cursor),
            ..request
        },
    )
    .await
    .expect("next chunk");
    assert!(next.text().len() <= 4096);
    assert_eq!(fixture::request_count(&session), 14);
    assert_eq!(auth::request_count(&session), 1);
    assert_eq!(next.source_url(), first.source_url());
    let mut attacker = cursor.clone();
    attacker.source_url =
        Url::parse("https://127.0.0.1:1/private-token-target").expect("untrusted URL");
    assert!(matches!(
        read(
            &session,
            SectionRequest {
                continuation: Some(&attacker),
                ..request
            }
        )
        .await,
        Err(BookError::StaleContinuation)
    ));
    assert_eq!(fixture::request_count(&session), 19);
    let changed = fixture::session(Scenario::BookChanged).expect("changed source");
    assert!(matches!(
        read(
            &changed,
            SectionRequest {
                continuation: Some(cursor),
                ..request
            }
        )
        .await,
        Err(BookError::StaleContinuation)
    ));
    let moved = fixture::session(Scenario::BookMoved).expect("moved source");
    assert!(matches!(
        read(
            &moved,
            SectionRequest {
                continuation: Some(cursor),
                ..request
            }
        )
        .await,
        Err(BookError::StaleContinuation)
    ));
    assert_eq!(fixture::request_count(&moved), 4);
}

/// A former sharing observation and token cannot authorize a fresh unknown account result.
#[tokio::test]
async fn sharing_loss_stops_before_outline_or_chapter_loading() {
    let session = fixture::session(Scenario::Normal).expect("local services");
    let id = first_id("2");
    let request = SectionRequest {
        book_id: "2",
        section_id: &id,
        continuation: None,
    };
    let first = read(&session, request).await.expect("shared read");
    let lost = fixture::session(Scenario::SharingLost).expect("new account evidence");
    assert!(matches!(
        read(
            &lost,
            SectionRequest {
                continuation: first.next_cursor(),
                ..request
            }
        )
        .await,
        Err(BookError::AccessUnknown)
    ));
    assert_eq!(fixture::request_count(&lost), 3);
}

/// Every terminal upstream failure returns no content and never retries a page.
#[tokio::test]
async fn source_failures_stop_at_the_first_failed_page() {
    for (scenario, requests) in [
        (Scenario::BookRedirect, 4),
        (Scenario::BookRejected, 5),
        (Scenario::BookDenied, 5),
        (Scenario::BookPreview, 5),
        (Scenario::BookMalformed, 5),
        (Scenario::BookOversized, 4),
        (Scenario::BookWrongMedia, 4),
    ] {
        let session = fixture::session(scenario).expect("local services");
        let id = first_id("1");
        let error = read(
            &session,
            SectionRequest {
                book_id: "1",
                section_id: &id,
                continuation: None,
            },
        )
        .await
        .expect_err("fail closed");
        assert!(!matches!(error, BookError::AccessDenied));
        let text = format!("{error} {error:?}");
        for private in [
            "invented-session",
            "lantern",
            "<article",
            "first",
            "CobaltSession",
        ] {
            assert!(!text.contains(private), "{text}");
        }
        assert_eq!(fixture::request_count(&session), requests);
        assert_eq!(auth::request_count(&session), 1);
        if matches!(scenario, Scenario::BookRejected) {
            assert!(
                read(
                    &session,
                    SectionRequest {
                        book_id: "1",
                        section_id: &id,
                        continuation: None
                    }
                )
                .await
                .is_err()
            );
            assert_eq!(fixture::request_count(&session), 6);
            assert_eq!(auth::request_count(&session), 1);
        }
    }
}

/// Dropping an in-flight read does not follow another page or start a retry.
#[tokio::test]
async fn cancellation_stops_the_selected_request_without_background_reads() {
    let session = fixture::session(Scenario::BookWaiting).expect("local services");
    let id = first_id("1");
    let mut reading = Box::pin(read(
        &session,
        SectionRequest {
            book_id: "1",
            section_id: &id,
            continuation: None,
        },
    ));
    tokio::select! {
        result = &mut reading => panic!("read completed before cancellation: {result:?}"),
        result = tokio::time::timeout(Duration::from_secs(2), async {
            while fixture::request_count(&session) < 5 { tokio::time::sleep(Duration::from_millis(1)).await; }
        }) => result.expect("selected request started"),
    }
    drop(reading);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(fixture::request_count(&session), 5);
    outline(&session, "1").await.expect("owner remains usable");
    assert_eq!(fixture::request_count(&session), 9);
}
