-- Your SQL goes here
CREATE TABLE shared_list_members (
    list_id UUID REFERENCES lists(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role TEXT DEFAULT 'editor', -- editor, viewer, etc.
    added_at TIMESTAMP DEFAULT now(),
    PRIMARY KEY (list_id, user_id)
);