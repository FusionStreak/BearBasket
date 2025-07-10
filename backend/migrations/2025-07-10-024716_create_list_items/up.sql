-- Your SQL goes here
CREATE TABLE list_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    list_id UUID REFERENCES lists(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    quantity TEXT DEFAULT '1',
    checked BOOLEAN DEFAULT false,
    position INTEGER, -- for custom ordering
    added_by UUID REFERENCES users(id),
    created_at TIMESTAMP DEFAULT now(),
    updated_at TIMESTAMP DEFAULT now()
);