use actix_web::{Scope, web};

use crate::handlers::{
    backup::{backup, get_backups_list, remove_backups, restore_backup},
    collection::{
        create_collection, delete_collection, get_collections, update_collections_metadata,
    },
    document::{
        add_document, delete_document, get_document_content, get_documents_metadata,
        import_documents, reindex, update_document_content, update_documents_metadata,
    },
    general::{get_info, health_check, retrieve_task_result},
    search::{intelligent_search, search},
    user::{create_user, get_user_configurations, login, update_user_configurations},
};

pub fn configure_routes() -> Scope {
    web::scope("/api/v1")
        .route("/health", web::get().to(health_check))
        .route("/info", web::get().to(get_info))
        .route(
            "/retrieve_task_result",
            web::post().to(retrieve_task_result),
        )
        .route("/sync/create_user", web::post().to(create_user))
        .route(
            "/sync/get_user_configurations",
            web::post().to(get_user_configurations),
        )
        .route(
            "/sync/update_user_configurations",
            web::post().to(update_user_configurations),
        )
        .route("/sync/login", web::post().to(login))
        .route("/async/reindex", web::post().to(reindex))
        .route("/async/import_documents", web::post().to(import_documents))
        .route("/async/add_document", web::post().to(add_document))
        .route("/async/delete_document", web::post().to(delete_document))
        .route(
            "/async/update_documents_metadata",
            web::post().to(update_documents_metadata),
        )
        .route(
            "/async/update_document_content",
            web::post().to(update_document_content),
        )
        .route(
            "/async/update_collections_metadata",
            web::post().to(update_collections_metadata),
        )
        .route(
            "/sync/get_document_content",
            web::post().to(get_document_content),
        )
        .route(
            "/sync/get_documents_metadata",
            web::get().to(get_documents_metadata),
        )
        .route("/sync/create_collection", web::post().to(create_collection))
        .route("/sync/delete_collection", web::post().to(delete_collection))
        .route("/sync/get_collections", web::get().to(get_collections))
        .route(
            "/sync/intelligent_search",
            web::post().to(intelligent_search),
        )
        .route("/sync/search", web::post().to(search))
        .route("/sync/remove_backups", web::post().to(remove_backups))
        .route("/sync/get_backups_list", web::post().to(get_backups_list))
        .route("/async/backup", web::post().to(backup))
        .route("/async/restore_backup", web::post().to(restore_backup))
}
