//! Synthetic metadata and page contracts; no live account or catalog fixtures.

use super::*;
use std::{cell::Cell, error::Error};

/// Creates invented metadata with a single supplied observation.
fn record(id: &str, evidence: AccessEvidence) -> BookRecord {
    BookRecord {
        id: id.to_owned(),
        title: format!("Invented volume {id}"),
        source_url: format!("https://example.com/books/{id}"),
        evidence,
    }
}

/// Builds an observation for one invented sharing context.
fn shared(context: &str, state: AccessState) -> AccessEvidence {
    AccessEvidence::Shared {
        context_id: context.to_owned(),
        state,
    }
}

/// Supplies a normalized page without interpreting opaque continuations.
fn page(cursor: Option<&str>, entries: Vec<BookRecord>, next: Option<&str>) -> LibraryPage {
    LibraryPage {
        cursor: cursor.map(str::to_owned),
        entries,
        next_cursor: next.map(str::to_owned),
    }
}

/// Exercises the public assembly operation with a complete single page.
fn assemble(entries: Vec<BookRecord>) -> Result<LibrarySnapshot, LibraryError> {
    assemble_library([Ok(page(None, entries, None))])
}

/// Checks evidence-derived categories and the returned metadata contract.
#[test]
fn access_categories_require_explicit_evidence() {
    for (evidence, expected) in [
        (AccessEvidence::CatalogOnly, BookAccess::Unknown),
        (
            AccessEvidence::Purchased {
                state: AccessState::Granted,
            },
            BookAccess::Purchased,
        ),
        (
            AccessEvidence::Purchased {
                state: AccessState::Unavailable,
            },
            BookAccess::Unavailable,
        ),
        (
            AccessEvidence::Purchased {
                state: AccessState::Unknown,
            },
            BookAccess::Unknown,
        ),
        (shared("group-a", AccessState::Granted), BookAccess::Shared),
        (
            shared("group-a", AccessState::Unavailable),
            BookAccess::Unavailable,
        ),
        (shared("group-a", AccessState::Unknown), BookAccess::Unknown),
    ] {
        let snapshot =
            assemble(vec![record("volume-a", evidence.clone())]).expect("valid metadata");
        let books = snapshot.books();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].id(), "volume-a");
        assert_eq!(books[0].title(), "Invented volume volume-a");
        assert_eq!(
            books[0].source_url().as_str(),
            "https://example.com/books/volume-a"
        );
        assert_eq!(books[0].evidence(), &[evidence]);
        assert_eq!(books[0].access(), expected);
    }
}

/// Preserves independent grants and never promotes unknown evidence to access.
#[test]
fn independent_contexts_survive_revocation() {
    let purchased = AccessEvidence::Purchased {
        state: AccessState::Granted,
    };
    for (evidence, expected) in [
        (
            vec![purchased.clone(), shared("a", AccessState::Granted)],
            BookAccess::PurchasedAndShared,
        ),
        (
            vec![purchased, shared("a", AccessState::Unavailable)],
            BookAccess::Purchased,
        ),
        (
            vec![
                shared("a", AccessState::Granted),
                shared("b", AccessState::Unavailable),
            ],
            BookAccess::Shared,
        ),
        (
            vec![
                shared("a", AccessState::Unavailable),
                shared("b", AccessState::Unavailable),
            ],
            BookAccess::Unavailable,
        ),
        (
            vec![
                shared("a", AccessState::Unavailable),
                shared("b", AccessState::Unknown),
            ],
            BookAccess::Unknown,
        ),
        (
            vec![
                shared("a", AccessState::Unavailable),
                AccessEvidence::CatalogOnly,
            ],
            BookAccess::Unknown,
        ),
    ] {
        let snapshot = assemble(
            evidence
                .into_iter()
                .map(|evidence| record("one", evidence))
                .collect(),
        )
        .expect("independent observations");
        assert_eq!(snapshot.books()[0].access(), expected);
    }
}

