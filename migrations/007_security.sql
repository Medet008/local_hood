-- Гостевой доступ (шлагбаум)
CREATE TYPE guest_access_status AS ENUM ('pending', 'active', 'expired', 'completed', 'cancelled');

CREATE TABLE guest_access (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id),
    created_by UUID NOT NULL REFERENCES users(id),

    -- Данные гостя
    guest_name VARCHAR(100),
    guest_phone VARCHAR(20),
    vehicle_number VARCHAR(20),

    -- Код доступа
    access_code VARCHAR(10) UNIQUE NOT NULL,
    qr_code_url TEXT,

    -- Время
    duration_minutes INT DEFAULT 30,
    expires_at TIMESTAMPTZ NOT NULL,

    -- Въезд/выезд
    entered_at TIMESTAMPTZ,
    exited_at TIMESTAMPTZ,

    -- Статус
    status guest_access_status DEFAULT 'pending',

    -- SMS уведомления
    owner_notified BOOLEAN DEFAULT false,
    chairman_notified BOOLEAN DEFAULT false,
    overstay_notified BOOLEAN DEFAULT false,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_guest_access_code ON guest_access(access_code);
CREATE INDEX idx_guest_access_complex ON guest_access(complex_id);
CREATE INDEX idx_guest_access_status ON guest_access(status);
CREATE INDEX idx_guest_access_created_by ON guest_access(created_by);
CREATE INDEX idx_guest_access_expires ON guest_access(expires_at);

-- Шлагбаумы
CREATE TABLE barriers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id),

    name VARCHAR(100) NOT NULL,
    location VARCHAR(200),

    -- Интеграция
    device_type VARCHAR(50),
    device_ip VARCHAR(45),
    device_port INT,
    api_key VARCHAR(255),

    is_active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_barriers_complex ON barriers(complex_id);

-- Логи доступа через шлагбаум
CREATE TYPE barrier_action AS ENUM ('entry', 'exit');

CREATE TABLE barrier_access_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id),
    barrier_id UUID REFERENCES barriers(id),

    user_id UUID REFERENCES users(id),
    guest_access_id UUID REFERENCES guest_access(id),

    action barrier_action NOT NULL,
    vehicle_number VARCHAR(20),

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_barrier_logs_complex ON barrier_access_logs(complex_id);
CREATE INDEX idx_barrier_logs_created ON barrier_access_logs(created_at);
CREATE INDEX idx_barrier_logs_user ON barrier_access_logs(user_id);
CREATE INDEX idx_barrier_logs_guest ON barrier_access_logs(guest_access_id);

-- Камеры видеонаблюдения
CREATE TABLE cameras (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id),

    name VARCHAR(100) NOT NULL,
    location VARCHAR(200),

    -- Подключение
    stream_url TEXT,
    cloud_provider VARCHAR(50),  -- trassir, hikvision, dahua
    cloud_camera_id VARCHAR(100),

    -- Настройки
    is_public BOOLEAN DEFAULT false,  -- Доступ для всех жителей
    requires_owner BOOLEAN DEFAULT false,  -- Только для владельцев

    is_active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_cameras_complex ON cameras(complex_id);

-- Домофоны
CREATE TABLE intercoms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id),

    name VARCHAR(100) NOT NULL,
    location VARCHAR(200),

    -- Интеграция
    device_type VARCHAR(50),
    device_id VARCHAR(100),
    sip_address VARCHAR(255),

    is_active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_intercoms_complex ON intercoms(complex_id);

-- Логи звонков домофона
CREATE TYPE intercom_call_status AS ENUM ('missed', 'answered', 'opened', 'rejected');

CREATE TABLE intercom_calls (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    intercom_id UUID NOT NULL REFERENCES intercoms(id),
    apartment_id UUID REFERENCES apartments(id),
    user_id UUID REFERENCES users(id),

    status intercom_call_status NOT NULL,
    duration_seconds INT,
    snapshot_url TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_intercom_calls_intercom ON intercom_calls(intercom_id);
CREATE INDEX idx_intercom_calls_apartment ON intercom_calls(apartment_id);
CREATE INDEX idx_intercom_calls_created ON intercom_calls(created_at);
