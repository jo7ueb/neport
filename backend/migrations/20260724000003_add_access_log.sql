-- Centralizes "who did this, from which IP, when" into one table instead of
-- adding an ip_address column to every domain table that records an action.
-- Each domain table below gets a nullable FK to a fresh row inserted here at
-- write time; existing rows (predating this migration) get NULL, same as any
-- other "column added later, no known value for old rows" column in this
-- schema (see ticket_dependencies.created_by_id for precedent).
CREATE TABLE access_log (
    id bigint NOT NULL,
    actor_id bigint NOT NULL,
    ip_address text,
    occurred_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE SEQUENCE access_log_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE access_log_id_seq OWNED BY access_log.id;

ALTER TABLE ONLY access_log ALTER COLUMN id SET DEFAULT nextval('access_log_id_seq'::regclass);

ALTER TABLE ONLY access_log
    ADD CONSTRAINT access_log_pkey PRIMARY KEY (id);

ALTER TABLE ONLY access_log
    ADD CONSTRAINT access_log_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES users(id) ON DELETE RESTRICT;

CREATE INDEX idx_access_log_actor_id ON access_log (actor_id);

-- audit_log used to store ip_address directly; it now goes through
-- access_log like everything else, so the same column carries every source's
-- IP uniformly for the admin audit log's read-side UNION ALL query.
ALTER TABLE audit_log ADD COLUMN access_log_id bigint REFERENCES access_log(id);
ALTER TABLE audit_log DROP COLUMN ip_address;

ALTER TABLE ticket_activity_log ADD COLUMN access_log_id bigint REFERENCES access_log(id);
ALTER TABLE ticket_comments ADD COLUMN access_log_id bigint REFERENCES access_log(id);

-- Two columns because a single row already represents two distinct events
-- (add + soft-delete) - see ticket_dependencies.deleted_at's own comment.
ALTER TABLE ticket_dependencies ADD COLUMN created_access_log_id bigint REFERENCES access_log(id);
ALTER TABLE ticket_dependencies ADD COLUMN deleted_access_log_id bigint REFERENCES access_log(id);

ALTER TABLE daily_approvals ADD COLUMN access_log_id bigint REFERENCES access_log(id);
ALTER TABLE monthly_closures ADD COLUMN access_log_id bigint REFERENCES access_log(id);

-- Two columns for the same reason as ticket_dependencies above: one row
-- covers both the start and (later) end event.
ALTER TABLE work_sessions ADD COLUMN start_access_log_id bigint REFERENCES access_log(id);
ALTER TABLE work_sessions ADD COLUMN end_access_log_id bigint REFERENCES access_log(id);

ALTER TABLE attachments ADD COLUMN access_log_id bigint REFERENCES access_log(id);
ALTER TABLE user_approvers ADD COLUMN access_log_id bigint REFERENCES access_log(id);