/// Starts a new result instead of retaining a prior purchase or sharing grant.
#[test]
fn fresh_discovery_does_not_inherit_grants() {
    let original = assemble(vec![record("one", shared("a", AccessState::Granted))])
        .expect("original snapshot");
    for (entries, expected) in [
        (
            vec![record("one", shared("a", AccessState::Unavailable))],
            Some(BookAccess::Unavailable),
        ),
        (
            vec![record("one", AccessEvidence::CatalogOnly)],
            Some(BookAccess::Unknown),
        ),
        (vec![], None),
    ] {
        let fresh = assemble(entries).expect("fresh complete snapshot");
        assert_eq!(fresh.books().first().map(Book::access), expected);
    }
    assert_eq!(original.books()[0].access(), BookAccess::Shared);
}

/// Combines duplicate observations across pages without changing result order.
#[test]
fn duplicates_merge_with_stable_book_and_evidence_order() {
    let catalog = record("b", AccessEvidence::CatalogOnly);
    let purchase = record(
        "b",
        AccessEvidence::Purchased {
            state: AccessState::Granted,
        },
    );
    let share = record("b", shared("a", AccessState::Granted));
    let result = assemble_library([
        Ok(page(
            None,
            vec![
                catalog.clone(),
                catalog,
                record("a", shared("b", AccessState::Unavailable)),
                purchase.clone(),
            ],
            Some("second"),
        )),
        Ok(page(
            Some("second"),
            vec![purchase.clone(), share.clone()],
            None,
        )),
    ])
    .expect("complete duplicate library");
    assert_eq!(
        result.books().iter().map(Book::id).collect::<Vec<_>>(),
        ["b", "a"]
    );
    assert_eq!(
        result.books()[0].evidence(),
        &[
            AccessEvidence::CatalogOnly,
            purchase.evidence,
            share.evidence
        ]
    );
    assert_eq!(result.books()[0].access(), BookAccess::PurchasedAndShared);
}

/// Rejects conflicting states in either order for the same evidence context.
#[test]
fn same_context_conflicts_never_select_the_last_record() {
    let states = [
        AccessState::Granted,
        AccessState::Unavailable,
        AccessState::Unknown,
    ];
    for first in states {
        for second in states {
            for observations in [
                [
                    AccessEvidence::Purchased { state: first },
                    AccessEvidence::Purchased { state: second },
                ],
                [shared("group", first), shared("group", second)],
            ] {
                let [left, right] = observations;
                let result = assemble_library([
                    Ok(page(None, vec![record("one", left)], Some("next"))),
                    Ok(page(Some("next"), vec![record("one", right)], None)),
                ]);
                if first == second {
                    assert_eq!(
                        result.expect("same observation").books()[0]
                            .evidence()
                            .len(),
                        1
                    );
                } else {
                    assert!(matches!(result, Err(LibraryError::ConflictingEvidence)));
                }
            }
        }
    }
}

/// Rejects differing metadata for one identifier regardless of arrival order.
#[test]
fn conflicting_titles_and_sources_fail_the_whole_discovery() {
    let original = record("one", AccessEvidence::CatalogOnly);
    let mut title = original.clone();
    title.title = "Another invented title".to_owned();
    let mut url = original.clone();
    url.source_url = "https://example.org/another".to_owned();
    for changed in [title, url] {
        for records in [
            vec![original.clone(), changed.clone()],
            vec![changed.clone(), original.clone()],
        ] {
            assert!(matches!(
                assemble(records),
                Err(LibraryError::ConflictingMetadata)
            ));
        }
    }
}

