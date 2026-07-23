use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::workflows::models::StatusCategory;

pub struct StatusChange {
    pub occurred_at: DateTime<Utc>,
    pub from_status_id: i64,
    pub to_status_id: i64,
}

// Sums how long a ticket has spent in an "in_progress"-category status,
// reconstructed from its status_change activity log: a change's
// from_status_id is what the ticket was in during the interval that just
// ended, and its to_status_id is what it's in from then on - so a ticket
// that was paused (moved back to a todo-category status) and later resumed
// only accrues time for the in_progress stretches, not the pause. A ticket
// currently sitting in an in_progress status keeps accruing up to `now`, so
// calling this again later returns a larger number for an unfinished ticket.
pub fn in_progress_seconds(
    created_at: DateTime<Utc>,
    current_status_id: i64,
    changes: &[StatusChange],
    categories: &HashMap<i64, StatusCategory>,
    now: DateTime<Utc>,
) -> i64 {
    let mut total = chrono::Duration::zero();
    let mut interval_start = created_at;
    let mut interval_status = changes.first().map_or(current_status_id, |c| c.from_status_id);

    for change in changes {
        if categories.get(&interval_status) == Some(&StatusCategory::InProgress) {
            total += change.occurred_at - interval_start;
        }
        interval_start = change.occurred_at;
        interval_status = change.to_status_id;
    }

    if categories.get(&interval_status) == Some(&StatusCategory::InProgress) {
        total += now - interval_start;
    }

    total.num_seconds().max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(offset_hours: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(0, 0).unwrap() + chrono::Duration::hours(offset_hours)
    }

    fn categories() -> HashMap<i64, StatusCategory> {
        HashMap::from([
            (1, StatusCategory::Todo),
            (2, StatusCategory::InProgress),
            (3, StatusCategory::Done),
        ])
    }

    #[test]
    fn zero_changes_uses_current_status_for_the_whole_span() {
        // Never moved out of an in_progress status since creation.
        let seconds = in_progress_seconds(t(0), 2, &[], &categories(), t(5));
        assert_eq!(seconds, 5 * 3600);
    }

    #[test]
    fn zero_changes_in_a_todo_status_counts_nothing() {
        let seconds = in_progress_seconds(t(0), 1, &[], &categories(), t(5));
        assert_eq!(seconds, 0);
    }

    #[test]
    fn simple_todo_to_in_progress_to_done() {
        let changes = vec![
            StatusChange { occurred_at: t(1), from_status_id: 1, to_status_id: 2 },
            StatusChange { occurred_at: t(4), from_status_id: 2, to_status_id: 3 },
        ];
        // todo for 1h (not counted), in_progress for 3h (1->4), done onward (not counted).
        let seconds = in_progress_seconds(t(0), 3, &changes, &categories(), t(10));
        assert_eq!(seconds, 3 * 3600);
    }

    #[test]
    fn pausing_back_to_todo_and_resuming_only_counts_in_progress_stretches() {
        let changes = vec![
            StatusChange { occurred_at: t(1), from_status_id: 1, to_status_id: 2 }, // todo -> in_progress at t1
            StatusChange { occurred_at: t(3), from_status_id: 2, to_status_id: 1 }, // paused at t3 (2h in_progress)
            StatusChange { occurred_at: t(6), from_status_id: 1, to_status_id: 2 }, // resumed at t6
            StatusChange { occurred_at: t(8), from_status_id: 2, to_status_id: 3 }, // done at t8 (2h in_progress)
        ];
        let seconds = in_progress_seconds(t(0), 3, &changes, &categories(), t(20));
        assert_eq!(seconds, 4 * 3600);
    }

    #[test]
    fn still_in_progress_keeps_accruing_up_to_now() {
        let changes = vec![StatusChange { occurred_at: t(1), from_status_id: 1, to_status_id: 2 }];
        let seconds = in_progress_seconds(t(0), 2, &changes, &categories(), t(4));
        assert_eq!(seconds, 3 * 3600);
    }
}
