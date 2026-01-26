use actix_web::{HttpResponse, Result, web};
use tokio::sync::RwLock;

use crate::{
    api_models::{
        callbacks::{GenericResponse, RetrieveTaskResultRequest},
        general::{HealthResponse, InfoResponse},
    },
    app_state::AppState,
    tasks_scheduler::TaskStatus,
};

pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn get_info(data: web::Data<RwLock<AppState>>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(GenericResponse::succeed(
        format!(""),
        &InfoResponse {
            service: "OpenNote".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            host: data.read().await.config.server.host.clone(),
            port: data.read().await.config.server.port.clone(),
        },
    )))
}

pub async fn retrieve_task_result(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<RetrieveTaskResultRequest>,
) -> Result<HttpResponse> {
    // First, check the task status with a read lock
    log::info!("Fetching task status for id: {}", request.task_id);
    let task_record: Option<crate::tasks_scheduler::TaskRecord> = {
        let read_guard = data.read().await;
        if let Some(task_record) = read_guard
            .tasks_scheduler
            .lock()
            .await
            .search_by_task_id(&request.task_id)
        {
            Some(task_record.clone())
        } else {
            None
        }
    }; // Read lock is dropped here

    log::info!("Task status acquired for id: {}", request.task_id);
    match task_record {
        Some(result) => {
            match result.status {
                TaskStatus::Failed => {
                    return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                        request.task_id.clone(),
                        result.message.unwrap_or("unknown".to_string()),
                    )));
                }
                TaskStatus::InProgress => {
                    return Ok(HttpResponse::Ok()
                        .json(GenericResponse::in_progress(request.task_id.clone())));
                }
                TaskStatus::Completed => {
                    // Now acquire write lock separately to get the result
                    let result = data
                        .write()
                        .await
                        .tasks_scheduler
                        .lock()
                        .await
                        .get_task_result(&request.task_id);

                    if let Some(result) = result {
                        return Ok(HttpResponse::Ok()
                            .json(GenericResponse::succeed(request.task_id.clone(), &result)));
                    }

                    return Ok(HttpResponse::Ok()
                        .json(GenericResponse::in_progress(request.task_id.clone())));
                }
            }
        }
        None => {
            return Ok(HttpResponse::NotFound().json(GenericResponse::fail(
                request.task_id.clone(),
                "Task not found.".to_string(),
            )));
        }
    }
}