/// Rejects missing or unsafe metadata before a snapshot can be returned.
#[test]
fn invalid_normalized_metadata_is_an_explicit_failure() {
    for invalid in ["", " ", " padded", "padded ", "control\nvalue", "\0"] {
        let mut id = record("one", AccessEvidence::CatalogOnly);
        id.id = invalid.to_owned();
        let mut title = record("one", AccessEvidence::CatalogOnly);
        title.title = invalid.to_owned();
        for record in [id, title] {
            assert!(matches!(
                assemble(vec![record]),
                Err(LibraryError::InvalidMetadata)
            ));
        }
        assert!(matches!(
            assemble(vec![record("one", shared(invalid, AccessState::Granted))]),
            Err(LibraryError::InvalidEvidence)
        ));
    }
    for invalid in [
        "",
        "not a URL",
        "/relative",
        "http://example.com/book",
        "file:///book",
        "https://",
        "https://user@example.com/book",
        "https://user:password@example.com/book",
        "https://example.com/book#section",
        " https://example.com/book",
        "https://example.com/book ",
        "https://example.com/bo\nok",
    ] {
        let mut record = record("one", AccessEvidence::CatalogOnly);
        record.source_url = invalid.to_owned();
        assert!(matches!(
            assemble(vec![record]),
            Err(LibraryError::InvalidMetadata)
        ));
    }
}

/// Distinguishes an explicit empty library from absent or incomplete pages.
#[test]
fn only_terminal_input_can_return_an_empty_snapshot() {
    assert!(
        assemble(vec![])
            .expect("terminal empty page")
            .books()
            .is_empty()
    );
    assert!(matches!(
        assemble_library([]),
        Err(LibraryError::IncompleteDiscovery)
    ));
    assert!(matches!(
        assemble_library([Ok(page(None, vec![], Some("missing")))]),
        Err(LibraryError::IncompleteDiscovery)
    ));
    let result = assemble_library([
        Ok(page(None, vec![], Some("empty"))),
        Ok(page(Some("empty"), vec![], Some("last"))),
        Ok(page(
            Some("last"),
            vec![record("one", AccessEvidence::CatalogOnly)],
            None,
        )),
    ])
    .expect("empty intermediate pages are valid");
    assert_eq!(result.books().len(), 1);
}

/// Rejects missing, out-of-order, repeated, cyclic, and post-terminal pages.
#[test]
fn pagination_requires_one_consistent_complete_chain() {
    let cases = [
        vec![page(Some("not-initial"), vec![], None)],
        vec![
            page(None, vec![], Some("a")),
            page(Some("wrong"), vec![], None),
        ],
        vec![page(None, vec![], Some("a")), page(None, vec![], None)],
        vec![
            page(None, vec![], Some("a")),
            page(Some("a"), vec![], Some("a")),
        ],
        vec![
            page(None, vec![], Some("a")),
            page(Some("a"), vec![], Some("b")),
            page(Some("b"), vec![], Some("a")),
        ],
        vec![page(None, vec![], None), page(None, vec![], None)],
        vec![page(None, vec![], Some(""))],
        vec![page(None, vec![], Some(" "))],
        vec![page(None, vec![], Some("control\ncursor"))],
    ];
    for pages in cases {
        assert!(matches!(
            assemble_library(pages.into_iter().map(Ok)),
            Err(LibraryError::InvalidPagination)
        ));
    }
}

/// Builds bounded synthetic pages; each raw record can be the same duplicate.
fn full_pages(count: usize, entries_per_page: usize, duplicate: bool) -> Vec<LibraryPage> {
    (0..count)
        .map(|index| LibraryPage {
            cursor: (index > 0).then(|| format!("page-{index}")),
            entries: (0..entries_per_page)
                .map(|entry| {
                    let id = if duplicate {
                        "same".to_owned()
                    } else {
                        format!("book-{index}-{entry}")
                    };
                    record(&id, AccessEvidence::CatalogOnly)
                })
                .collect(),
            next_cursor: (index + 1 < count).then(|| format!("page-{}", index + 1)),
        })
        .collect()
}

