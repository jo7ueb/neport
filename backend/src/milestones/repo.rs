use chrono::NaiveDate;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::error::{AppError, Result};
use super::models::{BurndownData, BurndownPoint, Milestone};

type MilestoneRow = (
    i64,
    i64,
    String,
    Option<NaiveDate>,
    Option<NaiveDate>,
    Option<String>,
    chrono::DateTime<chrono::Utc>,
    Option<i64>,
    i64,
    i64,
    i64,
    f64,
    i64,
);

fn row_to_milestone(row: MilestoneRow) -> Milestone {
    let (
        id,
        project_id,
        name,
        start_date,
        due_date,
        description,
        created_at,
        created_by_id,
        total_story_points,
        done_story_points,
        remaining_story_points,
        remaining_estimated_hours,
        unestimated_ticket_count,
    ) = row;
    Milestone {
        id,
        project_id,
        name,
        start_date,
        due_date,
        description,
        created_at,
        created_by_id,
        total_story_points,
        done_story_points,
        remaining_story_points,
        remaining_estimated_hours,
        unestimated_ticket_count,
    }
}

const MILESTONE_COLUMNS: &str = "m.id, m.project_id, m.name, m.start_date, m.due_date, m.description, m.created_at, m.created_by_id";
const MILESTONE_SUMMARY_COLUMNS: &str = "COALESCE(SUM(t.story_points)::bigint, 0) AS total_story_points,
    COALESCE(SUM(t.story_points) FILTER (WHERE ws.category IN ('done', 'confirmed'))::bigint, 0) AS done_story_points,
    COALESCE(SUM(t.story_points) FILTER (WHERE ws.category NOT IN ('done', 'confirmed'))::bigint, 0) AS remaining_story_points,
    COALESCE(SUM(t.story_points) FILTER (WHERE ws.category NOT IN ('done', 'confirmed'))::double precision * p.hours_per_story_point, 0) AS remaining_estimated_hours,
    COUNT(*) FILTER (WHERE t.id IS NOT NULL AND t.story_points IS NULL AND ws.category NOT IN ('done', 'confirmed'))::bigint AS unestimated_ticket_count";

async fn fetch_milestone(db: &PgPool, where_clause: &str, bind_id: i64) -> Result<Option<Milestone>> {
    let sql = format!(
        "SELECT {MILESTONE_COLUMNS}, {MILESTONE_SUMMARY_COLUMNS}
         FROM milestones m
         JOIN projects p ON p.id = m.project_id
         LEFT JOIN tickets t ON t.milestone_id = m.id AND t.deleted_at IS NULL
            AND NOT EXISTS (SELECT 1 FROM tickets child WHERE child.parent_ticket_id = t.id AND child.deleted_at IS NULL)
         LEFT JOIN workflow_statuses ws ON ws.id = t.status_id
         WHERE {where_clause}
         GROUP BY m.id, p.hours_per_story_point"
    );
    sqlx::query_as::<_, MilestoneRow>(&sql)
        .bind(bind_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)
        .map(|row| row.map(row_to_milestone))
}

pub async fn create_milestone(
    db: &PgPool,
    project_id: i64,
    name: &str,
    start_date: Option<NaiveDate>,
    due_date: Option<NaiveDate>,
    description: Option<&str>,
    created_by_id: Option<i64>,
) -> Result<Milestone> {
    let inserted_id: (i64,) = sqlx::query_as(
        "INSERT INTO milestones (project_id, name, start_date, due_date, description, created_at, created_by_id) VALUES ($1, $2, $3, $4, $5, NOW(), $6) RETURNING id"
    )
    .bind(project_id)
    .bind(name)
    .bind(start_date)
    .bind(due_date)
    .bind(description)
    .bind(created_by_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;

    get_milestone(db, inserted_id.0).await?.ok_or(AppError::InternalServerError)
}

pub async fn get_milestone(db: &PgPool, id: i64) -> Result<Option<Milestone>> {
    fetch_milestone(db, "m.id = $1 AND m.deleted_at IS NULL", id).await
}

pub async fn list_milestones_for_project(db: &PgPool, project_id: i64) -> Result<Vec<Milestone>> {
    let sql = format!(
        "SELECT {MILESTONE_COLUMNS}, {MILESTONE_SUMMARY_COLUMNS}
         FROM milestones m
         JOIN projects p ON p.id = m.project_id
         LEFT JOIN tickets t ON t.milestone_id = m.id AND t.deleted_at IS NULL
            AND NOT EXISTS (SELECT 1 FROM tickets child WHERE child.parent_ticket_id = t.id AND child.deleted_at IS NULL)
         LEFT JOIN workflow_statuses ws ON ws.id = t.status_id
         WHERE m.project_id = $1 AND m.deleted_at IS NULL
         GROUP BY m.id, p.hours_per_story_point
         ORDER BY m.due_date ASC NULLS LAST, m.created_at ASC"
    );
    sqlx::query_as::<_, MilestoneRow>(&sql)
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
    let updated_id: Option<(i64,)> = sqlx::query_as(
        "UPDATE milestones SET name = COALESCE($2, name), start_date = COALESCE($3, start_date), due_date = COALESCE($4, due_date), description = COALESCE($5, description) WHERE id = $1 AND deleted_at IS NULL RETURNING id"
    )
    .bind(id)
    .bind(name)
    .bind(start_date)
    .bind(due_date)
    .bind(description)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)?;

    match updated_id {
        Some((id,)) => get_milestone(db, id).await,
        None => Ok(None),
    }
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

pub async fn burndown_data(db: &PgPool, milestone_id: i64) -> Result<Option<BurndownData>> {
    let milestone = match get_milestone(db, milestone_id).await? {
        Some(m) => m,
        None => return Ok(None),
    };
    let due_date = milestone.due_date;
    let start_date = match milestone.start_date {
        Some(d) => d,
        None => {
            return Ok(Some(BurndownData {
                start_date: None,
                due_date,
                total_at_start: 0,
                unestimated_ticket_count: milestone.unestimated_ticket_count,
                points: Vec::new(),
            }));
        }
    };

    let tickets: Vec<(i64, chrono::DateTime<chrono::Utc>, i64, Option<i32>)> = sqlx::query_as(
        "SELECT t.id, t.created_at, t.status_id, t.story_points FROM tickets t
         WHERE t.milestone_id = $1 AND t.deleted_at IS NULL
           AND NOT EXISTS (SELECT 1 FROM tickets child WHERE child.parent_ticket_id = t.id AND child.deleted_at IS NULL)"
    )
    .bind(milestone_id)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)?;

    if tickets.is_empty() {
        return Ok(Some(BurndownData {
            start_date: Some(start_date),
            due_date,
            total_at_start: 0,
            unestimated_ticket_count: 0,
            points: Vec::new(),
        }));
    }

    let ticket_ids: Vec<i64> = tickets.iter().map(|(id, _, _, _)| *id).collect();

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

    let mut status_ids: std::collections::HashSet<i64> = tickets.iter().map(|(_, _, s, _)| *s).collect();
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
        let mut existed_story_points = 0i64;
        for (ticket_id, created_at, current_status_id, story_points) in &tickets {
            if *created_at > cutoff {
                continue;
            }
            let Some(story_points) = story_points else {
                continue;
            };
            existed_story_points += i64::from(*story_points);

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
                remaining += i64::from(*story_points);
            }
        }

        if day == start_date {
            total_at_start = existed_story_points;
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

    Ok(Some(BurndownData {
        start_date: Some(start_date),
        due_date,
        total_at_start,
        unestimated_ticket_count: milestone.unestimated_ticket_count,
        points,
    }))
}
