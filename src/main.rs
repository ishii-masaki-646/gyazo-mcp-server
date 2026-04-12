mod app_state;
mod auth;
mod cli;
mod gyazo_api;
mod mcp_oauth;
mod runtime_config;
mod server;
mod service;
mod tools;

use std::{io, sync::Arc};

use crate::app_state::{AccessTokenRecord, AppState, AuthorizedSession};
use crate::auth::config as auth_config;
use crate::auth::oauth::{self, OAuthCallbackQuery};
use crate::auth::paths;
use crate::cli::{
    Cli, Command, ConfigArgs, ConfigCommand, EnvArgs, EnvCommand, ServiceArgs, ServiceCommand,
    StdioArgs,
};
use crate::gyazo_api::GyazoUserProfile;
use crate::mcp_oauth::{
    authorization_server_metadata_handler, authorize_handler, maybe_complete_mcp_authorization,
    protected_resource_metadata_handler, register_client_handler, require_mcp_bearer_token,
    token_handler,
};
use crate::runtime_config::RuntimeConfig;
use crate::server::GyazoServer;
use anyhow::{Result, anyhow, bail};
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use clap::Parser;
use dotenvy::{dotenv, from_path};
use rmcp::{
    ServiceExt,
    transport::{
        StreamableHttpServerConfig, StreamableHttpService, stdio,
        streamable_http_server::session::local::LocalSessionManager,
    },
};

fn load_env_files() -> Result<()> {
    if let Some(path) = paths::env_file_path()
        && path.exists()
    {
        from_path(path)?;
    }

    if let Err(error) = dotenv()
        && !error.not_found()
    {
        return Err(error.into());
    }

    Ok(())
}

async fn oauth_start_handler(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    match oauth::begin_login(app_state.as_ref()) {
        Ok(authorize_url) => Redirect::temporary(&authorize_url).into_response(),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Gyazo OAuth login を開始できませんでした: {error}"),
        )
            .into_response(),
    }
}

async fn oauth_callback_handler(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    match maybe_complete_mcp_authorization(app_state.as_ref(), &query).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(error) => {
            let (status, message) = error.into_parts();
            return (status, message).into_response();
        }
    }

    match oauth::complete_login(app_state.as_ref(), query).await {
        Ok(message) => (axum::http::StatusCode::OK, message).into_response(),
        Err(error) => {
            let (status, message) = error.into_parts();
            (status, message).into_response()
        }
    }
}

async fn root_handler() -> &'static str {
    "gyazo-mcp-server は起動中です"
}

type DirectAuthOutcome = Result<String, (StatusCode, String)>;

fn direct_auth_response_parts(outcome: &DirectAuthOutcome) -> (StatusCode, String) {
    match outcome {
        Ok(message) => (StatusCode::OK, message.clone()),
        Err((status, message)) => (*status, message.clone()),
    }
}

fn finalize_stdio_auth_outcome(outcome: Option<DirectAuthOutcome>) -> Result<String> {
    match outcome {
        Some(Ok(message)) => Ok(message),
        Some(Err((status, message))) => {
            bail!("Gyazo OAuth 認証に失敗しました (status {status}: {message})");
        }
        None => bail!("OAuth callback を受信できませんでした"),
    }
}

struct DirectAuthState {
    app_state: Arc<AppState>,
    completion: Arc<tokio::sync::Notify>,
    result: Arc<tokio::sync::Mutex<Option<DirectAuthOutcome>>>,
}

async fn complete_direct_auth(
    completion: &tokio::sync::Notify,
    result: &tokio::sync::Mutex<Option<DirectAuthOutcome>>,
    response: DirectAuthOutcome,
) -> (StatusCode, String) {
    let response_parts = direct_auth_response_parts(&response);

    let mut guard = result.lock().await;
    *guard = Some(response);
    drop(guard);
    completion.notify_waiters();

    response_parts
}

async fn direct_oauth_callback_handler(
    State(state): State<Arc<DirectAuthState>>,
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    let response = match oauth::complete_login(state.app_state.as_ref(), query).await {
        Ok(message) => Ok(message),
        Err(error) => Err(error.into_parts()),
    };

    complete_direct_auth(state.completion.as_ref(), state.result.as_ref(), response).await
}

