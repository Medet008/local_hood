-- Таблица адресов
CREATE TABLE addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    city_id VARCHAR(50) NOT NULL REFERENCES cities(id),
    district VARCHAR(200),
    street VARCHAR(200) NOT NULL,
    building VARCHAR(20) NOT NULL,
    postal_code VARCHAR(10),
    latitude DECIMAL(10, 8),
    longitude DECIMAL(11, 8),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_addresses_city ON addresses(city_id);
CREATE INDEX idx_addresses_street ON addresses(street);

-- Создаем функцию для генерации полного адреса
CREATE OR REPLACE FUNCTION get_full_address(addr addresses)
RETURNS TEXT AS $$
BEGIN
    RETURN 'г. ' || (SELECT name FROM cities WHERE id = addr.city_id) || ', ' || addr.street || ', ' || addr.building;
END;
$$ LANGUAGE plpgsql STABLE;
