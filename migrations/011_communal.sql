-- Типы услуг
CREATE TYPE utility_type AS ENUM (
    'electricity',
    'cold_water',
    'hot_water',
    'heating',
    'gas',
    'maintenance',
    'garbage',
    'elevator',
    'intercom',
    'parking',
    'security',
    'other'
);

-- Счетчики
CREATE TABLE meters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    apartment_id UUID NOT NULL REFERENCES apartments(id) ON DELETE CASCADE,
    utility_type utility_type NOT NULL,

    serial_number VARCHAR(50),
    installation_date DATE,
    verification_date DATE,
    next_verification_date DATE,

    is_active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_meters_apartment ON meters(apartment_id);
CREATE INDEX idx_meters_type ON meters(utility_type);

-- Показания счетчиков
CREATE TABLE meter_readings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    meter_id UUID NOT NULL REFERENCES meters(id) ON DELETE CASCADE,
    apartment_id UUID NOT NULL REFERENCES apartments(id),

    value DECIMAL(12, 3) NOT NULL,
    previous_value DECIMAL(12, 3),
    consumption DECIMAL(12, 3),

    reading_date DATE NOT NULL,
    submitted_by UUID REFERENCES users(id),

    -- Фото показаний
    photo_url TEXT,

    is_verified BOOLEAN DEFAULT false,
    verified_by UUID REFERENCES users(id),

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_meter_readings_meter ON meter_readings(meter_id);
CREATE INDEX idx_meter_readings_apartment ON meter_readings(apartment_id);
CREATE INDEX idx_meter_readings_date ON meter_readings(reading_date);

-- Статус счёта
CREATE TYPE bill_status AS ENUM ('pending', 'paid', 'overdue', 'cancelled');

-- Счета на оплату
CREATE TABLE bills (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    apartment_id UUID NOT NULL REFERENCES apartments(id),
    complex_id UUID NOT NULL REFERENCES complexes(id),

    -- Период
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,

    -- Сумма
    amount DECIMAL(12, 2) NOT NULL,
    debt DECIMAL(12, 2) DEFAULT 0,
    penalty DECIMAL(12, 2) DEFAULT 0,
    total_amount DECIMAL(12, 2) NOT NULL,

    -- Оплата
    status bill_status DEFAULT 'pending',
    due_date DATE NOT NULL,
    paid_at TIMESTAMPTZ,
    paid_amount DECIMAL(12, 2),

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_bills_apartment ON bills(apartment_id);
CREATE INDEX idx_bills_complex ON bills(complex_id);
CREATE INDEX idx_bills_status ON bills(status);
CREATE INDEX idx_bills_period ON bills(period_start, period_end);

-- Детализация счёта
CREATE TABLE bill_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bill_id UUID NOT NULL REFERENCES bills(id) ON DELETE CASCADE,

    utility_type utility_type NOT NULL,
    description VARCHAR(200),

    -- Расчёт
    quantity DECIMAL(12, 3),
    unit VARCHAR(20),  -- кВт*ч, м³, и т.д.
    rate DECIMAL(10, 4),
    amount DECIMAL(12, 2) NOT NULL,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_bill_items_bill ON bill_items(bill_id);

-- Платежи
CREATE TYPE payment_status AS ENUM ('pending', 'processing', 'completed', 'failed', 'refunded');
CREATE TYPE payment_method AS ENUM ('card', 'kaspi', 'halyk', 'bank_transfer', 'cash');

CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bill_id UUID REFERENCES bills(id),
    apartment_id UUID NOT NULL REFERENCES apartments(id),
    user_id UUID NOT NULL REFERENCES users(id),

    amount DECIMAL(12, 2) NOT NULL,
    method payment_method NOT NULL,

    status payment_status DEFAULT 'pending',

    -- Данные платежа
    external_id VARCHAR(100),
    payment_url TEXT,

    completed_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_payments_bill ON payments(bill_id);
CREATE INDEX idx_payments_apartment ON payments(apartment_id);
CREATE INDEX idx_payments_user ON payments(user_id);
CREATE INDEX idx_payments_status ON payments(status);
