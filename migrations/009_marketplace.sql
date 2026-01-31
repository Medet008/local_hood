-- Категории маркетплейса
CREATE TABLE marketplace_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    name_kz VARCHAR(100),
    slug VARCHAR(100) UNIQUE NOT NULL,
    icon VARCHAR(50),
    parent_id UUID REFERENCES marketplace_categories(id),
    sort_order INT DEFAULT 0,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Вставка базовых категорий
INSERT INTO marketplace_categories (id, name, name_kz, slug, icon, sort_order) VALUES
(gen_random_uuid(), 'Мебель', 'Жиһаз', 'furniture', 'sofa', 1),
(gen_random_uuid(), 'Электроника', 'Электроника', 'electronics', 'tv', 2),
(gen_random_uuid(), 'Одежда', 'Киім', 'clothing', 'shirt', 3),
(gen_random_uuid(), 'Детские товары', 'Балаларға арналған тауарлар', 'kids', 'baby', 4),
(gen_random_uuid(), 'Спорт', 'Спорт', 'sports', 'dumbbell', 5),
(gen_random_uuid(), 'Книги', 'Кітаптар', 'books', 'book', 6),
(gen_random_uuid(), 'Бытовая техника', 'Тұрмыстық техника', 'appliances', 'washing-machine', 7),
(gen_random_uuid(), 'Растения', 'Өсімдіктер', 'plants', 'plant', 8),
(gen_random_uuid(), 'Услуги', 'Қызметтер', 'services', 'wrench', 9),
(gen_random_uuid(), 'Другое', 'Басқа', 'other', 'more', 10);

-- Статус объявления
CREATE TYPE listing_status AS ENUM ('draft', 'active', 'sold', 'reserved', 'archived');

-- Объявления маркетплейса
CREATE TABLE marketplace_listings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    complex_id UUID NOT NULL REFERENCES complexes(id),
    seller_id UUID NOT NULL REFERENCES users(id),
    category_id UUID NOT NULL REFERENCES marketplace_categories(id),

    title VARCHAR(200) NOT NULL,
    description TEXT,
    price DECIMAL(12, 2) NOT NULL,
    is_negotiable BOOLEAN DEFAULT false,
    is_free BOOLEAN DEFAULT false,

    -- Состояние
    condition VARCHAR(50),  -- new, like_new, good, fair

    status listing_status DEFAULT 'active',

    -- Статистика
    views_count INT DEFAULT 0,
    favorites_count INT DEFAULT 0,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_listings_complex ON marketplace_listings(complex_id);
CREATE INDEX idx_listings_seller ON marketplace_listings(seller_id);
CREATE INDEX idx_listings_category ON marketplace_listings(category_id);
CREATE INDEX idx_listings_status ON marketplace_listings(status);
CREATE INDEX idx_listings_price ON marketplace_listings(price);

-- Фотографии объявлений
CREATE TABLE listing_photos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES marketplace_listings(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    is_main BOOLEAN DEFAULT false,
    sort_order INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_listing_photos_listing ON listing_photos(listing_id);

-- Избранное
CREATE TABLE listing_favorites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES marketplace_listings(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(listing_id, user_id)
);

CREATE INDEX idx_listing_favorites_user ON listing_favorites(user_id);

-- Сообщения по объявлениям
CREATE TABLE listing_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES marketplace_listings(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id),
    recipient_id UUID NOT NULL REFERENCES users(id),

    message TEXT NOT NULL,
    is_read BOOLEAN DEFAULT false,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_listing_messages_listing ON listing_messages(listing_id);
CREATE INDEX idx_listing_messages_sender ON listing_messages(sender_id);
CREATE INDEX idx_listing_messages_recipient ON listing_messages(recipient_id);