/// Checks exact page and aggregate caps with both unique and duplicate records.
#[test]
fn exact_entry_and_page_limits_are_accepted() {
    for duplicate in [false, true] {
        let result = assemble_library(
            full_pages(MAX_PAGES, MAX_ENTRIES_PER_PAGE, duplicate)
                .into_iter()
                .map(Ok),
        )
        .expect("exact maximum");
        assert_eq!(MAX_PAGES * MAX_ENTRIES_PER_PAGE, MAX_ENTRIES);
        assert_eq!(
            result.books().len(),
            if duplicate { 1 } else { MAX_ENTRIES }
        );
        assert!(
            assemble_library(full_pages(MAX_PAGES, 0, duplicate).into_iter().map(Ok))
                .expect("empty pages still count")
                .books()
                .is_empty()
        );
    }
}

/// Counts duplicate and unavailable records before retaining or validating them.
#[test]
fn oversized_pages_fail_before_entry_processing() {
    for evidence in [
        AccessEvidence::CatalogOnly,
        shared("a", AccessState::Unavailable),
        shared("b", AccessState::Unknown),
    ] {
        let mut entries = vec![record("same", evidence); MAX_ENTRIES_PER_PAGE + 1];
        entries[0].title.clear();
        assert!(matches!(
            assemble(entries),
            Err(LibraryError::LimitExceeded)
        ));
    }
}

/// Stops at the first page beyond the cap, even for an infinite input iterator.
#[test]
fn extra_pages_and_total_entries_cannot_hide_behind_duplicates() {
    let mut pages = full_pages(MAX_PAGES + 1, MAX_ENTRIES_PER_PAGE, true);
    pages[MAX_PAGES].entries.truncate(1);
    assert_eq!(
        pages.iter().map(|page| page.entries.len()).sum::<usize>(),
        MAX_ENTRIES + 1
    );
    assert!(matches!(
        assemble_library(pages.into_iter().map(Ok)),
        Err(LibraryError::LimitExceeded)
    ));
    assert!(matches!(
        assemble_library(full_pages(MAX_PAGES + 1, 0, false).into_iter().map(Ok)),
        Err(LibraryError::LimitExceeded)
    ));

    let reads = Cell::new(0);
    let endless = (0..).map(|index| {
        reads.set(reads.get() + 1);
        Ok(LibraryPage {
            cursor: (index > 0).then(|| format!("page-{index}")),
            entries: vec![],
            next_cursor: Some(format!("page-{}", index + 1)),
        })
    });
    assert!(matches!(
        assemble_library(endless),
        Err(LibraryError::LimitExceeded)
    ));
    assert_eq!(reads.get(), MAX_PAGES + 1);
}

/// Keeps private metadata out of diagnostics on both successful and failed input.
#[test]
fn metadata_and_validation_diagnostics_are_redacted() {
    let evidence = shared("synthetic-private-context", AccessState::Granted);
    let record = record("synthetic-private-book", evidence.clone());
    let page = page(None, vec![record.clone()], Some("synthetic-private-cursor"));
    let snapshot = assemble(vec![record.clone()]).expect("valid snapshot");
    for diagnostic in [
        format!("{evidence:?}"),
        format!("{record:?}"),
        format!("{page:?}"),
        format!("{snapshot:?}"),
    ] {
        assert!(!diagnostic.contains("synthetic-private"));
    }
    let mut conflicting = record.clone();
    conflicting.source_url = "https://example.com/synthetic-private-other".to_owned();
    let error = assemble(vec![record, conflicting]).expect_err("metadata conflict");
    assert!(!format!("{error} {error:?}").contains("synthetic-private"));
    assert!(error.source().is_none());
}

/// Keeps live discovery explicitly unavailable instead of returning an empty list.
#[test]
fn live_discovery_reports_its_blocker_without_inputs_or_side_effects() {
    let error = discover_library().expect_err("live discovery remains blocked");
    assert!(matches!(error, LibraryError::UnsupportedDiscovery));
    assert!(error.to_string().contains("permitted endpoint contract"));
    assert!(error.source().is_none());
}
