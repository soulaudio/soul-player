/// Soul Server - Multi-user music streaming server
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware as axum_middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use clap::{Parser, Subcommand};
use soul_core::types::UserId;
use soul_server::{
    api,
    config::ServerConfig,
    jobs, middleware,
    services::{AuthService, FileStorage, TranscodingService},
    state::AppState,
};
use soul_storage::LocalStorageContext;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tower::ServiceExt;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "soul-server")]
#[command(about = "Soul Player multi-user streaming server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the HTTP server
    Serve {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Create a new user
    AddUser {
        /// Username
        #[arg(short, long)]
        username: String,
        /// Password
        #[arg(short, long)]
        password: String,
    },
    /// List all users
    ListUsers,
    /// Scan a directory for music files
    Scan {
        /// Directory path to scan
        path: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "soul_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config: _ } => {
            serve().await?;
        }
        Commands::AddUser { username, password } => {
            add_user(&username, &password).await?;
        }
        Commands::ListUsers => {
            list_users().await?;
        }
        Commands::Scan { path } => {
            scan_directory(&path).await?;
        }
    }

    Ok(())
}

async fn serve() -> anyhow::Result<()> {
    // Load configuration
    let config = ServerConfig::load()?;
    config.validate()?;

    tracing::info!("Starting Soul Server");
    tracing::info!("Host: {}", config.server.host);
    tracing::info!("Port: {}", config.server.port);

    // Initialize database
    let pool = soul_storage::create_pool(&config.storage.database_url).await?;
    soul_storage::run_migrations(&pool).await?;

    // For server, we use a system user (user_id = 1)
    // The actual user context will be determined by JWT authentication
    let system_user_id = UserId::new("1".to_string());
    let db = LocalStorageContext::new(pool, system_user_id);
    let db = Arc::new(db);
    tracing::info!("Database connected");

    // Initialize file storage
    let file_storage = FileStorage::new(config.storage.music_storage_path.clone());
    file_storage.initialize().await?;
    let file_storage = Arc::new(file_storage);
    tracing::info!("File storage initialized");

    // Initialize auth service
    let auth_service = AuthService::new(
        config.auth.jwt_secret.clone(),
        config.auth.jwt_expiration_hours,
        config.auth.jwt_refresh_expiration_days,
    );
    let auth_service = Arc::new(auth_service);
    tracing::info!("Auth service initialized");

    // Initialize transcoding service
    let transcoding_service = TranscodingService::new(config.transcoding.ffmpeg_path.clone());
    let transcoding_service = Arc::new(transcoding_service);

    // Initialize transcoding queue
    if config.transcoding.enabled {
        let queue = Arc::new(jobs::TranscodingQueue::new(
            Arc::clone(&transcoding_service),
            Arc::clone(&file_storage),
            config.transcoding.workers,
        ));
        Arc::clone(&queue).start();
        tracing::info!(
            "Transcoding queue started with {} workers",
            config.transcoding.workers
        );
    }

    // Build application state
    let app_state = AppState::new(db, Arc::clone(&auth_service), file_storage);

    // Build router
    let app = create_router(app_state, auth_service);

    // Create server address
    let addr = SocketAddr::from((
        config.server.host.parse::<std::net::IpAddr>()?,
        config.server.port,
    ));

    tracing::info!("Server listening on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn create_router(app_state: AppState, auth_service: Arc<AuthService>) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(api::health::health))
        .route("/auth/login", post(api::auth::login))
        .route("/auth/refresh", post(api::auth::refresh));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        // Tracks
        .route("/tracks", get(api::tracks::list_tracks))
        .route("/tracks/:id", get(api::tracks::get_track))
        .route("/tracks/import", post(api::tracks::import_track))
        .route("/tracks/:id", delete(api::tracks::delete_track))
        // Playlists
        .route("/playlists", get(api::playlists::list_playlists))
        .route("/playlists", post(api::playlists::create_playlist))
        .route("/playlists/:id", get(api::playlists::get_playlist))
        .route("/playlists/:id", put(api::playlists::update_playlist))
        .route("/playlists/:id", delete(api::playlists::delete_playlist))
        .route(
            "/playlists/:id/tracks",
            post(api::playlists::add_track_to_playlist),
        )
        .route(
            "/playlists/:id/tracks/:track_id",
            delete(api::playlists::remove_track_from_playlist),
        )
        .route("/playlists/:id/share", post(api::playlists::share_playlist))
        .route(
            "/playlists/:id/share/:user_id",
            delete(api::playlists::unshare_playlist),
        )
        // Streaming
        .route("/stream/:track_id", get(api::stream::stream_track))
        // Devices
        .route("/devices", post(api::devices::register_device))
        .route("/devices", get(api::devices::list_devices))
        .route("/devices/:id", delete(api::devices::unregister_device))
        .route("/devices/:id/activate", put(api::devices::activate_device))
        .route("/devices/:id/heartbeat", post(api::devices::heartbeat))
        // Playback state
        .route("/playback", get(api::playback::get_playback))
        .route("/playback", put(api::playback::update_playback))
        .route("/playback/play", post(api::playback::play))
        .route("/playback/pause", post(api::playback::pause))
        .route("/playback/seek", post(api::playback::seek))
        .route("/playback/volume", post(api::playback::set_volume))
        .route("/playback/skip/next", post(api::playback::skip_next))
        .route("/playback/skip/previous", post(api::playback::skip_previous))
        .route("/playback/transfer", post(api::playback::transfer))
        // Admin
        .route("/admin/users", post(api::admin::create_user))
        .route("/admin/users", get(api::admin::list_users))
        .route("/admin/users/:id", delete(api::admin::delete_user))
        .route("/admin/scan", post(api::admin::trigger_scan))
        .route("/admin/scan/status", get(api::admin::scan_status))
        .layer(axum_middleware::from_fn_with_state(
            Arc::clone(&auth_service),
            middleware::auth_middleware,
        ));

    // Static file serving for web UI (SPA with fallback to index.html)
    let web_dir = PathBuf::from(
        std::env::var("SOUL_WEB_DIR").unwrap_or_else(|_| "/app/web".to_string()),
    );

    let spa_fallback = move |req: Request<Body>| {
        let web_dir = web_dir.clone();
        async move {
            // Try to serve the file directly
            let path = req.uri().path().trim_start_matches('/');
            let file_path = web_dir.join(path);

            if file_path.exists() && file_path.is_file() {
                // Serve the actual file
                match ServeDir::new(&web_dir)
                    .oneshot(req)
                    .await
                {
                    Ok(res) => res.into_response(),
                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                }
            } else {
                // SPA fallback: serve index.html
                let index_path = web_dir.join("index.html");
                if index_path.exists() {
                    match tokio::fs::read(&index_path).await {
                        Ok(contents) => Response::builder()
                            .status(StatusCode::OK)
                            .header("content-type", "text/html; charset=utf-8")
                            .body(Body::from(contents))
                            .unwrap()
                            .into_response(),
                        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    }
                } else {
                    // No web UI available, return 404
                    StatusCode::NOT_FOUND.into_response()
                }
            }
        }
    };

    // Combine routes
    Router::new()
        .nest("/api", public_routes.merge(protected_routes))
        .fallback(spa_fallback)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(CorsLayer::permissive())
        .with_state(app_state)
}

async fn add_user(username: &str, password: &str) -> anyhow::Result<()> {
    let config = ServerConfig::load()?;
    let pool = soul_storage::create_pool(&config.storage.database_url).await?;
    soul_storage::run_migrations(&pool).await?;

    let auth_service = AuthService::new(
        config.auth.jwt_secret.clone(),
        config.auth.jwt_expiration_hours,
        config.auth.jwt_refresh_expiration_days,
    );

    // TODO: Create user - need to implement user creation
    // For now, just show what would be done
    tracing::warn!("User creation not yet implemented");
    tracing::info!("Would create user: {}", username);

    // Hash password
    let password_hash = auth_service.hash_password(password)?;

    // Store credentials using soul_storage::users module directly
    // TODO: Store in database once user creation is implemented
    tracing::info!("Password hash: {}", password_hash);
    tracing::warn!("Password credential storage not yet implemented");

    Ok(())
}

async fn list_users() -> anyhow::Result<()> {
    let config = ServerConfig::load()?;
    let pool = soul_storage::create_pool(&config.storage.database_url).await?;
    soul_storage::run_migrations(&pool).await?;

    // Use soul_storage::users::get_all directly
    let users = soul_storage::users::get_all(&pool).await?;

    println!("Users:");
    for user in users {
        println!("  {} - {}", user.id, user.name);
    }

    Ok(())
}

async fn scan_directory(_path: &str) -> anyhow::Result<()> {
    // TODO: Implement directory scanning
    tracing::warn!("Directory scanning not yet implemented");
    Ok(())
}
