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
        (name = "announcements", description = "Объявления"),
        (name = "marketplace", description = "AllMix - маркетплейс между соседями"),
        (name = "voting", description = "Голосования собственников"),
        (name = "communal", description = "Коммунальные услуги: счётчики, счета, оплата"),
        (name = "Чаты", description = "Чаты и сообщения между соседями"),
        (name = "Уведомления", description = "Уведомления пользователя"),
        (name = "Заявки на обслуживание", description = "Заявки на ремонт и обслуживание")
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
        // Marketplace
        crate::api::marketplace::get_categories,
        crate::api::marketplace::list_listings,
        crate::api::marketplace::get_listing,
        crate::api::marketplace::create_listing,
        crate::api::marketplace::update_listing,
        crate::api::marketplace::delete_listing,
        crate::api::marketplace::toggle_favorite,
        crate::api::marketplace::send_message,
        crate::api::marketplace::my_listings,
        crate::api::marketplace::my_favorites,
        // Voting
        crate::api::voting::list_votings,
        crate::api::voting::get_voting,
        crate::api::voting::create_voting,
        crate::api::voting::cast_vote,
        crate::api::voting::close_voting,
        // Communal
        crate::api::communal::get_meters,
        crate::api::communal::submit_reading,
        crate::api::communal::get_readings_history,
        crate::api::communal::get_bills,
        crate::api::communal::get_bill,
        crate::api::communal::create_payment,
        crate::api::communal::get_payment,
        // Chat
        crate::api::chat::list_chats,
        crate::api::chat::create_private_chat,
        crate::api::chat::get_messages,
        crate::api::chat::send_message,
        crate::api::chat::mark_chat_as_read,
        // Notifications
        crate::api::notifications::list_notifications,
        crate::api::notifications::mark_as_read,
        crate::api::notifications::mark_all_as_read,
        crate::api::notifications::register_push_token,
        crate::api::notifications::get_unread_count,
        // Maintenance
        crate::api::maintenance::list_requests,
        crate::api::maintenance::get_request,
        crate::api::maintenance::create_request,
        crate::api::maintenance::update_status,
        crate::api::maintenance::rate_request,
        crate::api::maintenance::get_comments,
        crate::api::maintenance::add_comment,
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
            // Marketplace
            crate::models::CategoryResponse,
            crate::models::ListingResponse,
            crate::models::ListingStatus,
            crate::models::SellerInfo,
            crate::models::CreateListingRequest,
            crate::models::UpdateListingRequest,
            crate::models::ListingsQuery,
            crate::models::SendMessageRequest,
            crate::api::marketplace::FavoriteResponse,
            crate::api::marketplace::SuccessResponse,
            // Voting
            crate::models::VotingType,
            crate::models::VotingStatus,
            crate::models::VotingResponse,
            crate::models::VotingOptionResponse,
            crate::models::CreateVotingRequest,
            crate::models::CastVoteRequest,
            crate::api::voting::SuccessResponse,
            crate::api::voting::VoteResponse,
            crate::api::voting::VotingsQuery,
            // Communal
            crate::models::MeterResponse,
            crate::models::MeterReading,
            crate::models::SubmitReadingRequest,
            crate::models::BillResponse,
            crate::models::BillItemResponse,
            crate::models::BillStatus,
            crate::models::CreatePaymentRequest,
            crate::models::PaymentResponse,
            crate::models::PaymentStatus,
            crate::models::PaymentMethod,
            crate::api::communal::SubmitReadingResponse,
            crate::api::communal::BillsQuery,
            // Chat
            crate::models::ChatResponse,
            crate::models::ChatType,
            crate::models::MessagePreview,
            crate::models::ChatMessageResponse,
            crate::models::SenderInfo,
            crate::models::CreatePrivateChatRequest,
            crate::models::SendChatMessageRequest,
            crate::models::MessagesQuery,
            crate::api::chat::ChatSuccessResponse,
            // Notifications
            crate::models::NotificationResponse,
            crate::models::NotificationType,
            crate::models::NotificationsQuery,
            crate::models::RegisterPushTokenRequest,
            crate::api::notifications::NotificationSuccessResponse,
            crate::api::notifications::MarkAllReadResponse,
            crate::api::notifications::UnreadCountResponse,
            // Maintenance
            crate::models::MaintenanceRequestResponse,
            crate::models::MaintenancePhotoResponse,
            crate::models::MaintenanceCategory,
            crate::models::MaintenancePriority,
            crate::models::MaintenanceStatus,
            crate::models::CreateMaintenanceRequest,
            crate::models::UpdateMaintenanceStatusRequest,
            crate::models::RateMaintenanceRequest,
            crate::models::AddMaintenanceCommentRequest,
            crate::api::maintenance::MaintenanceSuccessResponse,
            crate::api::maintenance::CommentCreatedResponse,
            crate::api::maintenance::CommentResponse,
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
