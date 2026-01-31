-- Таблица квартир
CREATE TABLE apartments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id) ON DELETE CASCADE,
    building VARCHAR(20),
    entrance VARCHAR(10),
    number VARCHAR(20) NOT NULL,
    floor INT,
    area DECIMAL(10, 2),
    rooms_count INT,

    -- Владелец и жилец
    owner_id UUID REFERENCES users(id),
    resident_id UUID REFERENCES users(id),

    -- Верификация владения
    is_ownership_verified BOOLEAN DEFAULT false,
    ownership_document_url TEXT,
    verified_at TIMESTAMPTZ,
    verified_by UUID REFERENCES users(id),

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(complex_id, building, number)
);

CREATE INDEX idx_apartments_complex ON apartments(complex_id);
CREATE INDEX idx_apartments_owner ON apartments(owner_id);
CREATE INDEX idx_apartments_resident ON apartments(resident_id);

-- Заявки на присоединение к ЖК
CREATE TYPE join_request_status AS ENUM ('pending', 'approved', 'rejected');

CREATE TABLE join_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    complex_id UUID NOT NULL REFERENCES complexes(id),
    apartment_id UUID REFERENCES apartments(id),

    -- Данные заявки
    apartment_number VARCHAR(20) NOT NULL,
    building VARCHAR(20),
    is_owner BOOLEAN DEFAULT false,

    -- Документы
    document_url TEXT,

    -- Статус
    status join_request_status DEFAULT 'pending',
    reviewed_by UUID REFERENCES users(id),
    reviewed_at TIMESTAMPTZ,
    rejection_reason TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_join_requests_user ON join_requests(user_id);
CREATE INDEX idx_join_requests_complex ON join_requests(complex_id);
CREATE INDEX idx_join_requests_status ON join_requests(status);
