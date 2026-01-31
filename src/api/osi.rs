use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{is_chairman_or_higher, AppState, AuthUser};
use crate::models::{
    AddCouncilMemberRequest, ChairmanInfo, CouncilMember, CouncilMemberResponse,
    CreateWorkerRequest, Osi, OsiDocument, OsiDocumentResponse, OsiResponse, OsiWorker,
    UpdateOsiRequest,
};

/// Успешный ответ на добавление члена совета
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AddCouncilMemberResponse {
    pub success: bool,
    pub member_id: Uuid,
}

/// Успешный ответ на удаление
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
}

/// Успешный ответ на добавление документа
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AddDocumentResponse {
    pub success: bool,
    pub document_id: Uuid,
}

/// Запрос на добавление документа
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct AddDocumentRequest {
    pub title: String,
    pub file_url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_document_type")]
    pub document_type: String,
}

fn default_document_type() -> String {
    "other".to_string()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/by-complex/:complex_id", get(get_osi))
        .route("/:id", get(get_osi_by_id).put(update_osi))
        .route("/:id/council", get(get_council).post(add_council_member))
        .route("/:id/council/:member_id", delete(remove_council_member))
        .route("/:id/workers", get(get_workers).post(add_worker))
        .route(
            "/:id/workers/:worker_id",
            put(update_worker).delete(remove_worker),
        )
        .route("/:id/documents", get(get_documents).post(add_document))
}

