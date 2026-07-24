use chrono::NaiveDate;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::error::{AppError, Result};
use super::models::{BurndownData, BurndownPoint, Milestone};

type MilestoneRow = (i64, i64, String, Option<NaiveDate>, Option<NaiveDate>, Option<String>, chrono::DateTime<chrono::Utc>, Option<i64>);

fn row_to_milestone(row: MilestoneRow) -> Milestone {
    let (id, project_id, name, start_date, due_date, description, created_at, created_by_id) = row;
    Milestone {
        id,
        project_id,
        name,
        start_date,
        due_date,
        description,
        created_at,
        created_by_id,
    }
}

const MILESTONE_COLUMNS: &str = "id, project_id, name, start_date, due_date, description, created_at, created_by_id";

pub async fn create_milestone(
    db: &PgPool,
    project_id: i64,
    name: &str,
    start_date: Option<NaiveDate>,
    due_date: Option<NaiveDate>,
    description: Option<&str>,
    created_by_id: Option<i64>,
) -> Result<Milestone> {
    sqlx::query_as::<_, MilestoneRow>(
        &format!("INSERT INTO milestones (project_id, name, start_date, due_date, description, created_at, created_by_id) VALUES ($1, $2, $3, $4, $5, NOW(), $6) RETURNING {MILESTONE_COLUMNS}")
    )
    .bind(project_id)
    .bind(name)
    .bind(start_date)
    .bind(due_date)
    .bind(description)
    .bind(created_by_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)
    .map(row_to_milestone)
}

pub async fn get_milestone(db: &PgPool, id: i64) -> Result<Option<Milestone>> {
    sqlx::query_as::<_, MilestoneRow>(
        &format!("SELECT {MILESTONE_COLUMNS} FROM milestones WHERE id = $1 AND deleted_at IS NULL")
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)
    .map(|row| row.map(row_to_milestone))
}

pub async fn list_milestones_for_project(db: &PgPool, project_id: i64) -> Result<Vec<Milestone>> {
    sqlx::query_as::<_, MilestoneRow>(
        &format!("SELECT {MILESTONE_COLUMNS} FROM milestones WHERE project_id = $1 AND deleted_at IS NULL ORDER BY due_date ASC NULLS LAST, created_at ASC")
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| rows.into_iter().map(row_to_milestone).collect())
}

pub async fn update_milestone(
    db: &PgPool,
    id: i64,
    name: Option<&str>,
    start_date: Option<NaiveDate>,
    due_date: Option<NaiveDate>,
    description: Option<&str>,
) -> Result<Option<Milestone>> {
    sqlx::query_as::<_, MilestoneRow>(
        &format!("UPDATE milestones SET name = COALESCE($2, name), start_date = COALESCE($3, start_date), due_date = COALESCE($4, due_date), description = COALESCE($5, description) WHERE id = $1 AND deleted_at IS NULL RETURNING {MILESTONE_COLUMNS}")
    )
    .bind(id)
    .bind(name)
    .bind(start_date)
    .bind(due_date)
    .bind(description)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)
    .map(|row| row.map(row_to_milestone))
}

