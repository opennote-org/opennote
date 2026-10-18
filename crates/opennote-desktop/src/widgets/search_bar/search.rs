use futures::{StreamExt, stream::FuturesUnordered};
use gpui_kit::{Context, SharedString, Task, component::list::ListState};
use uuid::Uuid;

use opennote_embedder::{entry::EmbedderEntry, vectorization::send_vectorization};
use opennote_models::{
    configurations::fields::search::SupportedSearchMethod, payload::create_query,
};

use crate::{
    globals::{
        actions::search::{SearchStatus, search},
        bootstrap::helpers::get_bootstrap,
        helpers::run_async_background,
        states::server_registry::ServerStates,
    },
    widgets::search_bar::search_results::SearchResultsList,
};

#[derive(Clone)]
pub struct SearchRequest {
    pub method: SupportedSearchMethod,
    pub query: String,
    pub vector: Option<Vec<f32>>,
    pub top_n: usize,
}

pub(super) async fn create_query_vector(
    embedding_query: &str,
    embedders: &EmbedderEntry,
) -> anyhow::Result<Vec<f32>> {
    let payloads = send_vectorization(vec![create_query(embedding_query)], embedders).await?;
    payloads
        .into_iter()
        .next()
        .map(|payload| payload.vector)
        .filter(|vector| !vector.is_empty())
        .ok_or_else(|| anyhow::anyhow!("No query embedding returned"))
}

/// Coordinate a debounced search and apply only results belonging to the active query.
pub fn spawn_search(
    query_id: usize,
    request: SearchRequest,
    servers: Vec<(SharedString, ServerStates)>,
    block_ids: Vec<Uuid>,
    cx: &mut Context<ListState<SearchResultsList>>,
) -> Task<()> {
    let SearchRequest {
        method,
        query,
        top_n,
        ..
    } = request;

    let bootstrap = get_bootstrap(cx);
    let databases = bootstrap.0.databases.clone();
    let embedders = bootstrap.0.embedders.clone();
    let system_configurations = bootstrap.get_configurations().system.clone();

    let executor = cx.background_executor().clone();
    let tokio_handle = tokio::runtime::Handle::current();

    cx.spawn(async move |this, cx| {
        // Avoid embedding every intermediate keystroke during fast typing.
        executor.timer(std::time::Duration::from_millis(150)).await;

        // Stop if a newer query replaced this one during the debounce delay,
        // or if the results list no longer exists.
        if !this
            .update(cx, |list, _| list.delegate().active_query_id == query_id)
            .unwrap_or(false)
        {
            return;
        }

        let vector = match method {
            SupportedSearchMethod::Keyword => None,
            SupportedSearchMethod::Semantic => {
                let embedding_query = query.clone();

                let result = run_async_background(&executor, tokio_handle.clone(), async move {
                    create_query_vector(&embedding_query, &embedders).await
                })
                .await;

                match result {
                    Ok(vector) => Some(vector),
                    Err(error) => {
                        tracing::warn!("Semantic search unavailable: {error}");
                        let _ = this.update(cx, |list, cx| {
                            if list.delegate().active_query_id == query_id {
                                list.delegate_mut().status = SearchStatus::SemanticUnavailable;
                                cx.notify();
                            }
                        });

                        return;
                    }
                }
            }
        };

        // The query may have changed while embedding generation was awaiting.
        // Skip server searches if it is outdated or the results list no longer exists.
        if !this
            .update(cx, |list, _| list.delegate().active_query_id == query_id)
            .unwrap_or(false)
        {
            return;
        }

        let mut tasks = FuturesUnordered::new();
        for (name, server) in servers {
            let databases = databases.clone();
            let system_configurations = system_configurations.clone();
            let request = SearchRequest {
                method,
                query: query.clone(),
                vector: vector.clone(),
                top_n,
            };

            let block_ids: Vec<_> = block_ids
                .iter()
                .copied()
                .filter(|id| server.blocks.contains_key(id))
                .collect();

            if block_ids.is_empty() {
                continue;
            }

            tasks.push(run_async_background(
                &executor,
                tokio_handle.clone(),
                async move {
                    search(
                        &name,
                        &server,
                        &databases,
                        block_ids,
                        request.method,
                        system_configurations,
                        Some(request.query),
                        request.vector,
                        request.top_n,
                    )
                    .await
                },
            ));
        }

        let mut failed = false;
        while let Some(result) = tasks.next().await {
            let results = match result {
                Ok(results) => results,
                Err(error) => {
                    tracing::warn!("Search failed: {error}");
                    failed = true;
                    Vec::new()
                }
            };

            let current = this
                .update(cx, |list, cx| {
                    let delegate = list.delegate_mut();

                    // Discard results if a newer query started while the server request was running.
                    if delegate.active_query_id != query_id {
                        return false;
                    }

                    delegate.extend_results(results, top_n);

                    delegate.status = match (tasks.is_empty(), failed, delegate.results.len()) {
                        (false, _, _) => SearchStatus::Searching,
                        (true, true, _) => SearchStatus::PartialFailure,
                        (true, false, 0) => SearchStatus::Empty,
                        (true, false, count) => SearchStatus::Matches(count),
                    };

                    cx.notify();

                    true
                })
                .unwrap_or(false);

            if !current {
                return;
            }
        }
    })
}