async fn resolve_stdio_session(app_state: &AppState) -> Result<AuthorizedSession> {
    let backend_access_token = app_state.resolve_backend_access_token()?.ok_or_else(|| {
        anyhow!("stdio 起動には保存済み OAuth token か GYAZO_MCP_PERSONAL_ACCESS_TOKEN が必要です")
    })?;

    Ok(AuthorizedSession {
        record: AccessTokenRecord {
            backend_access_token,
            gyazo_user: GyazoUserProfile {
                email: String::new(),
                name: String::new(),
                profile_image: String::new(),
                uid: String::new(),
            },
        },
    })
}

async fn run_stdio_auth_flow(
    app_state: Arc<AppState>,
    runtime_config: RuntimeConfig,
) -> Result<()> {
    let authorize_url = oauth::begin_login(app_state.as_ref())?;
    let completion = Arc::new(tokio::sync::Notify::new());
    let result = Arc::new(tokio::sync::Mutex::new(None));
    let auth_state = Arc::new(DirectAuthState {
        app_state,
        completion: completion.clone(),
        result: result.clone(),
    });

    let app = Router::new()
        .route(
            runtime_config.oauth_callback_path(),
            get(direct_oauth_callback_handler),
        )
        .route("/", get(root_handler))
        .with_state(auth_state.clone());

    let listener = tokio::net::TcpListener::bind(runtime_config.bind_address()).await?;
    eprintln!("Gyazo OAuth 認証を開始します。ブラウザで次の URL を開いてください:");
    eprintln!("{authorize_url}");
    eprintln!(
        "callback は {} で待ち受けます。完了するとこのコマンドは終了します。",
        runtime_config.oauth_callback_url()
    );

    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        completion.notified().await;
    });
    let server_task = tokio::spawn(server.into_future());

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            bail!("OAuth 認証を中断しました");
        }
        _ = auth_state.completion.notified() => {}
    }

    server_task.await??;

    let message = finalize_stdio_auth_outcome(result.lock().await.take())?;
    eprintln!("{message}");

    Ok(())
}

async fn run_stdio_server(app_state: Arc<AppState>) -> Result<()> {
    let authorized_session = resolve_stdio_session(app_state.as_ref()).await?;
    let server = GyazoServer::with_fallback_authorized_session(app_state, authorized_session)?;

    tracing::info!("Gyazo MCP stdio サーバーを起動します");

    server.serve(stdio()).await?.waiting().await?;

    Ok(())
}

fn run_config_command(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Init => runtime_config::init_config(),
        ConfigCommand::Show => runtime_config::show_config(),
        ConfigCommand::Get(get_args) => runtime_config::get_config(&get_args.key),
        ConfigCommand::Set(set_args) => runtime_config::set_config(&set_args.key, &set_args.value),
        ConfigCommand::Unset(unset_args) => runtime_config::unset_config(&unset_args.key),
        ConfigCommand::Path => {
            let path = paths::config_file_path()
                .ok_or_else(|| anyhow!("設定ディレクトリを特定できませんでした"))?;
            println!("{}", path.display());
            Ok(())
        }
    }
}

fn run_env_command(args: EnvArgs) -> Result<()> {
    match args.command {
        EnvCommand::Init => auth_config::init_env(),
        EnvCommand::Show => auth_config::show_env(),
        EnvCommand::Get(get_args) => auth_config::get_env(&get_args.key),
        EnvCommand::Set(set_args) => auth_config::set_env(&set_args.key, &set_args.value),
        EnvCommand::Unset(unset_args) => auth_config::unset_env(&unset_args.key),
        EnvCommand::Path => {
            let path = paths::env_file_path()
                .ok_or_else(|| anyhow!("設定ディレクトリを特定できませんでした"))?;
            println!("{}", path.display());
            Ok(())
        }
    }
}

fn run_service_command(args: ServiceArgs) -> Result<()> {
    match args.command {
        ServiceCommand::Install => service::install(),
        ServiceCommand::Uninstall => service::uninstall(),
        ServiceCommand::Status => service::status(),
        ServiceCommand::Start => service::start(service_runtime_port_hint()?),
        ServiceCommand::Stop => service::stop(service_runtime_port_hint()?),
        ServiceCommand::Restart => service::restart(service_runtime_port_hint()?),
    }
}

/// `service start` / `service stop` / `service restart` の判定対象を TCP ポート
/// から特定するためのポート番号を返す。Windows でのみ runtime config を読む
/// 必要がある。非 Windows では `service::start` / `service::stop` /
/// `service::restart` がポートを参照しないため、`RuntimeConfig::load()` を
/// 呼ばずにダミー値を返すことで、`service` 系コマンドの「壊れた config でも
/// 救済に使える」前提を維持する。
#[cfg(target_os = "windows")]
fn service_runtime_port_hint() -> Result<u16> {
    load_env_files()?;
    Ok(RuntimeConfig::load()?.tcp_port())
}

