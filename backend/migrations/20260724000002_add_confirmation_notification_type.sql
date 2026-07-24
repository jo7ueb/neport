-- Adds the notif_type value for "a ticket reached done and needs a project
-- manager's confirmation" notifications. See tickets::handlers::record_update_activity.
ALTER TABLE notifications DROP CONSTRAINT notifications_notif_type_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_notif_type_check
    CHECK (notif_type = ANY (ARRAY[
        'ticket_assigned'::text,
        'ticket_overdue'::text,
        'ticket_commented'::text,
        'report_approved'::text,
        'report_rejected'::text,
        'ticket_mentioned'::text,
        'report_mentioned'::text,
        'ticket_confirmation_requested'::text
    ]));