pub async fn delete_milestone(db: &PgPool, id: i64, deleted_by_id: Option<i64>) -> Result<()> {
    sqlx::query("UPDATE milestones SET deleted_at = NOW(), deleted_by_id = $2 WHERE id = $1")
        .bind(id)
        .bind(deleted_by_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

// Reconstructs, day by day, how many of the milestone's tickets were not yet
// "done" as of that day - by replaying ticket_activity_log's status_change
// events rather than relying on any snapshot table. Returns None if the
// milestone doesn't exist or hasn't had a start_date set yet (there's no
// meaningful x-axis origin without one).
pub async fn burndown_data(db: &PgPool, milestone_id: i64) -> Result<Option<BurndownData>> {
    let milestone = match get_milestone(db, milestone_id).await? {
        Some(m) => m,
        None => return Ok(None),
    };
    let due_date = milestone.due_date;
    let start_date = match milestone.start_date {
        Some(d) => d,
        None => {
            return Ok(Some(BurndownData { start_date: None, due_date, total_at_start: 0, points: Vec::new() }));
        }
    };

    let tickets: Vec<(i64, chrono::DateTime<chrono::Utc>, i64)> = sqlx::query_as(
        "SELECT id, created_at, status_id FROM tickets WHERE milestone_id = $1 AND deleted_at IS NULL"
    )
    .bind(milestone_id)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)?;

    if tickets.is_empty() {
        return Ok(Some(BurndownData { start_date: Some(start_date), due_date, total_at_start: 0, points: Vec::new() }));
    }

    let ticket_ids: Vec<i64> = tickets.iter().map(|(id, _, _)| *id).collect();

    let events: Vec<(i64, chrono::DateTime<chrono::Utc>, i64, i64)> = sqlx::query_as(
        "SELECT ticket_id, occurred_at, from_status_id, to_status_id FROM ticket_activity_log
         WHERE ticket_id = ANY($1) AND activity_type = 'status_change'
           AND from_status_id IS NOT NULL AND to_status_id IS NOT NULL
         ORDER BY occurred_at ASC"
    )
    .bind(&ticket_ids)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)?;

    let mut status_ids: std::collections::HashSet<i64> = tickets.iter().map(|(_, _, s)| *s).collect();
    status_ids.extend(events.iter().flat_map(|(_, _, from, to)| [*from, *to]));
    let status_ids: Vec<i64> = status_ids.into_iter().collect();

    let categories: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, category FROM workflow_statuses WHERE id = ANY($1)"
    )
    .bind(&status_ids)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)?;
    let category_map: HashMap<i64, String> = categories.into_iter().collect();

    let mut events_by_ticket: HashMap<i64, Vec<(chrono::DateTime<chrono::Utc>, i64, i64)>> = HashMap::new();
    for (ticket_id, occurred_at, from_status_id, to_status_id) in events {
        events_by_ticket.entry(ticket_id).or_default().push((occurred_at, from_status_id, to_status_id));
    }

    let today = chrono::Utc::now().date_naive();
    let end_date = due_date.map(|d| d.min(today)).unwrap_or(today).max(start_date);
    let total_days = due_date.and_then(|d| {
        let n = (d - start_date).num_days();
        (n > 0).then_some(n)
    });

    let mut points = Vec::new();
    let mut total_at_start: i64 = 0;
    let mut day = start_date;
    loop {
        let cutoff = day.and_hms_opt(23, 59, 59).unwrap().and_utc();

        let mut remaining = 0i64;
        let mut existed = 0i64;
        for (ticket_id, created_at, current_status_id) in &tickets {
            if *created_at > cutoff {
                continue;
            }
            existed += 1;

            // Status as of `cutoff`: start from what the ticket was created
            // with (the first event's from_status_id, or its current status
            // if it has never changed), then apply every event that had
            // already happened by `cutoff`.
            let mut status_id = match events_by_ticket.get(ticket_id).and_then(|evts| evts.first()) {
                Some((_, from_status_id, _)) => *from_status_id,
                None => *current_status_id,
            };
            if let Some(evts) = events_by_ticket.get(ticket_id) {
                for (occurred_at, _, to_status_id) in evts {
                    if *occurred_at <= cutoff {
                        status_id = *to_status_id;
                    } else {
                        break;
                    }
                }
            }

            let category = category_map.get(&status_id).map(String::as_str).unwrap_or("todo");
            if category != "done" && category != "confirmed" {
                remaining += 1;
            }
        }

        if day == start_date {
            total_at_start = existed;
        }

        let ideal = total_days.map(|total| {
            let elapsed = (day - start_date).num_days();
            let frac = (elapsed as f64 / total as f64).min(1.0);
            total_at_start as f64 * (1.0 - frac)
        });

        points.push(BurndownPoint { date: day, remaining, ideal });

        if day >= end_date {
            break;
        }
        day = day.succ_opt().unwrap();
    }

    Ok(Some(BurndownData { start_date: Some(start_date), due_date, total_at_start, points }))
}
