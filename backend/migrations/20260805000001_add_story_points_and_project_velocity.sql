ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS hours_per_story_point double precision NOT NULL DEFAULT 1.0;

ALTER TABLE tickets
    ADD COLUMN IF NOT EXISTS story_points integer;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'tickets_story_points_check'
    ) THEN
        ALTER TABLE tickets
            ADD CONSTRAINT tickets_story_points_check
            CHECK ((story_points = ANY (ARRAY[1, 2, 3, 5, 8, 13, 21])) OR story_points IS NULL);
    END IF;
END $$;
