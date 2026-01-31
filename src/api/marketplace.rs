use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{AppState, AuthUser};
use crate::models::{
    CategoryResponse, CreateListingRequest, ListingResponse, ListingStatus,
    ListingsQuery, MarketplaceCategory, MarketplaceListing, SellerInfo,
    SendMessageRequest, UpdateListingRequest,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/categories", get(get_categories))
        .route("/listings", get(list_listings))
        .route("/listings", post(create_listing))
        .route("/listings/:id", get(get_listing))
        .route("/listings/:id", put(update_listing))
        .route("/listings/:id", delete(delete_listing))
        .route("/listings/:id/favorite", post(toggle_favorite))
        .route("/listings/:id/message", post(send_message))
        .route("/my-listings", get(my_listings))
        .route("/favorites", get(my_favorites))
}

async fn get_user_complex(state: &AppState, user_id: Uuid) -> AppResult<Uuid> {
    let complex: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT c.id
        FROM complexes c
        JOIN apartments a ON a.complex_id = c.id
        WHERE a.owner_id = $1 OR a.resident_id = $1
        LIMIT 1
        "#
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;

    complex.map(|(id,)| id).ok_or_else(|| AppError::Forbidden)
}

async fn get_categories(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<CategoryResponse>>> {
    let categories = sqlx::query_as::<_, MarketplaceCategory>(
        "SELECT * FROM marketplace_categories WHERE is_active = true ORDER BY sort_order"
    )
    .fetch_all(&state.pool)
    .await?;

    let response: Vec<CategoryResponse> = categories.into_iter().map(CategoryResponse::from).collect();
    Ok(Json(response))
}

async fn list_listings(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<ListingsQuery>,
) -> AppResult<Json<Vec<ListingResponse>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.page.unwrap_or(0) * limit;
    let search_pattern = query.query.as_ref().map(|q| format!("%{}%", q));

    let listings = sqlx::query_as::<_, MarketplaceListing>(
        r#"
        SELECT l.* FROM marketplace_listings l
        WHERE l.complex_id = $1
          AND l.status = 'active'
          AND ($2::uuid IS NULL OR l.category_id = $2)
          AND ($3::varchar IS NULL OR l.title ILIKE $3 OR l.description ILIKE $3)
          AND ($4::decimal IS NULL OR l.price >= $4)
          AND ($5::decimal IS NULL OR l.price <= $5)
        ORDER BY l.created_at DESC
        LIMIT $6 OFFSET $7
        "#
    )
    .bind(complex_id)
    .bind(query.category.as_ref().and_then(|c| Uuid::parse_str(c).ok()))
    .bind(&search_pattern)
    .bind(&query.min_price)
    .bind(&query.max_price)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for listing in listings {
        response.push(build_listing_response(&state, &listing, auth_user.user_id).await?);
    }

    Ok(Json(response))
}

async fn build_listing_response(
    state: &AppState,
    listing: &MarketplaceListing,
    user_id: Uuid,
) -> AppResult<ListingResponse> {
    let category = sqlx::query_as::<_, MarketplaceCategory>(
        "SELECT * FROM marketplace_categories WHERE id = $1"
    )
    .bind(listing.category_id)
    .fetch_one(&state.pool)
    .await?;

    let seller: (Uuid, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT id, first_name, last_name, avatar_url FROM users WHERE id = $1"
    )
    .bind(listing.seller_id)
    .fetch_one(&state.pool)
    .await?;

    let photos: Vec<(String,)> = sqlx::query_as(
        "SELECT url FROM listing_photos WHERE listing_id = $1 ORDER BY sort_order"
    )
    .bind(listing.id)
    .fetch_all(&state.pool)
    .await?;

    let is_favorite: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM listing_favorites WHERE listing_id = $1 AND user_id = $2"
    )
    .bind(listing.id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;

    Ok(ListingResponse {
        id: listing.id,
        title: listing.title.clone(),
        description: listing.description.clone(),
        price: listing.price,
        is_negotiable: listing.is_negotiable,
        is_free: listing.is_free,
        condition: listing.condition.clone(),
        status: listing.status.clone(),
        category: CategoryResponse::from(category),
        seller: SellerInfo {
            id: seller.0,
            name: format!("{} {}", seller.1.unwrap_or_default(), seller.2.unwrap_or_default()).trim().to_string(),
            avatar_url: seller.3,
        },
        photos: photos.into_iter().map(|(url,)| url).collect(),
        views_count: listing.views_count,
        favorites_count: listing.favorites_count,
        is_favorite: is_favorite.is_some(),
        created_at: listing.created_at,
    })
}

