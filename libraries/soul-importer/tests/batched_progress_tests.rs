mod test_helpers;

use soul_core::types::CreateLibrarySource;

#[tokio::test]
async fn test_update_counts_bulk() {
    let pool = test_helpers::setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: "/fake".to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    let progress = soul_storage::scan_progress::start(&pool, source.id, Some(100))
        .await
        .unwrap();

    // Bulk update all counters at once
    soul_storage::scan_progress::update_counts(&pool, progress.id, 50, 30, 10, 5, 5)
        .await
        .unwrap();

    // Read back and verify
    let updated = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .expect("scan progress should exist");

    assert_eq!(updated.processed_files, 50);
    assert_eq!(updated.new_files, 30);
    assert_eq!(updated.updated_files, 10);
    assert_eq!(updated.removed_files, 5);
    assert_eq!(updated.errors, 5);
}

#[tokio::test]
async fn test_update_counts_additive() {
    let pool = test_helpers::setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: "/fake".to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    let progress = soul_storage::scan_progress::start(&pool, source.id, Some(200))
        .await
        .unwrap();

    // First bulk update
    soul_storage::scan_progress::update_counts(&pool, progress.id, 50, 30, 10, 5, 5)
        .await
        .unwrap();

    // Second bulk update — should be additive
    soul_storage::scan_progress::update_counts(&pool, progress.id, 50, 20, 5, 0, 25)
        .await
        .unwrap();

    // Read back and verify totals
    let updated = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .expect("scan progress should exist");

    assert_eq!(updated.processed_files, 100); // 50 + 50
    assert_eq!(updated.new_files, 50); // 30 + 20
    assert_eq!(updated.updated_files, 15); // 10 + 5
    assert_eq!(updated.removed_files, 5); // 5 + 0
    assert_eq!(updated.errors, 30); // 5 + 25
}