#[cfg(not(target_os = "windows"))]
fn service_runtime_port_hint() -> Result<u16> {
    Ok(0)
}

async fn run_http_server(app_state: Arc<AppState>, runtime_config: RuntimeConfig) -> Result<()> {
    // 起動時にキャッシュを温めておく
    let has_verified = mcp_oauth::get_verified_session(app_state.as_ref())
        .await
        .is_some();
    if has_verified {
        tracing::info!("保存済みトークンで Gyazo API と疎通確認できました");
    }

    let service_app_state = app_state.clone();
    let service: StreamableHttpService<GyazoServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || GyazoServer::new(service_app_state.clone()).map_err(io::Error::other),
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default(),
        );
    let mcp_routes = Router::new()
        .nest_service(runtime_config.mcp_path(), service)
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            require_mcp_bearer_token,
        ));

    let app = Router::new()
        .route(
            runtime_config.protected_resource_metadata_root_path(),
            get(protected_resource_metadata_handler),
        )
        .route(
            &runtime_config.protected_resource_metadata_path(),
            get(protected_resource_metadata_handler),
        )
        .route(
            runtime_config.authorization_server_metadata_path(),
            get(authorization_server_metadata_handler),
        )
        .route(
            runtime_config.authorization_endpoint_path(),
            get(authorize_handler),
        )
        .route(runtime_config.token_endpoint_path(), post(token_handler))
        .route(
            runtime_config.registration_endpoint_path(),
            post(register_client_handler),
        )
        .route("/", get(root_handler))
        .route(runtime_config.oauth_start_path(), get(oauth_start_handler))
        .route(
            runtime_config.oauth_callback_path(),
            get(oauth_callback_handler),
        )
        .merge(mcp_routes)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(runtime_config.bind_address()).await?;
    tracing::info!(
        bind_address = %runtime_config.bind_address(),
        mcp_url = %runtime_config.mcp_url(),
        protected_resource_metadata_url = %runtime_config.protected_resource_metadata_url(),
        authorization_server_metadata_url = %runtime_config.authorization_server_metadata_url(),
        authorization_endpoint_url = %runtime_config.authorization_endpoint_url(),
        token_endpoint_url = %runtime_config.token_endpoint_url(),
        registration_endpoint_url = %runtime_config.registration_endpoint_url(),
        oauth_start_url = %runtime_config.oauth_start_url(),
        oauth_callback_url = %runtime_config.oauth_callback_url(),
        "Gyazo MCP HTTP サーバーを起動します",
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(dir) = &cli.config_dir {
        paths::set_config_dir_override(std::path::PathBuf::from(dir));
    } else {
        // CLI override がなければ、デフォルト .env から GYAZO_MCP_CONFIG_DIR を
        // 先読みして環境変数にセットする。load_env_files() より前に行うのは、
        // load_env_files() 自体が paths::config_dir() → env_file_path() を
        // 経由するため、先に config_dir を確定させておく必要があるため。
        if let Some(dir) = auth_config::read_config_dir_from_default_env() {
            // Safety: main の最初期でまだ他スレッドは起動していない
            unsafe { std::env::set_var("GYAZO_MCP_CONFIG_DIR", &dir) };
        }
    }

    // config/env/service コマンドは設定ファイルの読み書きを自前で行うため、
    // load_env_files() や RuntimeConfig::load() より前にディスパッチする。
    // これにより config.toml が壊れていても config set で復旧できる。
    match cli.command {
        Some(Command::Config(args)) => return run_config_command(args),
        Some(Command::Env(args)) => return run_env_command(args),
        Some(Command::Service(args)) => return run_service_command(args),
        _ => {}
    }

    load_env_files()?;
    let runtime_config = RuntimeConfig::load()?;

    tracing_subscriber::fmt()
        .with_env_filter(runtime_config.tracing_env_filter())
        .with_writer(std::io::stderr)
        .init();
    let app_state = Arc::new(AppState::new(runtime_config.clone())?);

    match cli.command {
        Some(Command::Stdio(StdioArgs { auth: true })) => {
            run_stdio_auth_flow(app_state, runtime_config).await?
        }
        Some(Command::Stdio(StdioArgs { auth: false })) => run_stdio_server(app_state).await?,
        Some(Command::Config(_) | Command::Env(_) | Command::Service(_)) => unreachable!(),
        None => run_http_server(app_state, runtime_config).await?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use axum::http::StatusCode;
    use tokio::time::timeout;

    use super::{complete_direct_auth, direct_auth_response_parts, finalize_stdio_auth_outcome};
    // service_runtime_port_hint を参照する回帰テストは非 Windows 限定。
    // Windows ビルドでは下記 import が unused 警告になるので cfg ガードする。
    #[cfg(not(target_os = "windows"))]
    use super::service_runtime_port_hint;

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn service_runtime_port_hint_does_not_load_runtime_config_on_unix() {
        // 回帰テスト:
        // `service` 系コマンドは RuntimeConfig::load() より前に動かせる前提で
        // 設計されている (壊れた config.toml でも install/uninstall/status で
        // 復旧できるようにするため)。Windows の停止対象特定で導入した
        // service_runtime_port_hint() が、非 Windows で RuntimeConfig::load() を
        // 呼ぶ実装に回帰すると、この前提が壊れる。
        //
        // ここでは GYAZO_MCP_CONFIG_DIR を実在しない壊れたパスに向けても
        // service_runtime_port_hint() が成功し、ダミー値 0 を返すことを保証する。
        // (環境変数を触るので #[test] は直列化が必要なケースもあるが、
        // service_runtime_port_hint の動作を変えずに完結するため副作用は閉じる)
        let prev = std::env::var("GYAZO_MCP_CONFIG_DIR").ok();
        // Safety: テスト用の一時上書き
        unsafe {
            std::env::set_var(
                "GYAZO_MCP_CONFIG_DIR",
                "/nonexistent/gyazo-mcp-server-broken-path-for-test",
            );
        }

        let result = service_runtime_port_hint();

        // 後始末
        unsafe {
            match prev {
                Some(v) => std::env::set_var("GYAZO_MCP_CONFIG_DIR", v),
                None => std::env::remove_var("GYAZO_MCP_CONFIG_DIR"),
            }
        }

        assert!(
            result.is_ok(),
            "非 Windows で service_runtime_port_hint が失敗しました: {result:?}"
        );
        assert_eq!(
            result.unwrap(),
            0,
            "非 Windows ではポートは未使用なのでダミー値 0 を返すべきです"
        );
    }

    #[test]
    fn direct_auth_response_parts_returns_ok_for_success() {
        let response = direct_auth_response_parts(&Ok("done".to_string()));

        assert_eq!(response, (StatusCode::OK, "done".to_string()));
    }

    #[test]
    fn direct_auth_response_parts_preserves_failure_parts() {
        let response =
            direct_auth_response_parts(&Err((StatusCode::BAD_REQUEST, "bad request".to_string())));

        assert_eq!(
            response,
            (StatusCode::BAD_REQUEST, "bad request".to_string())
        );
    }

    #[test]
    fn finalize_stdio_auth_outcome_returns_success_message() {
        let message = finalize_stdio_auth_outcome(Some(Ok("saved".to_string()))).unwrap();

        assert_eq!(message, "saved");
    }

    #[test]
    fn finalize_stdio_auth_outcome_returns_failure_error() {
        let error = finalize_stdio_auth_outcome(Some(Err((
            StatusCode::BAD_GATEWAY,
            "exchange failed".to_string(),
        ))))
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Gyazo OAuth 認証に失敗しました (status 502 Bad Gateway: exchange failed)"
        );
    }

    #[test]
    fn finalize_stdio_auth_outcome_returns_missing_callback_error() {
        let error = finalize_stdio_auth_outcome(None).unwrap_err();

        assert_eq!(error.to_string(), "OAuth callback を受信できませんでした");
    }

    #[tokio::test]
    async fn complete_direct_auth_notifies_all_waiters_and_stores_result() {
        let completion = Arc::new(tokio::sync::Notify::new());
        let result = tokio::sync::Mutex::new(None);

        let waiter_one = completion.notified();
        let waiter_two = completion.notified();

        let response = complete_direct_auth(&completion, &result, Ok("saved".to_string())).await;

        assert_eq!(response, (StatusCode::OK, "saved".to_string()));
        assert_eq!(result.lock().await.as_ref(), Some(&Ok("saved".to_string())));
        timeout(Duration::from_millis(100), waiter_one)
            .await
            .expect("1つ目の waiter が起きる必要があります");
        timeout(Duration::from_millis(100), waiter_two)
            .await
            .expect("2つ目の waiter も起きる必要があります");
    }
}