async fn get_listing(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ListingResponse>> {
    let listing = sqlx::query_as::<_, MarketplaceListing>(
        "SELECT * FROM marketplace_listings WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Объявление не найдено".to_string()))?;

    // Увеличиваем просмотры
    sqlx::query("UPDATE marketplace_listings SET views_count = views_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    let response = build_listing_response(&state, &listing, auth_user.user_id).await?;
    Ok(Json(response))
}

async fn create_listing(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateListingRequest>,
) -> AppResult<Json<ListingResponse>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let listing = sqlx::query_as::<_, MarketplaceListing>(
        r#"
        INSERT INTO marketplace_listings (
            complex_id, seller_id, category_id, title, description,
            price, is_negotiable, is_free, condition, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#
    )
    .bind(complex_id)
    .bind(auth_user.user_id)
    .bind(payload.category_id)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&payload.price)
    .bind(payload.is_negotiable.unwrap_or(false))
    .bind(payload.is_free.unwrap_or(false))
    .bind(&payload.condition)
    .bind(ListingStatus::Active)
    .fetch_one(&state.pool)
    .await?;

    let response = build_listing_response(&state, &listing, auth_user.user_id).await?;
    Ok(Json(response))
}

async fn update_listing(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateListingRequest>,
) -> AppResult<Json<ListingResponse>> {
    let listing = sqlx::query_as::<_, MarketplaceListing>(
        "SELECT * FROM marketplace_listings WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Объявление не найдено".to_string()))?;

    if listing.seller_id != auth_user.user_id {
        return Err(AppError::Forbidden);
    }

    let updated = sqlx::query_as::<_, MarketplaceListing>(
        r#"
        UPDATE marketplace_listings SET
            category_id = COALESCE($2, category_id),
            title = COALESCE($3, title),
            description = COALESCE($4, description),
            price = COALESCE($5, price),
            is_negotiable = COALESCE($6, is_negotiable),
            is_free = COALESCE($7, is_free),
            condition = COALESCE($8, condition),
            status = COALESCE($9, status),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(id)
    .bind(&payload.category_id)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&payload.price)
    .bind(&payload.is_negotiable)
    .bind(&payload.is_free)
    .bind(&payload.condition)
    .bind(&payload.status)
    .fetch_one(&state.pool)
    .await?;

    let response = build_listing_response(&state, &updated, auth_user.user_id).await?;
    Ok(Json(response))
}

async fn delete_listing(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let listing = sqlx::query_as::<_, MarketplaceListing>(
        "SELECT * FROM marketplace_listings WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Объявление не найдено".to_string()))?;

    if listing.seller_id != auth_user.user_id {
        return Err(AppError::Forbidden);
    }

    sqlx::query("UPDATE marketplace_listings SET status = 'archived' WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({"success": true})))
}

async fn toggle_favorite(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    // Проверяем, есть ли уже в избранном
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM listing_favorites WHERE listing_id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(auth_user.user_id)
    .fetch_optional(&state.pool)
    .await?;

    if let Some((fav_id,)) = existing {
        // Удаляем из избранного
        sqlx::query("DELETE FROM listing_favorites WHERE id = $1")
            .bind(fav_id)
            .execute(&state.pool)
            .await?;

        sqlx::query("UPDATE marketplace_listings SET favorites_count = favorites_count - 1 WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;

        Ok(Json(json!({"is_favorite": false})))
    } else {
        // Добавляем в избранное
        sqlx::query("INSERT INTO listing_favorites (listing_id, user_id) VALUES ($1, $2)")
            .bind(id)
            .bind(auth_user.user_id)
            .execute(&state.pool)
            .await?;

        sqlx::query("UPDATE marketplace_listings SET favorites_count = favorites_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;

        Ok(Json(json!({"is_favorite": true})))
    }
}

async fn send_message(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<SendMessageRequest>,
) -> AppResult<Json<Value>> {
    let listing = sqlx::query_as::<_, MarketplaceListing>(
        "SELECT * FROM marketplace_listings WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Объявление не найдено".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO listing_messages (listing_id, sender_id, recipient_id, message)
        VALUES ($1, $2, $3, $4)
        "#
    )
    .bind(id)
    .bind(auth_user.user_id)
    .bind(listing.seller_id)
    .bind(&payload.message)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({"success": true})))
}

async fn my_listings(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Vec<ListingResponse>>> {
    let listings = sqlx::query_as::<_, MarketplaceListing>(
        r#"
        SELECT * FROM marketplace_listings
        WHERE seller_id = $1 AND status != 'archived'
        ORDER BY created_at DESC
        "#
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for listing in listings {
        response.push(build_listing_response(&state, &listing, auth_user.user_id).await?);
    }

    Ok(Json(response))
}

async fn my_favorites(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Vec<ListingResponse>>> {
    let listings = sqlx::query_as::<_, MarketplaceListing>(
        r#"
        SELECT l.* FROM marketplace_listings l
        JOIN listing_favorites f ON f.listing_id = l.id
        WHERE f.user_id = $1 AND l.status = 'active'
        ORDER BY f.created_at DESC
        "#
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for listing in listings {
        response.push(build_listing_response(&state, &listing, auth_user.user_id).await?);
    }

    Ok(Json(response))
}
