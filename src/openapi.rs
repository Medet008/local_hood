use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "LocalHood API",
        version = "1.0.0",
        description = "Backend API для LocalHood - платформы управления жилыми комплексами в Казахстане",
        contact(
            name = "LocalHood Team",
            email = "support@localhood.kz"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server")
    ),
    tags(
        (name = "auth", description = "Аутентификация и авторизация"),
        (name = "users", description = "Управление профилем пользователя"),
        (name = "cities", description = "Города Казахстана"),
        (name = "complexes", description = "Жилые комплексы"),
        (name = "apartments", description = "Квартиры и заявки на присоединение"),
        (name = "osi", description = "ОСИ/УК - Объединения собственников имущества"),
        (name = "security", description = "Безопасность: шлагбаум, камеры, домофон"),
        (name = "announcements", description = "Объявления")
    ),
    paths(
        // Auth
        crate::api::auth::send_code,
        crate::api::auth::verify_code,
        crate::api::auth::refresh_token,
        crate::api::auth::logout,
        // Cities
        crate::api::cities::list_cities,
        // Users
        crate::api::users::get_me,
        crate::api::users::update_me,
        crate::api::users::upload_avatar,
        crate::api::users::get_my_apartments,
        // Complexes
        crate::api::complexes::search_complexes,
        crate::api::complexes::get_complex,
        crate::api::complexes::check_complex_exists,
        crate::api::complexes::create_complex,
        crate::api::complexes::join_complex,
        // Apartments
        crate::api::apartments::get_join_requests,
        crate::api::apartments::review_join_request,
        // OSI
        crate::api::osi::get_osi,
        crate::api::osi::get_osi_by_id,
        crate::api::osi::update_osi,
        crate::api::osi::get_council,
        crate::api::osi::add_council_member,
        crate::api::osi::remove_council_member,
        crate::api::osi::get_workers,
        crate::api::osi::add_worker,
        crate::api::osi::update_worker,
        crate::api::osi::remove_worker,
        crate::api::osi::get_documents,
        crate::api::osi::add_document,
        // Security
        crate::api::security::open_barrier,
        crate::api::security::create_guest_access,
        crate::api::security::get_active_guests,
        crate::api::security::cancel_guest_access,
        crate::api::security::get_barrier_history,
        crate::api::security::process_entry,
        crate::api::security::process_exit,
        crate::api::security::get_cameras,
        crate::api::security::get_camera_stream,
        crate::api::security::open_intercom,
        crate::api::security::get_intercom_calls,
        // Announcements
        crate::api::announcements::list_announcements,
        crate::api::announcements::get_announcement,
        crate::api::announcements::create_announcement,
        crate::api::announcements::update_announcement,
        crate::api::announcements::delete_announcement,
        crate::api::announcements::mark_as_read,
    ),
    components(
        schemas(
            // Auth
            crate::models::SendCodeRequest,
            crate::models::VerifyCodeRequest,
            crate::models::AuthResponse,
            crate::models::RefreshTokenRequest,
            crate::models::TokenResponse,
            crate::models::UserPublic,
            crate::models::UserRole,
            crate::models::UpdateUserRequest,
            crate::api::auth::SendCodeResponse,
            crate::api::auth::LogoutResponse,
            // Users
            crate::api::users::AvatarUploadResponse,
            // Cities
            crate::models::CityResponse,
            // Complexes
            crate::models::ComplexResponse,
            crate::models::ComplexAmenities,
            crate::models::ComplexStatus,
            crate::models::CreateComplexRequest,
            crate::models::SearchComplexQuery,
            crate::models::JoinComplexRequest,
            crate::api::complexes::ComplexExistsResponse,
            crate::api::complexes::JoinComplexResponse,
            // Apartments
            crate::models::ApartmentResponse,
            crate::models::JoinRequestStatus,
            crate::models::JoinRequestResponse,
            crate::models::ReviewJoinRequestRequest,
            crate::api::apartments::ReviewResponse,
            // OSI
            crate::models::OsiResponse,
            crate::models::ChairmanInfo,
            crate::models::UpdateOsiRequest,
            crate::models::CouncilPosition,
            crate::models::CouncilMemberResponse,
            crate::models::AddCouncilMemberRequest,
            crate::models::WorkerRole,
            crate::models::OsiWorker,
            crate::models::CreateWorkerRequest,
            crate::models::DocumentType,
            crate::models::OsiDocumentResponse,
            crate::api::osi::AddCouncilMemberResponse,
            crate::api::osi::SuccessResponse,
            crate::api::osi::AddDocumentResponse,
            crate::api::osi::AddDocumentRequest,
            // Security
            crate::models::GuestAccessStatus,
            crate::models::GuestAccessResponse,
            crate::models::CreateGuestAccessRequest,
            crate::models::BarrierAction,
            crate::models::BarrierAccessLogResponse,
            crate::models::BarrierEntryRequest,
            crate::models::CameraResponse,
            crate::models::CameraStreamResponse,
            crate::models::IntercomCallStatus,
            crate::models::IntercomCallResponse,
            crate::api::security::SuccessResponse,
            crate::api::security::OpenIntercomRequest,
            // Announcements
            crate::models::AnnouncementCategory,
            crate::models::AnnouncementPriority,
            crate::models::AnnouncementResponse,
            crate::models::CreateAnnouncementRequest,
            crate::models::UpdateAnnouncementRequest,
            crate::api::announcements::SuccessResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
        }
    }
}
