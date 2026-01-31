-- Категории объявлений
CREATE TYPE announcement_category AS ENUM (
    'general',
    'maintenance',
    'emergency',
    'event',
    'financial',
    'voting'
);

-- Приоритет объявлений
CREATE TYPE announcement_priority AS ENUM ('low', 'normal', 'high', 'urgent');

-- Объявления
CREATE TABLE announcements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id) ON DELETE CASCADE,

    title VARCHAR(200) NOT NULL,
    content TEXT NOT NULL,

    category announcement_category DEFAULT 'general',
    priority announcement_priority DEFAULT 'normal',

    -- Вложения
    image_url TEXT,

    -- Публикация
    is_published BOOLEAN DEFAULT true,
    published_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    -- Автор
    author_id UUID NOT NULL REFERENCES users(id),

    -- Статистика
    views_count INT DEFAULT 0,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_announcements_complex ON announcements(complex_id);
CREATE INDEX idx_announcements_published ON announcements(is_published, published_at);
CREATE INDEX idx_announcements_category ON announcements(category);
CREATE INDEX idx_announcements_priority ON announcements(priority);

-- Прочитанные объявления
CREATE TABLE announcement_reads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    announcement_id UUID NOT NULL REFERENCES announcements(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    read_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(announcement_id, user_id)
);

CREATE INDEX idx_announcement_reads_user ON announcement_reads(user_id);
