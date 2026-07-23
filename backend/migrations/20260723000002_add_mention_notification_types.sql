-- Adds notif_type values for @mention notifications (comments, ticket
-- descriptions, daily reports). See backend/src/mentions.rs for extraction.
ALTER TABLE notifications DROP CONSTRAINT notifications_notif_type_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_notif_type_check
    CHECK (notif_type = ANY (ARRAY[
        'ticket_assigned'::text,
        'ticket_overdue'::text,
        'ticket_commented'::text,
        'report_approved'::text,
        'report_rejected'::text,
        'ticket_mentioned'::text,
        'report_mentioned'::text
    ]));
