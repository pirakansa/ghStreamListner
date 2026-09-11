use std::collections::BTreeSet;

use rusqlite::params;

use super::{
    github_updated_at_advanced, item_type_db_value, now_rfc3339, StreamItemSave, StreamItemUpsert,
};
use crate::storage::{Result, Storage};

#[derive(Clone, Debug)]
struct ExistingStreamItem {
    id: i64,
    updated_at_github: String,
    is_merged: Option<bool>,
    review_status: Option<String>,
    merged_at_github: Option<String>,
}

impl Storage {
    pub fn upsert_stream_item(&self, item: &StreamItemUpsert) -> Result<StreamItemSave> {
        let now = now_rfc3339();
        let existing_item = self.find_existing_stream_item(item)?;
        let changed = existing_item
            .as_ref()
            .map(|existing| {
                github_updated_at_advanced(&existing.updated_at_github, &item.updated_at_github)
            })
            .unwrap_or(true);

        let enrichment_changed = if item.graphql_enriched {
            match &existing_item {
                Some(existing) if !changed => {
                    existing.is_merged != item.is_merged
                        || existing.review_status != item.review_status
                        || existing.merged_at_github != item.merged_at_github
                        || self.enrichment_relations_changed(existing.id, item)?
                }
                _ => true,
            }
        } else {
            false
        };

        self.connection().execute(
            "INSERT INTO stream_items (
                host_id, node_id, repository_owner, repository_name, number, item_type,
                title, author_login, author_avatar_url, html_url, api_url, state, is_draft, is_merged,
                review_status, comment_count, created_at_github, updated_at_github,
                closed_at_github, merged_at_github, last_seen_at, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20, ?21, ?21, ?21
             )
             ON CONFLICT(host_id, repository_owner, repository_name, number, item_type)
             DO UPDATE SET
                node_id = excluded.node_id,
                title = excluded.title,
                author_login = excluded.author_login,
                author_avatar_url = excluded.author_avatar_url,
                html_url = excluded.html_url,
                api_url = excluded.api_url,
                state = excluded.state,
                is_draft = excluded.is_draft,
                is_merged = CASE WHEN ?22 = 1 THEN excluded.is_merged ELSE stream_items.is_merged END,
                review_status = CASE WHEN ?22 = 1 THEN excluded.review_status ELSE stream_items.review_status END,
                comment_count = excluded.comment_count,
                created_at_github = excluded.created_at_github,
                updated_at_github = excluded.updated_at_github,
                closed_at_github = excluded.closed_at_github,
                merged_at_github = CASE WHEN ?22 = 1 THEN excluded.merged_at_github ELSE stream_items.merged_at_github END,
                last_seen_at = excluded.last_seen_at,
                updated_at = excluded.updated_at",
            params![
                item.host_id,
                item.node_id,
                item.repository_owner,
                item.repository_name,
                item.number,
                item_type_db_value(&item.item_type),
                item.title,
                item.author_login,
                item.author_avatar_url,
                item.html_url,
                item.api_url,
                item.state,
                item.is_draft.map(i64::from),
                item.is_merged.map(i64::from),
                item.review_status,
                item.comment_count,
                item.created_at_github,
                item.updated_at_github,
                item.closed_at_github,
                item.merged_at_github,
                now,
                i64::from(item.graphql_enriched)
            ],
        )?;

        let id = existing_item
            .as_ref()
            .map(|existing| existing.id)
            .unwrap_or_else(|| self.connection().last_insert_rowid());

        self.connection().execute(
            "INSERT OR IGNORE INTO item_state (stream_item_id, updated_at)
             VALUES (?1, ?2)",
            params![id, now],
        )?;

        if existing_item.is_some() && changed {
            self.set_read_state(id, true)?;
        }

        if changed || existing_item.is_none() {
            self.replace_labels(id, &item.labels)?;
            self.replace_assignees(id, &item.assignees)?;
        }
        if item.graphql_enriched && (changed || enrichment_changed) {
            self.replace_review_requests(id, &item.review_requests)?;
            self.replace_reviews(id, &item.reviewers)?;
            self.replace_participants(id, &item.participants)?;
            self.replace_mentions(id, &item.mentions)?;
        }

        Ok(StreamItemSave {
            id,
            changed: changed || enrichment_changed,
        })
    }

    fn enrichment_relations_changed(&self, id: i64, item: &StreamItemUpsert) -> Result<bool> {
        let mut statement = self.connection().prepare(
            "SELECT 'request', login, avatar_url, NULL FROM stream_item_review_requests
             WHERE stream_item_id = ?1
             UNION ALL
             SELECT 'review', login, avatar_url, state FROM stream_item_reviews
             WHERE stream_item_id = ?1
             UNION ALL
             SELECT 'participant', login, avatar_url, NULL FROM stream_item_participants
             WHERE stream_item_id = ?1
             UNION ALL
             SELECT 'mention', login, NULL, NULL FROM stream_item_mentions
             WHERE stream_item_id = ?1",
        )?;
        let existing = statement
            .query_map(params![id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        let mut incoming = BTreeSet::new();
        for (kind, people) in [
            ("request", &item.review_requests),
            ("participant", &item.participants),
        ] {
            for person in people {
                incoming.insert((
                    kind.to_owned(),
                    person.login.clone(),
                    person.avatar_url.clone(),
                    None,
                ));
            }
        }
        for review in &item.reviewers {
            incoming.insert((
                "review".to_owned(),
                review.login.clone(),
                review.avatar_url.clone(),
                Some(review.state.clone()),
            ));
        }
        for mention in &item.mentions {
            incoming.insert(("mention".to_owned(), mention.clone(), None, None));
        }
        Ok(existing != incoming)
    }

    fn find_existing_stream_item(
        &self,
        item: &StreamItemUpsert,
    ) -> Result<Option<ExistingStreamItem>> {
        let result = self.connection().query_row(
            "SELECT id, updated_at_github, is_merged, review_status, merged_at_github FROM stream_items
             WHERE host_id = ?1
               AND repository_owner = ?2
               AND repository_name = ?3
               AND number = ?4
               AND item_type = ?5",
            params![
                item.host_id,
                item.repository_owner,
                item.repository_name,
                item.number,
                item_type_db_value(&item.item_type)
            ],
            |row| {
                Ok(ExistingStreamItem {
                    id: row.get(0)?,
                    updated_at_github: row.get(1)?,
                    is_merged: row.get(2)?,
                    review_status: row.get(3)?,
                    merged_at_github: row.get(4)?,
                })
            },
        );

        match result {
            Ok(existing_item) => Ok(Some(existing_item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
