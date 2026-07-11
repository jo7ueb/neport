-- Consolidated baseline schema. This replaces what used to be 14 separate
-- incremental migrations (init, system settings, workflow engine, ticket
-- activity log evolution, user avatars, project members, approvals,
-- timezones) now that no deployment needs to replay the incremental history
-- - squashed into a single file after a full DB wipe.
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Pasted/dragged-in images from markdown fields (ticket descriptions,
-- comments). Stored on disk under a fixed volume path, keyed by the sha256
-- of the file content so the same image can never collide with a
-- differently-named upload and re-uploading identical bytes is a no-op
-- (ON CONFLICT DO NOTHING at insert time - see attachments::repo::upsert).
CREATE TABLE attachments (
    id bigint NOT NULL,
    sha256 text NOT NULL,
    content_type text NOT NULL,
    size_bytes bigint NOT NULL,
    uploaded_by_id bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE SEQUENCE attachments_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE attachments_id_seq OWNED BY attachments.id;

-- System-wide audit trail for actions that mutate or remove an existing row
-- (so there's no natural "created_by" column to hang the actor off of) -
-- e.g. suspending/deleting a user, archiving a project, resetting a
-- password. Counterpart to ticket_activity_log, which covers ticket-scoped
-- events only. target_id is intentionally not a foreign key: it points into
-- whichever table target_type names, and the target may later be gone.
CREATE TABLE audit_log (
    id bigint NOT NULL,
    actor_id bigint NOT NULL,
    action text NOT NULL,
    target_type text NOT NULL,
    target_id bigint,
    from_text text,
    to_text text,
    occurred_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE SEQUENCE audit_log_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE audit_log_id_seq OWNED BY audit_log.id;

CREATE TABLE daily_approvals (
    id bigint NOT NULL,
    user_id bigint NOT NULL,
    report_date date NOT NULL,
    decision text NOT NULL,
    comment text DEFAULT ''::text NOT NULL,
    approver_id bigint NOT NULL,
    decided_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT daily_approvals_decision_check CHECK ((decision = ANY (ARRAY['approved'::text, 'rejected'::text])))
);

CREATE SEQUENCE daily_approvals_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE daily_approvals_id_seq OWNED BY daily_approvals.id;

CREATE TABLE daily_reports (
    id bigint NOT NULL,
    user_id bigint NOT NULL,
    report_date date NOT NULL,
    achievement text,
    leftover text,
    keep text,
    problem text,
    try text,
    submitted_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE SEQUENCE daily_reports_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE daily_reports_id_seq OWNED BY daily_reports.id;

CREATE TABLE milestones (
    id bigint NOT NULL,
    project_id bigint NOT NULL,
    name text NOT NULL,
    start_date date,
    due_date date,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by_id bigint,
    -- Soft delete: tickets.milestone_id (ON DELETE SET NULL) would otherwise
    -- lose the milestone's own name/due_date/description forever the moment
    -- it's deleted, even though tickets still historically reference it.
    deleted_at timestamp with time zone,
    deleted_by_id bigint
);

CREATE SEQUENCE milestones_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE milestones_id_seq OWNED BY milestones.id;

-- In-app notifications (assignment, comments, overdue, report decisions).
-- actor_id/ticket_id are resolved to display_name/title via JOIN at read
-- time rather than snapshotted, so a rename always shows the current value.
CREATE TABLE notifications (
    id bigint NOT NULL,
    user_id bigint NOT NULL,
    notif_type text NOT NULL,
    actor_id bigint,
    ticket_id bigint,
    report_date date,
    read_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT notifications_notif_type_check CHECK ((notif_type = ANY (ARRAY['ticket_assigned'::text, 'ticket_overdue'::text, 'ticket_commented'::text, 'report_approved'::text, 'report_rejected'::text])))
);

CREATE SEQUENCE notifications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE notifications_id_seq OWNED BY notifications.id;

CREATE TABLE monthly_closures (
    id bigint NOT NULL,
    user_id bigint NOT NULL,
    year integer NOT NULL,
    month smallint NOT NULL,
    action text NOT NULL,
    actor_id bigint NOT NULL,
    acted_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT monthly_closures_action_check CHECK ((action = ANY (ARRAY['closed'::text, 'reopened'::text]))),
    CONSTRAINT monthly_closures_month_check CHECK (((month >= 1) AND (month <= 12)))
);

CREATE SEQUENCE monthly_closures_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE monthly_closures_id_seq OWNED BY monthly_closures.id;

CREATE TABLE project_members (
    id bigint NOT NULL,
    project_id bigint NOT NULL,
    user_id bigint NOT NULL,
    role text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    added_by_id bigint,
    CONSTRAINT project_members_role_check CHECK ((role = ANY (ARRAY['manager'::text, 'member'::text, 'viewer'::text])))
);

CREATE SEQUENCE project_members_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE project_members_id_seq OWNED BY project_members.id;

CREATE TABLE projects (
    id bigint NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    archived_at timestamp with time zone,
    workflow_id bigint NOT NULL,
    created_by_id bigint
);

CREATE SEQUENCE projects_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE projects_id_seq OWNED BY projects.id;

CREATE TABLE sessions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id bigint NOT NULL,
    csrf_token text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE system_settings (
    id smallint DEFAULT 1 NOT NULL,
    katex_enabled boolean DEFAULT true NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    default_timezone text DEFAULT 'Asia/Tokyo'::text NOT NULL,
    maintenance_mode boolean DEFAULT false NOT NULL,
    CONSTRAINT system_settings_id_check CHECK ((id = 1))
);

CREATE TABLE ticket_activity_log (
    id bigint NOT NULL,
    ticket_id bigint NOT NULL,
    user_id bigint NOT NULL,
    activity_type text NOT NULL,
    occurred_at timestamp with time zone DEFAULT now() NOT NULL,
    from_status_id bigint,
    to_status_id bigint,
    from_assignee_id bigint,
    to_assignee_id bigint,
    field_name text,
    from_text text,
    to_text text,
    CONSTRAINT ticket_activity_log_activity_type_check CHECK ((activity_type = ANY (ARRAY['comment'::text, 'status_change'::text, 'assignee_change'::text, 'field_update'::text, 'created'::text])))
);

CREATE SEQUENCE ticket_activity_log_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE ticket_activity_log_id_seq OWNED BY ticket_activity_log.id;

CREATE TABLE ticket_comments (
    id bigint NOT NULL,
    ticket_id bigint NOT NULL,
    author_id bigint NOT NULL,
    body text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);

CREATE SEQUENCE ticket_comments_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE ticket_comments_id_seq OWNED BY ticket_comments.id;

CREATE TABLE ticket_dependencies (
    id bigint NOT NULL,
    predecessor_ticket_id bigint NOT NULL,
    successor_ticket_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    -- Nullable: rows created before this column existed have no known
    -- creator, and the timeline falls back to a generic message for those.
    created_by_id bigint,
    -- Soft delete: removing a dependency used to physically delete this row,
    -- which erased the very created_at/created_by_id the ticket timeline
    -- relies on to show "X added this dependency" - that entry would vanish
    -- the moment the link was removed. Kept around (with the uniqueness
    -- constraint below made partial) so the timeline can still show both the
    -- add and the remove.
    deleted_at timestamp with time zone,
    deleted_by_id bigint,
    CONSTRAINT ticket_dependencies_check CHECK ((predecessor_ticket_id <> successor_ticket_id))
);

CREATE SEQUENCE ticket_dependencies_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE ticket_dependencies_id_seq OWNED BY ticket_dependencies.id;

CREATE TABLE tickets (
    id bigint NOT NULL,
    project_id bigint NOT NULL,
    parent_ticket_id bigint,
    reporter_id bigint NOT NULL,
    assignee_id bigint,
    title text NOT NULL,
    description text,
    start_date date,
    end_date date,
    due_date date,
    milestone_id bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    status_id bigint NOT NULL,
    -- Soft delete: rows are never physically removed anymore, so that
    -- ticket_activity_log/ticket_comments (ON DELETE CASCADE) survive a
    -- "delete" and stay available for traceability. Every read query goes
    -- through TICKET_FROM_JOIN, which filters this out.
    deleted_at timestamp with time zone
);

CREATE SEQUENCE tickets_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE tickets_id_seq OWNED BY tickets.id;

CREATE TABLE user_approvers (
    id bigint NOT NULL,
    user_id bigint NOT NULL,
    approver_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    assigned_by_id bigint
);

CREATE SEQUENCE user_approvers_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE user_approvers_id_seq OWNED BY user_approvers.id;

CREATE TABLE users (
    id bigint NOT NULL,
    email text NOT NULL,
    password_hash text NOT NULL,
    display_name text NOT NULL,
    role text DEFAULT 'general'::text NOT NULL,
    status text DEFAULT 'active'::text NOT NULL,
    locale text DEFAULT 'ja'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    avatar_data bytea,
    avatar_content_type text,
    avatar_updated_at timestamp with time zone,
    is_approver boolean DEFAULT false NOT NULL,
    timezone text,
    created_by_id bigint,
    CONSTRAINT users_locale_check CHECK ((locale = ANY (ARRAY['ja'::text, 'en'::text]))),
    CONSTRAINT users_role_check CHECK ((role = ANY (ARRAY['admin'::text, 'general'::text]))),
    CONSTRAINT users_status_check CHECK ((status = ANY (ARRAY['active'::text, 'suspended'::text, 'deleted'::text])))
);

CREATE SEQUENCE users_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE users_id_seq OWNED BY users.id;

CREATE TABLE work_sessions (
    id bigint NOT NULL,
    user_id bigint NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    ended_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE SEQUENCE work_sessions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE work_sessions_id_seq OWNED BY work_sessions.id;

CREATE TABLE workflow_statuses (
    id bigint NOT NULL,
    workflow_id bigint NOT NULL,
    key text NOT NULL,
    label text NOT NULL,
    category text NOT NULL,
    "position" integer NOT NULL,
    is_initial boolean DEFAULT false NOT NULL,
    created_by_id bigint,
    CONSTRAINT workflow_statuses_category_check CHECK ((category = ANY (ARRAY['todo'::text, 'in_progress'::text, 'done'::text])))
);

CREATE SEQUENCE workflow_statuses_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE workflow_statuses_id_seq OWNED BY workflow_statuses.id;

CREATE TABLE workflow_transitions (
    id bigint NOT NULL,
    workflow_id bigint NOT NULL,
    from_status_id bigint NOT NULL,
    to_status_id bigint NOT NULL,
    created_by_id bigint,
    CONSTRAINT workflow_transitions_check CHECK ((from_status_id <> to_status_id))
);

CREATE SEQUENCE workflow_transitions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE workflow_transitions_id_seq OWNED BY workflow_transitions.id;

CREATE TABLE workflows (
    id bigint NOT NULL,
    name text NOT NULL,
    is_default boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by_id bigint
);

CREATE SEQUENCE workflows_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE workflows_id_seq OWNED BY workflows.id;

ALTER TABLE ONLY attachments ALTER COLUMN id SET DEFAULT nextval('attachments_id_seq'::regclass);

ALTER TABLE ONLY audit_log ALTER COLUMN id SET DEFAULT nextval('audit_log_id_seq'::regclass);

ALTER TABLE ONLY daily_approvals ALTER COLUMN id SET DEFAULT nextval('daily_approvals_id_seq'::regclass);

ALTER TABLE ONLY daily_reports ALTER COLUMN id SET DEFAULT nextval('daily_reports_id_seq'::regclass);

ALTER TABLE ONLY milestones ALTER COLUMN id SET DEFAULT nextval('milestones_id_seq'::regclass);

ALTER TABLE ONLY monthly_closures ALTER COLUMN id SET DEFAULT nextval('monthly_closures_id_seq'::regclass);

ALTER TABLE ONLY notifications ALTER COLUMN id SET DEFAULT nextval('notifications_id_seq'::regclass);

ALTER TABLE ONLY project_members ALTER COLUMN id SET DEFAULT nextval('project_members_id_seq'::regclass);

ALTER TABLE ONLY projects ALTER COLUMN id SET DEFAULT nextval('projects_id_seq'::regclass);

ALTER TABLE ONLY ticket_activity_log ALTER COLUMN id SET DEFAULT nextval('ticket_activity_log_id_seq'::regclass);

ALTER TABLE ONLY ticket_comments ALTER COLUMN id SET DEFAULT nextval('ticket_comments_id_seq'::regclass);

ALTER TABLE ONLY ticket_dependencies ALTER COLUMN id SET DEFAULT nextval('ticket_dependencies_id_seq'::regclass);

ALTER TABLE ONLY tickets ALTER COLUMN id SET DEFAULT nextval('tickets_id_seq'::regclass);

ALTER TABLE ONLY user_approvers ALTER COLUMN id SET DEFAULT nextval('user_approvers_id_seq'::regclass);

ALTER TABLE ONLY users ALTER COLUMN id SET DEFAULT nextval('users_id_seq'::regclass);

ALTER TABLE ONLY work_sessions ALTER COLUMN id SET DEFAULT nextval('work_sessions_id_seq'::regclass);

ALTER TABLE ONLY workflow_statuses ALTER COLUMN id SET DEFAULT nextval('workflow_statuses_id_seq'::regclass);

ALTER TABLE ONLY workflow_transitions ALTER COLUMN id SET DEFAULT nextval('workflow_transitions_id_seq'::regclass);

ALTER TABLE ONLY workflows ALTER COLUMN id SET DEFAULT nextval('workflows_id_seq'::regclass);

ALTER TABLE ONLY attachments
    ADD CONSTRAINT attachments_pkey PRIMARY KEY (id);

ALTER TABLE ONLY audit_log
    ADD CONSTRAINT audit_log_pkey PRIMARY KEY (id);

ALTER TABLE ONLY daily_approvals
    ADD CONSTRAINT daily_approvals_pkey PRIMARY KEY (id);

ALTER TABLE ONLY daily_reports
    ADD CONSTRAINT daily_reports_pkey PRIMARY KEY (id);

ALTER TABLE ONLY daily_reports
    ADD CONSTRAINT daily_reports_user_id_report_date_key UNIQUE (user_id, report_date);

ALTER TABLE ONLY milestones
    ADD CONSTRAINT milestones_pkey PRIMARY KEY (id);

ALTER TABLE ONLY monthly_closures
    ADD CONSTRAINT monthly_closures_pkey PRIMARY KEY (id);

ALTER TABLE ONLY notifications
    ADD CONSTRAINT notifications_pkey PRIMARY KEY (id);

ALTER TABLE ONLY project_members
    ADD CONSTRAINT project_members_pkey PRIMARY KEY (id);

ALTER TABLE ONLY project_members
    ADD CONSTRAINT project_members_project_id_user_id_key UNIQUE (project_id, user_id);

ALTER TABLE ONLY projects
    ADD CONSTRAINT projects_name_key UNIQUE (name);

ALTER TABLE ONLY projects
    ADD CONSTRAINT projects_pkey PRIMARY KEY (id);

ALTER TABLE ONLY sessions
    ADD CONSTRAINT sessions_pkey PRIMARY KEY (id);

ALTER TABLE ONLY system_settings
    ADD CONSTRAINT system_settings_pkey PRIMARY KEY (id);

ALTER TABLE ONLY ticket_activity_log
    ADD CONSTRAINT ticket_activity_log_pkey PRIMARY KEY (id);

ALTER TABLE ONLY ticket_comments
    ADD CONSTRAINT ticket_comments_pkey PRIMARY KEY (id);

ALTER TABLE ONLY ticket_dependencies
    ADD CONSTRAINT ticket_dependencies_pkey PRIMARY KEY (id);

-- Partial (not a table-level UNIQUE constraint): only currently-active links
-- need to be unique, so the same pair of tickets can be re-linked after a
-- previous link between them was soft-deleted.
CREATE UNIQUE INDEX ticket_dependencies_active_pair_key ON ticket_dependencies (predecessor_ticket_id, successor_ticket_id) WHERE deleted_at IS NULL;

ALTER TABLE ONLY tickets
    ADD CONSTRAINT tickets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY user_approvers
    ADD CONSTRAINT user_approvers_pkey PRIMARY KEY (id);

ALTER TABLE ONLY user_approvers
    ADD CONSTRAINT user_approvers_user_id_approver_id_key UNIQUE (user_id, approver_id);

ALTER TABLE ONLY users
    ADD CONSTRAINT users_email_key UNIQUE (email);

ALTER TABLE ONLY users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);

ALTER TABLE ONLY work_sessions
    ADD CONSTRAINT work_sessions_pkey PRIMARY KEY (id);

ALTER TABLE ONLY workflow_statuses
    ADD CONSTRAINT workflow_statuses_pkey PRIMARY KEY (id);

ALTER TABLE ONLY workflow_statuses
    ADD CONSTRAINT workflow_statuses_workflow_id_key_key UNIQUE (workflow_id, key);

ALTER TABLE ONLY workflow_transitions
    ADD CONSTRAINT workflow_transitions_from_status_id_to_status_id_key UNIQUE (from_status_id, to_status_id);

ALTER TABLE ONLY workflow_transitions
    ADD CONSTRAINT workflow_transitions_pkey PRIMARY KEY (id);

ALTER TABLE ONLY workflows
    ADD CONSTRAINT workflows_name_key UNIQUE (name);

ALTER TABLE ONLY workflows
    ADD CONSTRAINT workflows_pkey PRIMARY KEY (id);

CREATE UNIQUE INDEX attachments_sha256_key ON attachments USING btree (sha256);

CREATE INDEX idx_daily_approvals_user_date ON daily_approvals USING btree (user_id, report_date, decided_at DESC);

CREATE INDEX idx_daily_reports_user_id_report_date ON daily_reports USING btree (user_id, report_date);

CREATE INDEX idx_milestones_project_id ON milestones USING btree (project_id);

CREATE INDEX idx_monthly_closures_period ON monthly_closures USING btree (user_id, year, month, acted_at DESC);

CREATE INDEX idx_notifications_user_id_created_at ON notifications USING btree (user_id, created_at DESC);

CREATE INDEX idx_notifications_user_id_unread ON notifications USING btree (user_id) WHERE (read_at IS NULL);

-- Caps ticket_overdue notifications at one per ticket ever, regardless of
-- how many times the periodic sweep (see main.rs) finds it still overdue.
CREATE UNIQUE INDEX notifications_ticket_overdue_once ON notifications USING btree (ticket_id) WHERE (notif_type = 'ticket_overdue'::text);

CREATE INDEX idx_project_members_project_id ON project_members USING btree (project_id);

CREATE INDEX idx_project_members_user_id ON project_members USING btree (user_id);

CREATE INDEX idx_projects_archived_at ON projects USING btree (archived_at);

CREATE INDEX idx_sessions_expires_at ON sessions USING btree (expires_at);

CREATE INDEX idx_sessions_user_id ON sessions USING btree (user_id);

CREATE INDEX idx_ticket_activity_log_ticket_id_occurred_at ON ticket_activity_log USING btree (ticket_id, occurred_at);

CREATE INDEX idx_ticket_activity_log_user_id_occurred_at ON ticket_activity_log USING btree (user_id, occurred_at);

CREATE INDEX idx_ticket_comments_ticket_id_created_at ON ticket_comments USING btree (ticket_id, created_at);

CREATE INDEX idx_ticket_dependencies_predecessor ON ticket_dependencies USING btree (predecessor_ticket_id);

CREATE INDEX idx_ticket_dependencies_successor ON ticket_dependencies USING btree (successor_ticket_id);

CREATE INDEX idx_tickets_assignee_id ON tickets USING btree (assignee_id);

CREATE INDEX idx_tickets_parent_ticket_id ON tickets USING btree (parent_ticket_id);

CREATE INDEX idx_tickets_project_id ON tickets USING btree (project_id);

CREATE INDEX idx_user_approvers_approver_id ON user_approvers USING btree (approver_id);

CREATE INDEX idx_user_approvers_user_id ON user_approvers USING btree (user_id);

CREATE INDEX idx_users_email ON users USING btree (email) WHERE (status <> 'deleted'::text);

CREATE INDEX idx_users_status ON users USING btree (status);

CREATE UNIQUE INDEX idx_work_sessions_one_open_per_user ON work_sessions USING btree (user_id) WHERE (ended_at IS NULL);

CREATE INDEX idx_work_sessions_user_id_started_at ON work_sessions USING btree (user_id, started_at);

CREATE UNIQUE INDEX idx_workflow_statuses_single_initial ON workflow_statuses USING btree (workflow_id) WHERE is_initial;

CREATE UNIQUE INDEX idx_workflows_single_default ON workflows USING btree (is_default) WHERE is_default;

ALTER TABLE ONLY attachments
    ADD CONSTRAINT attachments_uploaded_by_id_fkey FOREIGN KEY (uploaded_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY audit_log
    ADD CONSTRAINT audit_log_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES users(id) ON DELETE RESTRICT;

ALTER TABLE ONLY daily_approvals
    ADD CONSTRAINT daily_approvals_approver_id_fkey FOREIGN KEY (approver_id) REFERENCES users(id) ON DELETE RESTRICT;

ALTER TABLE ONLY daily_approvals
    ADD CONSTRAINT daily_approvals_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY daily_reports
    ADD CONSTRAINT daily_reports_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY tickets
    ADD CONSTRAINT fk_tickets_milestone FOREIGN KEY (milestone_id) REFERENCES milestones(id) ON DELETE SET NULL;

ALTER TABLE ONLY milestones
    ADD CONSTRAINT milestones_created_by_id_fkey FOREIGN KEY (created_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY milestones
    ADD CONSTRAINT milestones_project_id_fkey FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;

ALTER TABLE ONLY monthly_closures
    ADD CONSTRAINT monthly_closures_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES users(id) ON DELETE RESTRICT;

ALTER TABLE ONLY monthly_closures
    ADD CONSTRAINT monthly_closures_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY notifications
    ADD CONSTRAINT notifications_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY notifications
    ADD CONSTRAINT notifications_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY notifications
    ADD CONSTRAINT notifications_ticket_id_fkey FOREIGN KEY (ticket_id) REFERENCES tickets(id) ON DELETE CASCADE;

ALTER TABLE ONLY project_members
    ADD CONSTRAINT project_members_added_by_id_fkey FOREIGN KEY (added_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY project_members
    ADD CONSTRAINT project_members_project_id_fkey FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;

ALTER TABLE ONLY project_members
    ADD CONSTRAINT project_members_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY projects
    ADD CONSTRAINT projects_created_by_id_fkey FOREIGN KEY (created_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY projects
    ADD CONSTRAINT projects_workflow_id_fkey FOREIGN KEY (workflow_id) REFERENCES workflows(id);

ALTER TABLE ONLY sessions
    ADD CONSTRAINT sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY ticket_activity_log
    ADD CONSTRAINT ticket_activity_log_from_assignee_id_fkey FOREIGN KEY (from_assignee_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY ticket_activity_log
    ADD CONSTRAINT ticket_activity_log_from_status_id_fkey FOREIGN KEY (from_status_id) REFERENCES workflow_statuses(id) ON DELETE SET NULL;

ALTER TABLE ONLY ticket_activity_log
    ADD CONSTRAINT ticket_activity_log_ticket_id_fkey FOREIGN KEY (ticket_id) REFERENCES tickets(id) ON DELETE CASCADE;

ALTER TABLE ONLY ticket_activity_log
    ADD CONSTRAINT ticket_activity_log_to_assignee_id_fkey FOREIGN KEY (to_assignee_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY ticket_activity_log
    ADD CONSTRAINT ticket_activity_log_to_status_id_fkey FOREIGN KEY (to_status_id) REFERENCES workflow_statuses(id) ON DELETE SET NULL;

ALTER TABLE ONLY ticket_activity_log
    ADD CONSTRAINT ticket_activity_log_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY ticket_comments
    ADD CONSTRAINT ticket_comments_author_id_fkey FOREIGN KEY (author_id) REFERENCES users(id);

ALTER TABLE ONLY ticket_comments
    ADD CONSTRAINT ticket_comments_ticket_id_fkey FOREIGN KEY (ticket_id) REFERENCES tickets(id) ON DELETE CASCADE;

ALTER TABLE ONLY ticket_dependencies
    ADD CONSTRAINT ticket_dependencies_created_by_id_fkey FOREIGN KEY (created_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY ticket_dependencies
    ADD CONSTRAINT ticket_dependencies_deleted_by_id_fkey FOREIGN KEY (deleted_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY ticket_dependencies
    ADD CONSTRAINT ticket_dependencies_predecessor_ticket_id_fkey FOREIGN KEY (predecessor_ticket_id) REFERENCES tickets(id) ON DELETE CASCADE;

ALTER TABLE ONLY ticket_dependencies
    ADD CONSTRAINT ticket_dependencies_successor_ticket_id_fkey FOREIGN KEY (successor_ticket_id) REFERENCES tickets(id) ON DELETE CASCADE;

ALTER TABLE ONLY tickets
    ADD CONSTRAINT tickets_assignee_id_fkey FOREIGN KEY (assignee_id) REFERENCES users(id);

ALTER TABLE ONLY tickets
    ADD CONSTRAINT tickets_parent_ticket_id_fkey FOREIGN KEY (parent_ticket_id) REFERENCES tickets(id) ON DELETE CASCADE;

ALTER TABLE ONLY tickets
    ADD CONSTRAINT tickets_project_id_fkey FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;

ALTER TABLE ONLY tickets
    ADD CONSTRAINT tickets_reporter_id_fkey FOREIGN KEY (reporter_id) REFERENCES users(id);

ALTER TABLE ONLY tickets
    ADD CONSTRAINT tickets_status_id_fkey FOREIGN KEY (status_id) REFERENCES workflow_statuses(id);

ALTER TABLE ONLY user_approvers
    ADD CONSTRAINT user_approvers_approver_id_fkey FOREIGN KEY (approver_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY user_approvers
    ADD CONSTRAINT user_approvers_assigned_by_id_fkey FOREIGN KEY (assigned_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY user_approvers
    ADD CONSTRAINT user_approvers_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY users
    ADD CONSTRAINT users_created_by_id_fkey FOREIGN KEY (created_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY work_sessions
    ADD CONSTRAINT work_sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY workflow_statuses
    ADD CONSTRAINT workflow_statuses_created_by_id_fkey FOREIGN KEY (created_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY workflow_statuses
    ADD CONSTRAINT workflow_statuses_workflow_id_fkey FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;

ALTER TABLE ONLY workflow_transitions
    ADD CONSTRAINT workflow_transitions_created_by_id_fkey FOREIGN KEY (created_by_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY workflow_transitions
    ADD CONSTRAINT workflow_transitions_from_status_id_fkey FOREIGN KEY (from_status_id) REFERENCES workflow_statuses(id) ON DELETE CASCADE;

ALTER TABLE ONLY workflow_transitions
    ADD CONSTRAINT workflow_transitions_to_status_id_fkey FOREIGN KEY (to_status_id) REFERENCES workflow_statuses(id) ON DELETE CASCADE;

ALTER TABLE ONLY workflow_transitions
    ADD CONSTRAINT workflow_transitions_workflow_id_fkey FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;

ALTER TABLE ONLY workflows
    ADD CONSTRAINT workflows_created_by_id_fkey FOREIGN KEY (created_by_id) REFERENCES users(id) ON DELETE SET NULL;

-- Seed data: the built-in "シンプル" (Simple) workflow with the 4 statuses
-- that used to be a hardcoded TicketStatus enum, and a ladder-shaped
-- transition graph (open <-> in_progress <-> done <-> closed, with "closed"
-- also reachable in a single hop from any earlier status, and reopenable
-- back to "open").
INSERT INTO system_settings (id, katex_enabled, default_timezone) VALUES (1, true, 'Asia/Tokyo');

INSERT INTO workflows (name, is_default) VALUES ('シンプル', true);

INSERT INTO workflow_statuses (workflow_id, key, label, category, position, is_initial)
SELECT id, 'open', '未着手', 'todo', 0, true FROM workflows WHERE name = 'シンプル'
UNION ALL SELECT id, 'in_progress', '進行中', 'in_progress', 1, false FROM workflows WHERE name = 'シンプル'
UNION ALL SELECT id, 'done', '完了', 'done', 2, false FROM workflows WHERE name = 'シンプル'
UNION ALL SELECT id, 'closed', 'クローズ', 'done', 3, false FROM workflows WHERE name = 'シンプル';

INSERT INTO workflow_transitions (workflow_id, from_status_id, to_status_id)
SELECT w.id, s_from.id, s_to.id
FROM workflows w
JOIN workflow_statuses s_from ON s_from.workflow_id = w.id
JOIN workflow_statuses s_to ON s_to.workflow_id = w.id
WHERE w.is_default = true
  AND (s_from.key, s_to.key) IN (
    ('open', 'in_progress'),
    ('in_progress', 'open'),
    ('in_progress', 'done'),
    ('done', 'in_progress'),
    ('done', 'closed'),
    ('closed', 'done'),
    ('open', 'closed'),
    ('in_progress', 'closed'),
    ('closed', 'open')
  );