/// Получение ОСИ по ID жилого комплекса
#[utoipa::path(
    get,
    path = "/api/v1/osi/by-complex/{complex_id}",
    tag = "osi",
    params(
        ("complex_id" = Uuid, Path, description = "ID жилого комплекса")
    ),
    responses(
        (status = 200, description = "Информация об ОСИ", body = OsiResponse),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn get_osi(
    State(state): State<AppState>,
    Path(complex_id): Path<Uuid>,
) -> AppResult<Json<OsiResponse>> {
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE complex_id = $1")
        .bind(complex_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    build_osi_response(&state, osi).await
}

/// Получение ОСИ по ID
#[utoipa::path(
    get,
    path = "/api/v1/osi/{id}",
    tag = "osi",
    params(
        ("id" = Uuid, Path, description = "ID ОСИ")
    ),
    responses(
        (status = 200, description = "Информация об ОСИ", body = OsiResponse),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn get_osi_by_id(
    State(state): State<AppState>,
    Path(osi_id): Path<Uuid>,
) -> AppResult<Json<OsiResponse>> {
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE id = $1")
        .bind(osi_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    build_osi_response(&state, osi).await
}

async fn build_osi_response(state: &AppState, osi: Osi) -> AppResult<Json<OsiResponse>> {
    let chairman = if let Some(chairman_id) = osi.chairman_id {
        sqlx::query_as::<_, (Uuid, Option<String>, Option<String>, String)>(
            "SELECT id, first_name, last_name, phone FROM users WHERE id = $1",
        )
        .bind(chairman_id)
        .fetch_optional(&state.pool)
        .await?
        .map(|(id, first_name, last_name, phone)| ChairmanInfo {
            id,
            name: format!(
                "{} {}",
                first_name.unwrap_or_default(),
                last_name.unwrap_or_default()
            )
            .trim()
            .to_string(),
            phone,
        })
    } else {
        None
    };

    Ok(Json(OsiResponse {
        id: osi.id,
        complex_id: osi.complex_id,
        name: osi.name,
        bin: osi.bin,
        chairman,
        phone: osi.phone,
        email: osi.email,
        address: osi.address,
    }))
}

/// Обновление информации об ОСИ
#[utoipa::path(
    put,
    path = "/api/v1/osi/{id}",
    tag = "osi",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID ОСИ")
    ),
    request_body = UpdateOsiRequest,
    responses(
        (status = 200, description = "ОСИ обновлено", body = OsiResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn update_osi(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(osi_id): Path<Uuid>,
    Json(payload): Json<UpdateOsiRequest>,
) -> AppResult<Json<OsiResponse>> {
    // Проверяем права
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE id = $1")
        .bind(osi_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    if osi.chairman_id != Some(auth_user.user_id) && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    let updated = sqlx::query_as::<_, Osi>(
        r#"
        UPDATE osi SET
            name = COALESCE($2, name),
            bin = COALESCE($3, bin),
            phone = COALESCE($4, phone),
            email = COALESCE($5, email),
            address = COALESCE($6, address),
            bank_name = COALESCE($7, bank_name),
            bank_bik = COALESCE($8, bank_bik),
            bank_account = COALESCE($9, bank_account),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(osi_id)
    .bind(&payload.name)
    .bind(&payload.bin)
    .bind(&payload.phone)
    .bind(&payload.email)
    .bind(&payload.address)
    .bind(&payload.bank_name)
    .bind(&payload.bank_bik)
    .bind(&payload.bank_account)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(OsiResponse {
        id: updated.id,
        complex_id: updated.complex_id,
        name: updated.name,
        bin: updated.bin,
        chairman: None,
        phone: updated.phone,
        email: updated.email,
        address: updated.address,
    }))
}

/// Получение членов совета ОСИ
#[utoipa::path(
    get,
    path = "/api/v1/osi/{id}/council",
    tag = "osi",
    params(
        ("id" = Uuid, Path, description = "ID ОСИ")
    ),
    responses(
        (status = 200, description = "Список членов совета", body = Vec<CouncilMemberResponse>)
    )
)]
pub async fn get_council(
    State(state): State<AppState>,
    Path(osi_id): Path<Uuid>,
) -> AppResult<Json<Vec<CouncilMemberResponse>>> {
    let members = sqlx::query_as::<_, CouncilMember>(
        "SELECT * FROM council_members WHERE osi_id = $1 AND is_active = true ORDER BY position",
    )
    .bind(osi_id)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for member in members {
        let user_info: (String, String) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(first_name || ' ' || last_name, phone),
                phone
            FROM users WHERE id = $1
            "#,
        )
        .bind(member.user_id)
        .fetch_one(&state.pool)
        .await?;

        response.push(CouncilMemberResponse {
            id: member.id,
            user_id: member.user_id,
            user_name: user_info.0,
            user_phone: user_info.1,
            position: member.position,
            responsibilities: member.responsibilities,
            appointed_at: member.appointed_at,
            is_active: member.is_active,
        });
    }

    Ok(Json(response))
}

/// Добавление члена совета ОСИ
#[utoipa::path(
    post,
    path = "/api/v1/osi/{id}/council",
    tag = "osi",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID ОСИ")
    ),
    request_body = AddCouncilMemberRequest,
    responses(
        (status = 200, description = "Член совета добавлен", body = AddCouncilMemberResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn add_council_member(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(osi_id): Path<Uuid>,
    Json(payload): Json<AddCouncilMemberRequest>,
) -> AppResult<Json<Value>> {
    // Проверяем права
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE id = $1")
        .bind(osi_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    if osi.chairman_id != Some(auth_user.user_id) && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    let member_id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO council_members (osi_id, user_id, position, responsibilities)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(osi_id)
    .bind(payload.user_id)
    .bind(&payload.position)
    .bind(&payload.responsibilities)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "member_id": member_id.0
    })))
}

/// Удаление члена совета ОСИ
#[utoipa::path(
    delete,
    path = "/api/v1/osi/{id}/council/{member_id}",
    tag = "osi",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID ОСИ"),
        ("member_id" = Uuid, Path, description = "ID члена совета")
    ),
    responses(
        (status = 200, description = "Член совета удалён", body = SuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn remove_council_member(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((osi_id, member_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE id = $1")
        .bind(osi_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    if osi.chairman_id != Some(auth_user.user_id) && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    sqlx::query("UPDATE council_members SET is_active = false WHERE id = $1 AND osi_id = $2")
        .bind(member_id)
        .bind(osi_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({"success": true})))
}

/// Получение работников ОСИ
#[utoipa::path(
    get,
    path = "/api/v1/osi/{id}/workers",
    tag = "osi",
    params(
        ("id" = Uuid, Path, description = "ID ОСИ")
    ),
    responses(
        (status = 200, description = "Список работников", body = Vec<OsiWorker>)
    )
)]
pub async fn get_workers(
    State(state): State<AppState>,
    Path(osi_id): Path<Uuid>,
) -> AppResult<Json<Vec<OsiWorker>>> {
    let workers = sqlx::query_as::<_, OsiWorker>(
        "SELECT * FROM osi_workers WHERE osi_id = $1 AND is_active = true ORDER BY last_name",
    )
    .bind(osi_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(workers))
}

/// Добавление работника ОСИ
#[utoipa::path(
    post,
    path = "/api/v1/osi/{id}/workers",
    tag = "osi",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID ОСИ")
    ),
    request_body = CreateWorkerRequest,
    responses(
        (status = 200, description = "Работник добавлен", body = OsiWorker),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn add_worker(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(osi_id): Path<Uuid>,
    Json(payload): Json<CreateWorkerRequest>,
) -> AppResult<Json<OsiWorker>> {
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE id = $1")
        .bind(osi_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    if osi.chairman_id != Some(auth_user.user_id) && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    let worker = sqlx::query_as::<_, OsiWorker>(
        r#"
        INSERT INTO osi_workers (osi_id, first_name, last_name, middle_name, phone, role, position_title, salary, hired_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(osi_id)
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(&payload.middle_name)
    .bind(&payload.phone)
    .bind(&payload.role)
    .bind(&payload.position_title)
    .bind(&payload.salary)
    .bind(&payload.hired_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(worker))
}

/// Обновление информации о работнике ОСИ
#[utoipa::path(
    put,
    path = "/api/v1/osi/{id}/workers/{worker_id}",
    tag = "osi",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID ОСИ"),
        ("worker_id" = Uuid, Path, description = "ID работника")
    ),
    request_body = CreateWorkerRequest,
    responses(
        (status = 200, description = "Работник обновлён", body = OsiWorker),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn update_worker(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((osi_id, worker_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CreateWorkerRequest>,
) -> AppResult<Json<OsiWorker>> {
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE id = $1")
        .bind(osi_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    if osi.chairman_id != Some(auth_user.user_id) && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    let worker = sqlx::query_as::<_, OsiWorker>(
        r#"
        UPDATE osi_workers SET
            first_name = $3,
            last_name = $4,
            middle_name = $5,
            phone = $6,
            role = $7,
            position_title = $8,
            salary = $9,
            hired_at = $10,
            updated_at = NOW()
        WHERE id = $1 AND osi_id = $2
        RETURNING *
        "#,
    )
    .bind(worker_id)
    .bind(osi_id)
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(&payload.middle_name)
    .bind(&payload.phone)
    .bind(&payload.role)
    .bind(&payload.position_title)
    .bind(&payload.salary)
    .bind(&payload.hired_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(worker))
}

/// Удаление работника ОСИ
#[utoipa::path(
    delete,
    path = "/api/v1/osi/{id}/workers/{worker_id}",
    tag = "osi",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID ОСИ"),
        ("worker_id" = Uuid, Path, description = "ID работника")
    ),
    responses(
        (status = 200, description = "Работник удалён", body = SuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn remove_worker(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((osi_id, worker_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE id = $1")
        .bind(osi_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    if osi.chairman_id != Some(auth_user.user_id) && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    sqlx::query("UPDATE osi_workers SET is_active = false WHERE id = $1 AND osi_id = $2")
        .bind(worker_id)
        .bind(osi_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({"success": true})))
}

/// Получение документов ОСИ
#[utoipa::path(
    get,
    path = "/api/v1/osi/{id}/documents",
    tag = "osi",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID ОСИ")
    ),
    responses(
        (status = 200, description = "Список документов", body = Vec<OsiDocumentResponse>),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn get_documents(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(osi_id): Path<Uuid>,
) -> AppResult<Json<Vec<OsiDocumentResponse>>> {
    let documents = sqlx::query_as::<_, OsiDocument>(
        "SELECT * FROM osi_documents WHERE osi_id = $1 ORDER BY created_at DESC",
    )
    .bind(osi_id)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for doc in documents {
        let uploader_name: Option<(String,)> = sqlx::query_as(
            "SELECT COALESCE(first_name || ' ' || last_name, phone) FROM users WHERE id = $1",
        )
        .bind(doc.uploaded_by)
        .fetch_optional(&state.pool)
        .await?;

        response.push(OsiDocumentResponse {
            id: doc.id,
            title: doc.title,
            description: doc.description,
            document_type: doc.document_type,
            file_url: doc.file_url,
            file_size: doc.file_size,
            uploaded_by_name: uploader_name.map(|(n,)| n),
            created_at: doc.created_at,
        });
    }

    Ok(Json(response))
}

/// Добавление документа ОСИ
#[utoipa::path(
    post,
    path = "/api/v1/osi/{id}/documents",
    tag = "osi",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID ОСИ")
    ),
    request_body = AddDocumentRequest,
    responses(
        (status = 200, description = "Документ добавлен", body = AddDocumentResponse),
        (status = 400, description = "Неверные данные"),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "ОСИ не найдено")
    )
)]
pub async fn add_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(osi_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<Value>> {
    let osi = sqlx::query_as::<_, Osi>("SELECT * FROM osi WHERE id = $1")
        .bind(osi_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ОСИ не найдено".to_string()))?;

    if osi.chairman_id != Some(auth_user.user_id) && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    let title = payload["title"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("title обязателен".to_string()))?;
    let file_url = payload["file_url"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("file_url обязателен".to_string()))?;
    let doc_type = payload["document_type"].as_str().unwrap_or("other");

    let doc_id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO osi_documents (osi_id, title, description, document_type, file_url, uploaded_by)
        VALUES ($1, $2, $3, $4::document_type, $5, $6)
        RETURNING id
        "#,
    )
    .bind(osi_id)
    .bind(title)
    .bind(payload["description"].as_str())
    .bind(doc_type)
    .bind(file_url)
    .bind(auth_user.user_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "document_id": doc_id.0
    })))
}
