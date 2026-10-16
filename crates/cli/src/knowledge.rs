// SPDX-License-Identifier: Apache-2.0
//! M7 knowledge records: typed model and input validation. Storage lives in
//! `knowledge_store.rs`; JSON emission and CLI wiring arrive in Task 7.
//! Foundation statuses are `active`/`archived` only: `superseded` and
//! `retracted` record statuses belong to a later stage and are rejected here.

/// A single immutable revision of a knowledge record.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordVersion {
    pub id: String,
    pub revision: u64,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub author: String,
    pub created_unix: i64,
}

/// Input for creating a record. There is deliberately no way to express a
/// starting revision: creation always yields revision 1.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRecord {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub author: String,
}

/// Input for updating a record. ID and kind are absent by construction: an
/// update can never rename them. Every update carries the new status
/// explicitly; status changes are ordinary revisions, not side channels.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateRecord {
    pub title: String,
    pub body: String,
    pub status: String,
    pub author: String,
}

/// Caller-supplied repository-scoped IDs: 1–128 ASCII chars,
/// `[a-z0-9][a-z0-9._-]*`. SQLite row IDs and titles are never identity.
pub fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 128 || !id.is_ascii() {
        return Err(format!("invalid record id: {id:?}"));
    }
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return Err(format!("invalid record id: {id:?}")),
    }
    let rest_ok = chars
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-');
    if !rest_ok {
        return Err(format!("invalid record id: {id:?}"));
    }
    Ok(())
}

/// Foundation kinds: `decision`, `constraint`, `benchmark`.
pub fn validate_kind(kind: &str) -> Result<(), String> {
    match kind {
        "decision" | "constraint" | "benchmark" => Ok(()),
        _ => Err(format!("invalid record kind: {kind:?}")),
    }
}

/// Foundation statuses: `active`, `archived`. `superseded`/`retracted` are
/// rejected here; relation retraction (an event on links) is unrelated and
/// unaffected.
pub fn validate_status(status: &str) -> Result<(), String> {
    match status {
        "active" | "archived" => Ok(()),
        _ => Err(format!("invalid record status: {status:?}")),
    }
}

pub fn validate_new(input: &NewRecord) -> Result<(), String> {
    validate_id(&input.id)?;
    validate_kind(&input.kind)?;
    validate_status(&input.status)?;
    if input.title.is_empty() || input.body.is_empty() || input.author.is_empty() {
        return Err("title, body and author are required".to_string());
    }
    Ok(())
}

pub fn validate_update(input: &UpdateRecord) -> Result<(), String> {
    validate_status(&input.status)?;
    if input.title.is_empty() || input.body.is_empty() || input.author.is_empty() {
        return Err("title, body and author are required".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_id_rules() {
        assert!(validate_id("decision-001").is_ok());
        assert!(validate_id("a").is_ok());
        assert!(validate_id("z9._-x").is_ok());
        for bad in ["", "A", "-abc", ".abc", "_abc", "has space", "UPPER", "ábc"] {
            assert!(validate_id(bad).is_err(), "must reject {bad:?}");
        }
        assert!(validate_id(&"a".repeat(129)).is_err());
        assert!(validate_id(&"a".repeat(128)).is_ok());
    }

    #[test]
    fn knowledge_kind_rules() {
        for ok in ["decision", "constraint", "benchmark"] {
            assert!(validate_kind(ok).is_ok());
        }
        for bad in ["", "note", "DECISION", "decision "] {
            assert!(validate_kind(bad).is_err());
        }
    }

    #[test]
    fn knowledge_status_rules() {
        assert!(validate_status("active").is_ok());
        assert!(validate_status("archived").is_ok());
        // Full-stage record statuses are never valid in Foundation, even
        // though relation retraction events are.
        for bad in ["", "superseded", "retracted", "Active", "deleted"] {
            assert!(validate_status(bad).is_err());
        }
    }

    #[test]
    fn knowledge_new_and_update_rules() {
        let good = NewRecord {
            id: "decision-001".to_string(),
            kind: "decision".to_string(),
            title: "t".to_string(),
            body: "b".to_string(),
            status: "active".to_string(),
            author: "r".to_string(),
        };
        assert!(validate_new(&good).is_ok());
        let mut bad_kind = good.clone();
        bad_kind.kind = "note".to_string();
        assert!(validate_new(&bad_kind).is_err());
        let mut bad_status = good.clone();
        bad_status.status = "retracted".to_string();
        assert!(validate_new(&bad_status).is_err());
        let mut empty_title = good.clone();
        empty_title.title = String::new();
        assert!(validate_new(&empty_title).is_err());
        let update = UpdateRecord {
            title: "t2".to_string(),
            body: "b2".to_string(),
            status: "archived".to_string(),
            author: "r".to_string(),
        };
        assert!(validate_update(&update).is_ok());
        let mut bad_update = update.clone();
        bad_update.status = "superseded".to_string();
        assert!(validate_update(&bad_update).is_err());
    }
}
