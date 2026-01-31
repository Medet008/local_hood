-- Тип чата
CREATE TYPE chat_type AS ENUM ('complex', 'building', 'private', 'support');

-- Чаты
CREATE TABLE chats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID REFERENCES complexes(id),

    chat_type chat_type NOT NULL,
    name VARCHAR(200),

    -- Для приватных чатов
    is_private BOOLEAN DEFAULT false,

    created_by UUID REFERENCES users(id),

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_chats_complex ON chats(complex_id);
CREATE INDEX idx_chats_type ON chats(chat_type);

-- Участники чата
CREATE TABLE chat_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    is_admin BOOLEAN DEFAULT false,
    is_muted BOOLEAN DEFAULT false,

    joined_at TIMESTAMPTZ DEFAULT NOW(),
    last_read_at TIMESTAMPTZ,

    UNIQUE(chat_id, user_id)
);

CREATE INDEX idx_chat_members_chat ON chat_members(chat_id);
CREATE INDEX idx_chat_members_user ON chat_members(user_id);

-- Сообщения
CREATE TABLE chat_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id),

    content TEXT NOT NULL,

    -- Вложения
    attachment_url TEXT,
    attachment_type VARCHAR(20),  -- image, file, voice

    -- Ответ на сообщение
    reply_to_id UUID REFERENCES chat_messages(id),

    is_edited BOOLEAN DEFAULT false,
    edited_at TIMESTAMPTZ,

    is_deleted BOOLEAN DEFAULT false,
    deleted_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_chat_messages_chat ON chat_messages(chat_id);
CREATE INDEX idx_chat_messages_sender ON chat_messages(sender_id);
CREATE INDEX idx_chat_messages_created ON chat_messages(created_at);

-- Прочитанные сообщения (для отслеживания непрочитанных)
CREATE TABLE message_reads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES chat_messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    read_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(message_id, user_id)
);

CREATE INDEX idx_message_reads_message ON message_reads(message_id);
CREATE INDEX idx_message_reads_user ON message_reads(user_id);
