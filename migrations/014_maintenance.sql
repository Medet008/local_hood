-- Категории заявок
CREATE TYPE maintenance_category AS ENUM (
    'plumbing',
    'electrical',
    'heating',
    'elevator',
    'common_area',
    'facade',
    'roof',
    'parking',
    'landscaping',
    'security',
    'other'
);

-- Статус заявки
CREATE TYPE maintenance_status AS ENUM (
    'new',
    'in_progress',
    'waiting_parts',
    'completed',
    'rejected',
    'cancelled'
);

-- Приоритет заявки
CREATE TYPE maintenance_priority AS ENUM ('low', 'normal', 'high', 'emergency');

-- Заявки на обслуживание
CREATE TABLE maintenance_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id),
    apartment_id UUID REFERENCES apartments(id),

    -- Заявитель
    requester_id UUID NOT NULL REFERENCES users(id),

    -- Детали
    category maintenance_category NOT NULL,
    title VARCHAR(200) NOT NULL,
    description TEXT,
    location VARCHAR(200),  -- "Подъезд 2, 3 этаж" или "Квартира 45"

    -- Приоритет и статус
    priority maintenance_priority DEFAULT 'normal',
    status maintenance_status DEFAULT 'new',

    -- Исполнитель
    assigned_to UUID REFERENCES osi_workers(id),
    assigned_at TIMESTAMPTZ,

    -- Завершение
    completed_at TIMESTAMPTZ,
    completion_notes TEXT,

    -- Оценка
    rating INT CHECK (rating >= 1 AND rating <= 5),
    rating_comment TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_maintenance_complex ON maintenance_requests(complex_id);
CREATE INDEX idx_maintenance_apartment ON maintenance_requests(apartment_id);
CREATE INDEX idx_maintenance_requester ON maintenance_requests(requester_id);
CREATE INDEX idx_maintenance_status ON maintenance_requests(status);
CREATE INDEX idx_maintenance_category ON maintenance_requests(category);

-- Фотографии к заявке
CREATE TABLE maintenance_photos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL REFERENCES maintenance_requests(id) ON DELETE CASCADE,

    url TEXT NOT NULL,
    is_before BOOLEAN DEFAULT true,  -- До или после работ

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_maintenance_photos_request ON maintenance_photos(request_id);

-- Комментарии к заявке
CREATE TABLE maintenance_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL REFERENCES maintenance_requests(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),

    content TEXT NOT NULL,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_maintenance_comments_request ON maintenance_comments(request_id);
