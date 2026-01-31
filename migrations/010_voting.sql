-- Тип голосования
CREATE TYPE voting_type AS ENUM ('single_choice', 'multiple_choice', 'yes_no');

-- Статус голосования
CREATE TYPE voting_status AS ENUM ('draft', 'active', 'closed', 'cancelled');

-- Голосования
CREATE TABLE votings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id) ON DELETE CASCADE,
    osi_id UUID REFERENCES osi(id),

    title VARCHAR(200) NOT NULL,
    description TEXT,

    voting_type voting_type DEFAULT 'single_choice',
    status voting_status DEFAULT 'draft',

    -- Требования
    requires_owner BOOLEAN DEFAULT true,  -- Только владельцы
    quorum_percent INT DEFAULT 51,  -- Кворум в процентах

    -- Время
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL,

    -- Автор
    created_by UUID NOT NULL REFERENCES users(id),

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_votings_complex ON votings(complex_id);
CREATE INDEX idx_votings_status ON votings(status);
CREATE INDEX idx_votings_dates ON votings(starts_at, ends_at);

-- Варианты ответов
CREATE TABLE voting_options (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    voting_id UUID NOT NULL REFERENCES votings(id) ON DELETE CASCADE,

    text VARCHAR(500) NOT NULL,
    sort_order INT DEFAULT 0,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_voting_options_voting ON voting_options(voting_id);

-- Голоса
CREATE TABLE votes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    voting_id UUID NOT NULL REFERENCES votings(id) ON DELETE CASCADE,
    option_id UUID NOT NULL REFERENCES voting_options(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    apartment_id UUID REFERENCES apartments(id),

    -- Вес голоса (по площади квартиры)
    vote_weight DECIMAL(10, 4) DEFAULT 1,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(voting_id, user_id)  -- Один голос на голосование
);

CREATE INDEX idx_votes_voting ON votes(voting_id);
CREATE INDEX idx_votes_user ON votes(user_id);
CREATE INDEX idx_votes_option ON votes(option_id);

-- Документы к голосованию
CREATE TABLE voting_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    voting_id UUID NOT NULL REFERENCES votings(id) ON DELETE CASCADE,

    title VARCHAR(200) NOT NULL,
    file_url TEXT NOT NULL,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_voting_documents_voting ON voting_documents(voting_id);
