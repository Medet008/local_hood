-- Тип уведомления
CREATE TYPE notification_type AS ENUM (
    'announcement',
    'voting',
    'bill',
    'payment',
    'guest_access',
    'maintenance',
    'security',
    'chat',
    'marketplace',
    'system'
);

-- Уведомления
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    notification_type notification_type NOT NULL,
    title VARCHAR(200) NOT NULL,
    body TEXT,

    -- Данные для навигации
    data JSONB,

    is_read BOOLEAN DEFAULT false,
    read_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_type ON notifications(notification_type);
CREATE INDEX idx_notifications_read ON notifications(is_read);
CREATE INDEX idx_notifications_created ON notifications(created_at);

-- Push токены
CREATE TABLE push_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    token TEXT NOT NULL,
    platform VARCHAR(20) NOT NULL,  -- ios, android
    device_id VARCHAR(100),

    is_active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(user_id, token)
);

CREATE INDEX idx_push_tokens_user ON push_tokens(user_id);
CREATE INDEX idx_push_tokens_active ON push_tokens(is_active);
