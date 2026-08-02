CREATE TABLE api_tokens (
    id uuid PRIMARY KEY,
    user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash text NOT NULL UNIQUE,
    name text NOT NULL,
    scopes text[] NOT NULL DEFAULT ARRAY['read']::text[],
    created_at timestamptz NOT NULL DEFAULT now(),
    last_used_at timestamptz,
    revoked_at timestamptz
);

CREATE INDEX api_tokens_user_id_idx ON api_tokens(user_id);
CREATE INDEX api_tokens_active_hash_idx ON api_tokens(token_hash) WHERE revoked_at IS NULL;
