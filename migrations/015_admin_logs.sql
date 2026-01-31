-- Логи административных действий
CREATE TABLE admin_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    user_id UUID REFERENCES users(id),

    action VARCHAR(100) NOT NULL,
    entity_type VARCHAR(50),
    entity_id UUID,

    old_value JSONB,
    new_value JSONB,

    ip_address VARCHAR(45),
    user_agent TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_admin_logs_user ON admin_logs(user_id);
CREATE INDEX idx_admin_logs_action ON admin_logs(action);
CREATE INDEX idx_admin_logs_entity ON admin_logs(entity_type, entity_id);
CREATE INDEX idx_admin_logs_created ON admin_logs(created_at);

-- Настройки системы
CREATE TABLE system_settings (
    key VARCHAR(100) PRIMARY KEY,
    value JSONB NOT NULL,
    description TEXT,
    updated_by UUID REFERENCES users(id),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Базовые настройки
INSERT INTO system_settings (key, value, description) VALUES
('sms_enabled', 'true', 'Включить отправку SMS'),
('guest_access_default_duration', '30', 'Длительность гостевого доступа по умолчанию (минуты)'),
('max_guest_access_duration', '240', 'Максимальная длительность гостевого доступа (минуты)'),
('marketplace_enabled', 'true', 'Включить маркетплейс'),
('voting_quorum_default', '51', 'Кворум по умолчанию (проценты)');
