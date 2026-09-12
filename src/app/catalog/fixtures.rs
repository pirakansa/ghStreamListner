use crate::models::{ItemPerson, ItemReview, ItemType, StreamItem};

const FIXTURE_TIME: &str = "2026-09-01T12:00:00Z";
const FIXTURE_URL: &str = "https://github.example.test/demo/catalog";
fn sample_item(id: i64) -> StreamItem {
    let variant = (id - 1) % 6;
    let (item_type, title) = match variant {
        0 => (ItemType::Issue, "Unread issue with labels"),
        1 => (ItemType::PullRequest, "Draft pull request awaiting review"),
        2 => (ItemType::PullRequest, "Merged pull request with a bookmark"),
        3 => (ItemType::Discussion, "Discussion with the community"),
        4 => (ItemType::Issue, "Closed and archived issue"),
        _ => (
            ItemType::PullRequest,
            "Open pull request with reviewer feedback",
        ),
    };
    let is_pr = item_type == ItemType::PullRequest;
    let closed = matches!(variant, 2 | 4);
    StreamItem {
        id,
        repository_owner: "demo".into(),
        repository_name: "catalog".into(),
        number: id,
        item_type,
        title: title.into(),
        author_login: Some("sample-author".into()),
        author_avatar_url: None,
        html_url: format!("{FIXTURE_URL}/items/{id}"),
        state: if closed { "closed" } else { "open" }.into(),
        is_draft: is_pr.then_some(variant == 1),
        is_merged: is_pr.then_some(variant == 2),
        review_status: is_pr.then(|| {
            if variant == 2 {
                "approved"
            } else {
                "review_required"
            }
            .into()
        }),
        comment_count: id,
        created_at_github: FIXTURE_TIME.into(),
        updated_at_github: FIXTURE_TIME.into(),
        closed_at_github: closed.then(|| FIXTURE_TIME.into()),
        merged_at_github: (variant == 2).then(|| FIXTURE_TIME.into()),
        read_at: (variant % 2 != 0).then(|| FIXTURE_TIME.into()),
        labels: vec!["enhancement".into(), "needs-discussion".into()],
        assignees: vec![ItemPerson {
            login: "sample-assignee".into(),
            avatar_url: None,
        }],
        review_requests: if is_pr {
            vec![ItemPerson {
                login: "sample-reviewer".into(),
                avatar_url: None,
            }]
        } else {
            Vec::new()
        },
        reviewers: if is_pr {
            vec![ItemReview {
                login: "sample-maintainer".into(),
                avatar_url: None,
                state: "approved".into(),
            }]
        } else {
            Vec::new()
        },
        is_unread: variant % 2 == 0,
        is_bookmarked: variant == 2,
        is_archived: variant == 4,
    }
}

use crate::models::StreamSource;
use crate::saved_query_io::{ImportedFilterStream, ImportedSavedQuery};
use crate::storage::{items::StreamItemUpsert, Result, Storage};

pub(super) fn definitions() -> Vec<ImportedSavedQuery> {
    [
        (
            "Sample work",
            "repo:demo/catalog",
            StreamSource::IssueOrPullRequest,
        ),
        ("Community", "repo:demo/catalog", StreamSource::Discussion),
    ]
    .into_iter()
    .enumerate()
    .map(|(position, (name, query, source))| ImportedSavedQuery {
        name: name.into(),
        query: query.into(),
        source,
        enabled: true,
        position: position as i64,
        filter_streams: if position == 0 {
            vec![ImportedFilterStream {
                name: "Pull requests".into(),
                filter_query: "is:pr".into(),
                enabled: true,
                position: 0,
            }]
        } else {
            Vec::new()
        },
    })
    .collect()
}

pub(super) fn seed(storage: &Storage, host_id: i64) -> Result<()> {
    storage.replace_saved_queries(host_id, &definitions())?;
    for id in 1..=7 {
        let mut item = sample_item(id);
        if id == 7 {
            item.title =
                "Long title: 日本語の表示と折り返し — review layout with labels and assignees. "
                    .repeat(3);
            item.labels = (1..=8).map(|id| format!("sample-label-{id}")).collect();
        }
        let saved = storage.upsert_stream_item(&as_upsert(item.clone(), host_id))?;
        storage.set_read_state(saved.id, item.is_unread)?;
        storage.set_bookmarked(saved.id, item.is_bookmarked)?;
        storage.set_archived(saved.id, item.is_archived)?;
    }
    refresh_matches(storage, host_id)
}

pub(super) fn refresh_matches(storage: &Storage, host_id: i64) -> Result<()> {
    // Library visibility requires enabled query matches, so obtain the fixture IDs
    // directly when a newly imported definition has cleared all matches.
    let mut statement = storage
        .connection()
        .prepare("SELECT id, item_type FROM stream_items WHERE host_id = ?1")?;
    let samples = statement
        .query_map([host_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for query in storage
        .list_saved_queries(host_id)?
        .iter()
        .filter(|query| query.enabled)
    {
        for (id, kind) in &samples {
            let matches = match query.source {
                StreamSource::Discussion => kind == "discussion",
                StreamSource::IssueOrPullRequest | StreamSource::ProjectV2 => kind != "discussion",
            };
            if matches {
                storage.record_saved_query_match(query.id, *id, None)?;
            }
        }
    }
    Ok(())
}

fn as_upsert(item: StreamItem, host_id: i64) -> StreamItemUpsert {
    StreamItemUpsert {
        host_id,
        node_id: None,
        api_url: None,
        participants: Vec::new(),
        mentions: Vec::new(),
        graphql_enriched: true,
        repository_owner: item.repository_owner,
        repository_name: item.repository_name,
        number: item.number,
        item_type: item.item_type,
        title: item.title,
        author_login: item.author_login,
        author_avatar_url: item.author_avatar_url,
        html_url: item.html_url,
        state: item.state,
        is_draft: item.is_draft,
        is_merged: item.is_merged,
        review_status: item.review_status,
        comment_count: item.comment_count,
        created_at_github: item.created_at_github,
        updated_at_github: item.updated_at_github,
        closed_at_github: item.closed_at_github,
        merged_at_github: item.merged_at_github,
        labels: item.labels,
        assignees: item.assignees,
        review_requests: item.review_requests,
        reviewers: item.reviewers,
    }
}
