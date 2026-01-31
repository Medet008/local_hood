-- Статус ЖК
CREATE TYPE complex_status AS ENUM ('pending', 'active', 'inactive');

-- Таблица жилых комплексов
CREATE TABLE complexes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    city_id VARCHAR(50) NOT NULL REFERENCES cities(id),
    address_id UUID REFERENCES addresses(id),
    name VARCHAR(200) NOT NULL,
    description TEXT,

    -- Характеристики
    buildings_count INT DEFAULT 1,
    floors_count INT,
    apartments_count INT,
    year_built INT,

    -- Удобства
    has_parking BOOLEAN DEFAULT false,
    has_underground_parking BOOLEAN DEFAULT false,
    has_playground BOOLEAN DEFAULT false,
    has_gym BOOLEAN DEFAULT false,
    has_concierge BOOLEAN DEFAULT false,
    has_security BOOLEAN DEFAULT false,
    has_cctv BOOLEAN DEFAULT false,

    -- Статус
    status complex_status DEFAULT 'pending',

    -- Верификация
    verified_at TIMESTAMPTZ,
    verified_by UUID REFERENCES users(id),

    -- Создатель
    created_by UUID REFERENCES users(id),

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_complexes_city ON complexes(city_id);
CREATE INDEX idx_complexes_status ON complexes(status);
CREATE INDEX idx_complexes_address ON complexes(address_id);

-- Фотографии ЖК
CREATE TABLE complex_photos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    is_main BOOLEAN DEFAULT false,
    sort_order INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_complex_photos_complex ON complex_photos(complex_id);
