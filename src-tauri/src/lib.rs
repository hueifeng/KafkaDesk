mod app;
mod commands;
mod models;
mod repositories;
mod services;

use app::state::AppState;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::bootstrap(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::ping,
            commands::audit::list_audit_events,
            commands::audit::get_audit_event,
            commands::bookmarks::list_message_bookmarks,
            commands::bookmarks::create_message_bookmark,
            commands::bookmarks::delete_message_bookmark,
            commands::clusters::list_clusters,
            commands::clusters::get_cluster_profile,
            commands::clusters::create_cluster_profile,
            commands::clusters::update_cluster_profile,
            commands::clusters::test_cluster_connection,
            commands::correlation::list_correlation_rules,
            commands::correlation::create_correlation_rule,
            commands::correlation::update_correlation_rule,
            commands::groups::list_groups,
            commands::groups::get_group_detail,
            commands::groups::update_group_tags,
            commands::messages::query_messages,
            commands::messages::get_message_detail,
            commands::preferences::get_app_preferences,
            commands::preferences::update_app_preferences,
            commands::replay::create_replay_job,
            commands::replay::list_replay_jobs,
            commands::replay::get_replay_job,
            commands::replay_policy::get_replay_policy,
            commands::replay_policy::update_replay_policy,
            commands::saved_queries::list_saved_queries,
            commands::saved_queries::create_saved_query,
            commands::saved_queries::update_saved_query,
            commands::saved_queries::delete_saved_query,
            commands::schema_registry::list_schema_registry_profiles,
            commands::schema_registry::create_schema_registry_profile,
            commands::schema_registry::update_schema_registry_profile,
            commands::schema_registry::test_schema_registry_profile,
            commands::trace::run_trace_query,
            commands::topics::list_topics,
            commands::topics::get_topic_detail,
            commands::topics::get_topic_operations_overview,
            commands::topics::update_topic_config,
            commands::topics::expand_topic_partitions,
            commands::topics::update_topic_tags,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run KafkaDesk")
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
