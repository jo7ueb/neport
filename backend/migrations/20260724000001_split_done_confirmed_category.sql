-- Splits the "done" status category into two tiers: "done" now means the
-- assignee marked the ticket complete but nobody has verified it yet,
-- while a new "confirmed" category means a project manager reviewed and
-- closed it. Only the seeded "closed" status (key='closed') is migrated to
-- the new category - anything else already using 'done' (including any
-- custom status an admin created) keeps meaning "done, not yet confirmed".
ALTER TABLE workflow_statuses DROP CONSTRAINT workflow_statuses_category_check;
ALTER TABLE workflow_statuses ADD CONSTRAINT workflow_statuses_category_check
    CHECK ((category = ANY (ARRAY['todo'::text, 'in_progress'::text, 'done'::text, 'confirmed'::text])));

UPDATE workflow_statuses SET category = 'confirmed' WHERE key = 'closed' AND category = 'done';
