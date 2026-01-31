-- Таблица ОСИ (Объединение Собственников Имущества)
CREATE TABLE osi (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id) ON DELETE CASCADE,

    name VARCHAR(200) NOT NULL,
    bin VARCHAR(12),  -- БИН организации

    -- Председатель
    chairman_id UUID REFERENCES users(id),

    -- Контакты
    phone VARCHAR(20),
    email VARCHAR(255),
    address TEXT,

    -- Банковские реквизиты
    bank_name VARCHAR(200),
    bank_bik VARCHAR(20),
    bank_account VARCHAR(30),

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(complex_id)
);

CREATE INDEX idx_osi_complex ON osi(complex_id);
CREATE INDEX idx_osi_chairman ON osi(chairman_id);

-- Позиции в совете дома
CREATE TYPE council_position AS ENUM (
    'chairman',
    'deputy_chairman',
    'secretary',
    'treasurer',
    'member'
);

-- Совет дома
CREATE TABLE council_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osi_id UUID NOT NULL REFERENCES osi(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),

    position council_position NOT NULL,
    responsibilities TEXT,

    appointed_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    is_active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_council_members_osi ON council_members(osi_id);
CREATE INDEX idx_council_members_user ON council_members(user_id);

-- Работники ОСИ
CREATE TYPE worker_role AS ENUM (
    'accountant',
    'manager',
    'guard',
    'cleaner',
    'plumber',
    'electrician',
    'other'
);

CREATE TABLE osi_workers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osi_id UUID NOT NULL REFERENCES osi(id) ON DELETE CASCADE,

    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    middle_name VARCHAR(100),
    phone VARCHAR(20),

    role worker_role NOT NULL,
    position_title VARCHAR(100),

    salary DECIMAL(12, 2),
    hired_at DATE,

    is_active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_osi_workers_osi ON osi_workers(osi_id);

-- Документы ОСИ
CREATE TYPE document_type AS ENUM (
    'charter',
    'protocol',
    'contract',
    'report',
    'act',
    'other'
);

CREATE TABLE osi_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osi_id UUID NOT NULL REFERENCES osi(id) ON DELETE CASCADE,

    title VARCHAR(200) NOT NULL,
    description TEXT,
    document_type document_type NOT NULL,
    file_url TEXT NOT NULL,
    file_size INT,

    uploaded_by UUID NOT NULL REFERENCES users(id),

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_osi_documents_osi ON osi_documents(osi_id);
CREATE INDEX idx_osi_documents_type ON osi_documents(document_type);

-- Заявки на должность председателя
CREATE TYPE chairman_application_status AS ENUM ('pending', 'approved', 'rejected');

CREATE TABLE chairman_applications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    complex_id UUID NOT NULL REFERENCES complexes(id),

    -- Документы
    document_url TEXT,
    motivation TEXT,

    -- Статус
    status chairman_application_status DEFAULT 'pending',
    reviewed_by UUID REFERENCES users(id),
    reviewed_at TIMESTAMPTZ,
    rejection_reason TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_chairman_applications_user ON chairman_applications(user_id);
CREATE INDEX idx_chairman_applications_complex ON chairman_applications(complex_id);
CREATE INDEX idx_chairman_applications_status ON chairman_applications(status);
