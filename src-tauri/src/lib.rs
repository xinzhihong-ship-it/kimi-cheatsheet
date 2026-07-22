#[macro_use]
extern crate objc;

use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::menu::{ContextMenu, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_dialog::DialogExt;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CommandItem {
    pub cat: String,
    pub cmd: String,
    pub desc: String,
}

#[tauri::command]
fn load_commands(app: AppHandle) -> Result<Vec<CommandItem>, String> {
    let path = commands_path(&app)?;
    if !path.exists() {
        return Ok(default_commands());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {}", e))?;
    let commands: Vec<CommandItem> =
        serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))?;
    Ok(commands)
}

#[tauri::command]
fn save_commands(app: AppHandle, commands: Vec<CommandItem>) -> Result<(), String> {
    let path = commands_path(&app)?;
    let content =
        serde_json::to_string_pretty(&commands).map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn export_commands(app: AppHandle) -> Result<(), String> {
    let commands = load_commands(app.clone())?;
    let content =
        serde_json::to_string_pretty(&commands).map_err(|e| format!("序列化失败: {}", e))?;

    let (tx, rx) = std::sync::mpsc::channel::<Option<std::path::PathBuf>>();
    app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .save_file(move |path| {
            let _ = tx.send(path.map(|p| match p {
                tauri_plugin_dialog::FilePath::Path(pb) => pb,
                tauri_plugin_dialog::FilePath::Url(u) => std::path::PathBuf::from(u.path()),
            }));
        });

    match rx.recv().map_err(|e| format!("通道错误: {}", e))? {
        Some(path) => {
            fs::write(&path, content).map_err(|e| format!("导出失败: {}", e))?;
            Ok(())
        }
        None => Err("用户取消".to_string()),
    }
}

#[tauri::command]
async fn import_commands(app: AppHandle) -> Result<Vec<CommandItem>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<std::path::PathBuf>>();
    app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .pick_file(move |path| {
            let _ = tx.send(path.map(|p| match p {
                tauri_plugin_dialog::FilePath::Path(pb) => pb,
                tauri_plugin_dialog::FilePath::Url(u) => std::path::PathBuf::from(u.path()),
            }));
        });

    match rx.recv().map_err(|e| format!("通道错误: {}", e))? {
        Some(path) => {
            let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {}", e))?;
            let commands: Vec<CommandItem> =
                serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))?;
            Ok(commands)
        }
        None => Err("用户取消".to_string()),
    }
}

#[tauri::command]
fn get_autostart_status(app: AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("获取开机启动状态失败: {}", e))
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|e| format!("设置开机启动失败: {}", e))
}

fn commands_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {}", e))?;
    Ok(dir.join("commands.json"))
}

// ── 额度查询 ───────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct QuotaSettings {
    pub enabled: bool,
    pub api_key: String,
    pub interval_minutes: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct QuotaTier {
    pub name: String,
    pub limit: f64,
    pub remaining: f64,
    pub reset_time: Option<String>,
    pub percentage: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct QuotaResult {
    pub success: bool,
    pub error: Option<String>,
    pub fetched_at: Option<String>,
    pub weekly: Option<QuotaTier>,
    pub five_hour: Option<QuotaTier>,
    pub raw: serde_json::Value,
}

fn quota_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {}", e))?;
    Ok(dir.join("quota_settings.json"))
}

fn quota_last_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {}", e))?;
    Ok(dir.join("quota_last.json"))
}

fn version_last_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {}", e))?;
    Ok(dir.join("version_last.json"))
}

#[tauri::command]
fn load_quota_settings(app: AppHandle) -> Result<QuotaSettings, String> {
    let path = quota_settings_path(&app)?;
    if !path.exists() {
        return Ok(QuotaSettings::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))
}

#[tauri::command]
fn save_quota_settings(app: AppHandle, settings: QuotaSettings) -> Result<(), String> {
    let path = quota_settings_path(&app)?;
    let content = serde_json::to_string_pretty(&settings).map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("写入失败: {}", e))?;
    // 设置变更后立即刷新一次托盘标题
    let _ = app.emit("quota-settings-changed", ());
    Ok(())
}

#[tauri::command]
async fn refresh_quota(app: AppHandle) -> Result<QuotaResult, String> {
    let settings = load_quota_settings(app.clone())?;
    if !settings.enabled || settings.api_key.trim().is_empty() {
        return Ok(QuotaResult {
            success: false,
            error: Some("未启用或未配置 API Key".to_string()),
            ..Default::default()
        });
    }

    let result = fetch_kimi_quota(&settings.api_key).await;

    // 保存最新结果
    if let Ok(ref r) = result {
        if let Ok(path) = quota_last_path(&app) {
            if let Ok(content) = serde_json::to_string_pretty(r) {
                let _ = fs::write(&path, content);
            }
        }
    }

    // 更新托盘标题
    if let Ok(ref r) = result {
        update_tray_title(&app, r, &settings);
    }

    result
}

#[tauri::command]
fn load_last_quota(app: AppHandle) -> Result<QuotaResult, String> {
    let path = quota_last_path(&app)?;
    if !path.exists() {
        return Ok(QuotaResult::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))
}

// ── 启动设置 ───────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct AppSettings {
    pub cli_command: String,
    pub web_command: String,
    /// 启动 Web 后用于打开页面的浏览器 App 名，空 = 系统默认浏览器
    #[serde(default)]
    pub browser: String,
}

impl AppSettings {
    fn with_defaults() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            cli_command: r#"osascript -e 'tell application "Terminal" to do script "kimi"'"#.to_string(),
            #[cfg(not(target_os = "macos"))]
            cli_command: "kimi".to_string(),
            web_command: "kimi web".to_string(),
            browser: String::new(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct VersionInfo {
    pub local: String,
    pub latest: Option<String>,
    pub has_update: bool,
    pub error: Option<String>,
}

fn app_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {}", e))?;
    Ok(dir.join("app_settings.json"))
}

#[tauri::command]
fn load_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    let path = app_settings_path(&app)?;
    if !path.exists() {
        return Ok(AppSettings::with_defaults());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {}", e))?;
    let mut settings: AppSettings =
        serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))?;
    // 补齐默认值，避免旧配置缺字段
    if settings.cli_command.trim().is_empty() {
        settings.cli_command = AppSettings::with_defaults().cli_command;
    }
    if settings.web_command.trim().is_empty() {
        settings.web_command = AppSettings::with_defaults().web_command;
    }
    Ok(settings)
}

#[tauri::command]
fn save_app_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let path = app_settings_path(&app)?;
    let content =
        serde_json::to_string_pretty(&settings).map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

// ── 启动状态与进程管理 ─────────────────────────────────────────────────────

use std::sync::{Arc, Mutex};

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, Default)]
pub struct LaunchState {
    pub cli_running: bool,
    pub web_running: bool,
}

static LAUNCH_STATE: std::sync::OnceLock<Arc<Mutex<LaunchState>>> = std::sync::OnceLock::new();

fn launch_state() -> Arc<Mutex<LaunchState>> {
    LAUNCH_STATE.get_or_init(|| Arc::new(Mutex::new(LaunchState::default()))).clone()
}

/// 获取所有 kimi / kimi-code 进程，并排除本 GUI 应用自身
fn list_kimi_processes() -> Vec<(i32, String)> {
    let output = match std::process::Command::new("ps")
        .args(["-eo", "pid,args"])
        .output()
    {
        Ok(o) => o.stdout,
        Err(_) => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&output);
    text.lines()
        .skip(1) // 跳过表头
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse::<i32>().ok()?;
            let args = parts.collect::<Vec<_>>().join(" ");
            // 匹配 kimi 或 kimi-code，且排除本 GUI 应用
            let is_kimi = args.contains("kimi") || args.contains("kimi-code");
            let is_gui = args.contains("Kimi Cheatsheet.app") || args.contains("kimi-cheatsheet");
            if is_kimi && !is_gui {
                Some((pid, args))
            } else {
                None
            }
        })
        .collect()
}

/// 判断指定 PID 是否正在监听 TCP 端口
#[cfg(target_os = "macos")]
fn process_is_listening(pid: i32) -> bool {
    let output = match std::process::Command::new("lsof")
        .args(["-Pan", "-p", &pid.to_string(), "-i", "TCP"])
        .output()
    {
        Ok(o) => o.stdout,
        Err(_) => return false,
    };
    let text = String::from_utf8_lossy(&output);
    text.lines().any(|line| line.contains("(LISTEN"))
}

#[cfg(not(target_os = "macos"))]
fn process_is_listening(_pid: i32) -> bool {
    false
}

fn detect_launch_state() -> LaunchState {
    let procs = list_kimi_processes();

    // Web / Server：进程在监听 TCP 端口，或命令行显式带 web/server 参数
    let web_by_args = procs.iter().any(|(_, args)| {
        args.contains("kimi web") || args.contains("kimi-code web") ||
        args.contains("kimi server") || args.contains("kimi-code server")
    });
    let web_by_port = procs.iter().any(|(pid, _)| process_is_listening(*pid));
    let web_running = web_by_args || web_by_port;

    // CLI：存在 kimi/kimi-code 进程，且当前不是 web/server 模式
    let any_kimi = !procs.is_empty();
    let cli_running = any_kimi && !web_running;

    LaunchState {
        cli_running,
        web_running,
    }
}

#[tauri::command]
fn get_launch_state() -> LaunchState {
    let state = detect_launch_state();
    if let Ok(mut cached) = launch_state().lock() {
        *cached = state;
    }
    state
}

// ── Web 服务连接信息 ───────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct WebServerInfo {
    pub running: bool,
    pub url: String,
    pub token: String,
    pub full_url: String,
    pub note: String,
}

fn kimi_code_home() -> PathBuf {
    if let Ok(h) = std::env::var("KIMI_CODE_HOME") {
        if !h.trim().is_empty() {
            return PathBuf::from(h);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".kimi-code");
    }
    PathBuf::from(".kimi-code")
}

#[tauri::command]
fn get_web_server_info() -> WebServerInfo {
    let home = kimi_code_home();
    let mut info = WebServerInfo::default();

    // 读取 server/instances 下心跳最新的实例
    let dir = home.join("server").join("instances");
    let mut best: Option<serde_json::Value> = None;
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    let hb = v["heartbeat_at"].as_u64().unwrap_or(0);
                    let best_hb = best
                        .as_ref()
                        .and_then(|b| b["heartbeat_at"].as_u64())
                        .unwrap_or(0);
                    if best.is_none() || hb >= best_hb {
                        best = Some(v);
                    }
                }
            }
        }
    }

    if let Some(inst) = best {
        let pid = inst["pid"].as_i64().unwrap_or(0) as i32;
        let host = inst["host"].as_str().unwrap_or("127.0.0.1").to_string();
        let port = inst["port"].as_u64().unwrap_or(58627);
        if pid > 0 && process_is_listening(pid) {
            info.running = true;
            info.url = format!("http://{}:{}", host, port);
        }
    }

    if let Ok(token) = fs::read_to_string(home.join("server.token")) {
        info.token = token.trim().to_string();
    }
    if info.running && !info.token.is_empty() {
        info.full_url = format!("{}/#token={}", info.url, info.token);
    }
    if std::env::var("KIMI_CODE_PASSWORD").is_ok() {
        info.note = "检测到 KIMI_CODE_PASSWORD 环境变量，网页登录以该密码为准".to_string();
    }
    info
}

fn get_local_kimi_version() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    let shell = "zsh";
    #[cfg(not(target_os = "macos"))]
    let shell = "bash";

    let wrapped = "export PATH=\"$HOME/.kimi-code/bin:$HOME/.kimi/bin:$HOME/.cargo/bin:$HOME/.local/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:$PATH\"; kimi --version".to_string();

    let output = std::process::Command::new(shell)
        .arg("-lc")
        .arg(&wrapped)
        .env("TERM", "xterm-256color")
        .output()
        .map_err(|e| format!("执行失败: {}", e))?;

    if !output.status.success() {
        return Err("kimi --version 执行失败".to_string());
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        return Err("无法解析 kimi 版本".to_string());
    }
    Ok(version)
}

async fn fetch_latest_version(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client
        .get(url)
        .header("User-Agent", "kimi-cheatsheet")
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("API 返回 {}", status));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析 JSON 失败: {}", e))?;
    let tag = json["tag_name"]
        .as_str()
        .ok_or("返回中没有 tag_name")?;
    let version = tag.split('@').last().unwrap_or(tag).to_string();
    Ok(version)
}

async fn get_latest_kimi_version() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    // 优先从 npm registry 获取版本，与 `kimi upgrade` 实际来源保持一致
    let npm_url = "https://registry.npmjs.org/@moonshot-ai/kimi-code";
    match client.get(npm_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(version) = json["dist-tags"]["latest"].as_str() {
                    return Ok(version.to_string());
                }
            }
        }
        _ => {}
    }

    // fallback 到 GitHub releases（带国内代理）
    let urls = vec![
        "https://api.github.com/repos/MoonshotAI/kimi-code/releases/latest",
        "https://ghproxy.com/https://api.github.com/repos/MoonshotAI/kimi-code/releases/latest",
    ];

    let mut last_err = String::new();
    for url in urls {
        match fetch_latest_version(&client, url).await {
            Ok(v) => return Ok(v),
            Err(e) => last_err = format!("{} -> {}", url, e),
        }
    }
    Err(format!("所有版本检测源均失败: {}", last_err))
}

#[tauri::command]
async fn check_kimi_version(app: AppHandle) -> Result<VersionInfo, String> {
    let mut info = VersionInfo::default();
    match get_local_kimi_version() {
        Ok(v) => info.local = v,
        Err(e) => {
            info.error = Some(e);
            save_version_last(&app, &info);
            update_tray_icon_for_version(&app, &info);
            return Ok(info);
        }
    }
    match get_latest_kimi_version().await {
        Ok(v) => {
            // 远程版本号可能带 v 前缀，本地版本可能不带，统一去掉 v 再比较
            let local_norm = info.local.trim_start_matches('v');
            let latest_norm = v.trim_start_matches('v');
            info.has_update = latest_norm != local_norm;
            info.latest = Some(v);
        }
        Err(e) => info.error = Some(e),
    }
    save_version_last(&app, &info);
    update_tray_icon_for_version(&app, &info);
    Ok(info)
}

fn save_version_last(app: &AppHandle, info: &VersionInfo) {
    if let Ok(path) = version_last_path(app) {
        if let Ok(content) = serde_json::to_string_pretty(info) {
            let _ = fs::write(&path, content);
        }
    }
}

#[tauri::command]
fn load_last_version(app: AppHandle) -> Result<VersionInfo, String> {
    let path = version_last_path(&app)?;
    if !path.exists() {
        return Ok(VersionInfo::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))
}

#[tauri::command]
async fn upgrade_kimi() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // 打开 Terminal 窗口执行升级，让用户能看到进度
        let script = r#"osascript -e 'tell application "Terminal" to do script "kimi upgrade"'"#;
        spawn_command(script)?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        spawn_command("kimi upgrade")?;
    }
    Ok(())
}

fn spawn_command(command: &str) -> Result<(), String> {
    if command.trim().is_empty() {
        return Err("启动命令为空".to_string());
    }
    // 使用非交互登录 shell，仅加载 profile 获取 PATH，避免加载 starship 等交互插件
    #[cfg(target_os = "macos")]
    let shell = "zsh";
    #[cfg(not(target_os = "macos"))]
    let shell = "bash";

    let wrapped = format!(
        "export PATH=\"$HOME/.kimi-code/bin:$HOME/.kimi/bin:$HOME/.cargo/bin:$HOME/.local/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:$PATH\"; {}",
        command
    );

    let mut child = std::process::Command::new(shell)
        .arg("-lc")
        .arg(&wrapped)
        .env("TERM", "xterm-256color")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动失败: {}", e))?;

    // 等一小会儿，若进程快速退出则把 stderr 返回给前端
    std::thread::sleep(std::time::Duration::from_millis(800));
    if let Ok(Some(status)) = child.try_wait() {
        if !status.success() {
            let mut stderr = String::new();
            if let Some(mut err) = child.stderr.take() {
                use std::io::Read;
                let _ = err.read_to_string(&mut stderr);
            }
            let code = status.code().map(|c| c.to_string()).unwrap_or_else(|| "未知".to_string());
            let detail = if stderr.trim().is_empty() { "无错误输出".to_string() } else { stderr.trim().to_string() };
            return Err(format!("启动命令退出 (code {}): {}", code, detail));
        }
    }

    // 立即 detach，避免阻塞 GUI
    let _ = child.try_wait();
    Ok(())
}

/// 杀掉匹配 args 的 kimi 进程
fn kill_kimi_processes(predicate: impl Fn(&str) -> bool) -> Result<(), String> {
    let procs = list_kimi_processes();
    let mut killed = false;
    for (pid, args) in procs {
        if predicate(&args) {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            killed = true;
        }
    }
    if killed {
        Ok(())
    } else {
        Err("没有匹配的进程可停止".to_string())
    }
}

#[tauri::command]
async fn stop_cli() -> Result<LaunchState, String> {
    // 先杀掉非 web/server 参数且没有在监听的 kimi/kimi-code 进程
    let procs = list_kimi_processes();
    let mut killed = false;
    for (pid, args) in procs {
        let is_web_or_server = args.contains("kimi web") || args.contains("kimi-code web") ||
            args.contains("kimi server") || args.contains("kimi-code server");
        if !is_web_or_server && !process_is_listening(pid) {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            killed = true;
        }
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut state = get_launch_state();
    // 若仍检测到 CLI 且没有 web 占用，尝试杀掉所有 kimi/kimi-code 进程
    if state.cli_running && !state.web_running {
        let _ = kill_kimi_processes(|_| true);
        tokio::time::sleep(Duration::from_millis(500)).await;
        state = get_launch_state();
    }
    if !killed && !state.cli_running {
        return Err("没有检测到 CLI 进程".to_string());
    }
    Ok(state)
}

#[tauri::command]
async fn stop_web() -> Result<LaunchState, String> {
    let procs = list_kimi_processes();
    let mut killed = false;

    // 先杀掉带 web/server 参数的进程
    for (pid, args) in &procs {
        if args.contains("kimi web") || args.contains("kimi-code web") ||
           args.contains("kimi server") || args.contains("kimi-code server") {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            killed = true;
        }
    }

    // 再杀掉正在监听的 kimi/kimi-code 进程
    for (pid, _) in &procs {
        if process_is_listening(*pid) {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            killed = true;
        }
    }

    if !killed {
        return Err("没有检测到 Web 进程".to_string());
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(get_launch_state())
}

#[tauri::command]
async fn launch_cli(app: AppHandle) -> Result<LaunchState, String> {
    let settings = load_app_settings(app)?;
    spawn_command(&settings.cli_command)?;
    // 给进程一点时间启动再检测
    tokio::time::sleep(Duration::from_millis(800)).await;
    Ok(get_launch_state())
}

#[tauri::command]
async fn launch_web(app: AppHandle) -> Result<LaunchState, String> {
    let settings = load_app_settings(app)?;
    let mut cmd = settings.web_command.clone();
    // 从 App 启动时由应用负责按用户选择的浏览器打开页面，
    // 给 kimi web 自动补 --no-open，避免它再用默认浏览器打开一次
    let is_kimi_web = cmd.contains("kimi web") || cmd.contains("kimi-code web");
    if is_kimi_web && !cmd.contains("--no-open") {
        cmd.push_str(" --no-open");
    }
    spawn_command(&cmd)?;

    // 等服务注册 instance 并就绪（最多约 10 秒），然后用选定浏览器打开
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let info = get_web_server_info();
        if info.running {
            let url = if !info.full_url.is_empty() {
                info.full_url.clone()
            } else {
                info.url.clone()
            };
            if !url.is_empty() {
                if let Err(e) = open_in_browser(&settings.browser, &url) {
                    log::warn!("open web ui failed: {}", e);
                }
            }
            break;
        }
    }
    Ok(get_launch_state())
}

/// 用指定浏览器打开 URL；browser 为空时用系统默认浏览器
fn open_in_browser(browser: &str, url: &str) -> Result<(), String> {
    let mut cmd = std::process::Command::new("open");
    let b = browser.trim();
    if !b.is_empty() {
        cmd.arg("-a").arg(b);
    }
    cmd.arg(url);
    cmd.spawn().map_err(|e| format!("打开浏览器失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn open_web_ui(app: AppHandle) -> Result<(), String> {
    let settings = load_app_settings(app)?;
    let info = get_web_server_info();
    if !info.running {
        return Err("Web 服务未运行".to_string());
    }
    let url = if !info.full_url.is_empty() {
        info.full_url
    } else {
        info.url
    };
    open_in_browser(&settings.browser, &url)
}

/// 扫描本机已安装的常见浏览器（/Applications 与 ~/Applications）
#[tauri::command]
fn list_browsers() -> Vec<String> {
    const KNOWN: &[&str] = &[
        "Safari",
        "Google Chrome",
        "Microsoft Edge",
        "Arc",
        "Firefox",
        "Brave Browser",
        "Opera",
        "Vivaldi",
        "Chromium",
        "Orion",
        "Dia",
        "Comet",
        "Zen",
    ];
    let mut found: Vec<String> = Vec::new();
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    for dir in dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(app) = name.strip_suffix(".app") {
                    if KNOWN.iter().any(|k| k.eq_ignore_ascii_case(app))
                        && !found.iter().any(|f| f == app)
                    {
                        found.push(app.to_string());
                    }
                }
            }
        }
    }
    found.sort();
    found
}

async fn fetch_kimi_quota(api_key: &str) -> Result<QuotaResult, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .get("https://api.kimi.com/coding/v1/usages")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(QuotaResult {
            success: false,
            error: Some(format!("API Key 无效 (HTTP {})", status.as_u16())),
            ..Default::default()
        });
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Ok(QuotaResult {
            success: false,
            error: Some(format!("API 错误 (HTTP {}): {}", status.as_u16(), body)),
            ..Default::default()
        });
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let five_hour = body
        .get("limits")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("detail"))
        .map(parse_tier)
        .map(|mut t| {
            t.name = "5h".to_string();
            t
        });

    let weekly = body
        .get("usage")
        .map(parse_tier)
        .map(|mut t| {
            t.name = "W".to_string();
            t
        });

    Ok(QuotaResult {
        success: true,
        error: None,
        fetched_at: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        weekly,
        five_hour,
        raw: body,
    })
}

fn parse_tier(value: &serde_json::Value) -> QuotaTier {
    let limit = value
        .get("limit")
        .and_then(|v| v.as_f64())
        .or_else(|| value.get("limit").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))
        .unwrap_or(0.0);
    let remaining = value
        .get("remaining")
        .and_then(|v| v.as_f64())
        .or_else(|| value.get("remaining").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))
        .unwrap_or(0.0);
    let reset_time = value
        .get("resetTime")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let percentage = if limit > 0.0 {
        ((remaining / limit) * 100.0).round()
    } else {
        0.0
    };
    QuotaTier {
        name: String::new(),
        limit,
        remaining,
        reset_time,
        percentage,
    }
}

fn update_tray_title(app: &AppHandle, result: &QuotaResult, settings: &QuotaSettings) {
    let title = if !settings.enabled {
        "K".to_string()
    } else if !result.success {
        "K !".to_string()
    } else {
        let mut parts = Vec::new();
        if let Some(t) = &result.five_hour {
            parts.push(format!("5h:{}%", t.percentage as i64));
        }
        if let Some(t) = &result.weekly {
            parts.push(format!("W:{}%", t.percentage as i64));
        }
        let reset_parts: Vec<String> = [
            result.five_hour.as_ref().and_then(|t| compact_reset(&t.reset_time)),
            result.weekly.as_ref().and_then(|t| compact_reset(&t.reset_time)),
        ]
        .into_iter()
        .flatten()
        .collect();
        if parts.is_empty() {
            "K ...".to_string()
        } else {
            let mut title = format!("K {}", parts.join(" "));
            if !reset_parts.is_empty() {
                title.push_str(&format!(" {}", reset_parts.join("/")));
            }
            title
        }
    };

    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(Some(&title));
        let tooltip = build_tooltip(result, settings);
        if let Err(e) = tray.set_tooltip(Some(&tooltip)) {
            log::warn!("failed to set tray tooltip: {}", e);
        }
    }
}

fn compact_reset(reset_time: &Option<String>) -> Option<String> {
    let s = reset_time.as_deref()?;
    let dt = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    let now = chrono::Utc::now();
    let dur = dt.with_timezone(&chrono::Utc) - now;
    let secs = dur.num_seconds();
    if secs <= 0 {
        return Some("0".to_string());
    }
    if secs < 3600 {
        // < 1 小时：27m
        Some(format!("{}m", secs / 60))
    } else if secs < 86400 {
        // ≤ 1 天：始终显示分钟，1h27m / 1h0m
        Some(format!("{}h{}m", secs / 3600, (secs % 3600) / 60))
    } else if secs < 604800 {
        // 1~7 天（周额度常见）：5d3h，0 小时只显示天数
        let d = secs / 86400;
        let h = (secs % 86400) / 3600;
        if h == 0 {
            Some(format!("{}d", d))
        } else {
            Some(format!("{}d{}h", d, h))
        }
    } else {
        // ≥ 7 天备用：07-12（月-日）
        Some(dt.with_timezone(&chrono::Local).format("%m-%d").to_string())
    }
}

fn build_tooltip(result: &QuotaResult, settings: &QuotaSettings) -> String {
    if !settings.enabled {
        return "Kimi 命令速查".to_string();
    }
    if !result.success {
        return format!(
            "Kimi 额度查询失败: {}",
            result.error.as_deref().unwrap_or("未知错误")
        );
    }
    let mut parts = Vec::new();
    if let Some(t) = &result.five_hour {
        parts.push(format!(
            "5小时 {}/{} ({}%){}",
            fmt_num(t.remaining),
            fmt_num(t.limit),
            t.percentage as i64,
            time_until_reset(&t.reset_time).replace(" · ", " ")
        ));
    }
    if let Some(t) = &result.weekly {
        parts.push(format!(
            "周额度 {}/{} ({}%){}",
            fmt_num(t.remaining),
            fmt_num(t.limit),
            t.percentage as i64,
            time_until_reset(&t.reset_time).replace(" · ", " ")
        ));
    }
    if let Some(ts) = &result.fetched_at {
        parts.push(format!("更新于 {}", ts));
    }
    if parts.is_empty() {
        "Kimi 额度".to_string()
    } else {
        parts.join(" | ")
    }
}

#[cfg(target_os = "macos")]
fn set_macos_window_auto_hide(window: &tauri::WebviewWindow) {
    use objc::runtime::{Object, YES};
    use objc::{msg_send, sel};
    unsafe {
        if let Ok(ns_window) = window.ns_window() {
            let ns_window = ns_window as *mut Object;
            let _: () = msg_send![ns_window, setHidesOnDeactivate: YES];
        }
    }
}

fn time_until_reset(reset_time: &Option<String>) -> String {
    let Some(s) = reset_time else {
        return String::new();
    };
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) else {
        return String::new();
    };
    let now = chrono::Utc::now();
    let dur = dt.with_timezone(&chrono::Utc) - now;
    let secs = dur.num_seconds();
    if secs <= 0 {
        return " · 即将重置".to_string();
    }
    if secs < 3600 {
        format!(" · {}分钟后重置", secs / 60)
    } else if secs < 86400 {
        format!(" · {}小时{}分钟后重置", secs / 3600, (secs % 3600) / 60)
    } else {
        format!(" · {}天{}小时后重置", secs / 86400, (secs % 86400) / 3600)
    }
}

fn fmt_num(n: f64) -> String {
    if n == n.trunc() {
        format!("{:.0}", n)
    } else {
        format!("{:.2}", n)
    }
}

fn start_quota_refresher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // 启动时先读取设置和上次结果，恢复托盘标题
        let settings = load_quota_settings(app.clone()).unwrap_or_default();
        let last = load_last_quota(app.clone()).unwrap_or_default();
        update_tray_title(&app, &last, &settings);

        // 版本检测节流：与额度刷新同步，但两次检测至少间隔 5 分钟
        let mut last_version_check: Option<std::time::Instant> = None;

        loop {
            let settings = load_quota_settings(app.clone()).unwrap_or_default();
            if settings.enabled && settings.interval_minutes > 0 && !settings.api_key.trim().is_empty() {
                let _ = refresh_quota(app.clone()).await;

                // 额度刷新后顺便检测版本
                let should_check = match last_version_check {
                    None => true,
                    Some(t) => t.elapsed().as_secs() >= 300,
                };
                if should_check {
                    if let Ok(info) = check_kimi_version(app.clone()).await {
                        update_tray_icon_for_version(&app, &info);
                    }
                    last_version_check = Some(std::time::Instant::now());
                }
            }
            // 监听设置变化事件，用于立即响应用户修改
            let sleep_secs = if settings.enabled && settings.interval_minutes > 0 {
                settings.interval_minutes * 60
            } else {
                60
            };
            tokio::time::sleep(Duration::from_secs(sleep_secs.min(3600))).await;
        }
    });
}

fn default_commands() -> Vec<CommandItem> {
    vec![
        // ═══════════════════════════════════════════════════════════════════════
        // 0.23.1 实测可用命令（已用 kimicode 0.23.1 --help 核对）
        // ═══════════════════════════════════════════════════════════════════════

        // ── 启动命令 / 基本 ─────────────────────────────────────────────────────
        CommandItem { cat: "启动命令".into(), cmd: "kimi".into(), desc: "启动新的交互式会话".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi --version".into(), desc: "显示版本号".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi -V".into(), desc: "--version 简写".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi --help".into(), desc: "显示帮助".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi -h".into(), desc: "--help 简写".into() },

        // ── 启动命令 / 模型 ─────────────────────────────────────────────────────
        CommandItem { cat: "启动命令".into(), cmd: "kimi --model <model>".into(), desc: "指定本次使用的模型".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi -m <model>".into(), desc: "--model 简写".into() },

        // ── 启动命令 / 工作目录 ─────────────────────────────────────────────────
        CommandItem { cat: "启动命令".into(), cmd: "kimi --add-dir <dir>".into(), desc: "添加额外工作目录（可重复）".into() },

        // ── 启动命令 / 会话 ─────────────────────────────────────────────────────
        CommandItem { cat: "启动命令".into(), cmd: "kimi --continue".into(), desc: "继续当前目录最近的会话".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi -c".into(), desc: "--continue 简写".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi --session [id]".into(), desc: "恢复指定会话".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi -S [id]".into(), desc: "--session 简写".into() },

        // ── 启动命令 / 输入与模式 ───────────────────────────────────────────────
        CommandItem { cat: "启动命令".into(), cmd: "kimi --prompt \"...\"".into(), desc: "非交互执行单次提问".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi -p \"...\"".into(), desc: "--prompt 简写".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi --output-format <format>".into(), desc: "输出格式：text|stream-json".into() },

        // ── 启动命令 / 审批与计划 ───────────────────────────────────────────────
        CommandItem { cat: "启动命令".into(), cmd: "kimi --yolo".into(), desc: "自动批准所有工具调用".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi -y".into(), desc: "--yolo 简写".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi --auto".into(), desc: "以 auto 权限模式启动".into() },
        CommandItem { cat: "启动命令".into(), cmd: "kimi --plan".into(), desc: "以 Plan 模式启动".into() },

        // ── 启动命令 / 技能 ─────────────────────────────────────────────────────
        CommandItem { cat: "启动命令".into(), cmd: "kimi --skills-dir <dir>".into(), desc: "从指定目录加载 Skills（可重复）".into() },

        // ── 子命令 / 账号 ───────────────────────────────────────────────────────
        CommandItem { cat: "子命令 / 账号".into(), cmd: "kimi login".into(), desc: "登录 Kimi 账号".into() },

        // ── 子命令 / acp ────────────────────────────────────────────────────────
        CommandItem { cat: "子命令 / acp".into(), cmd: "kimi acp".into(), desc: "以 ACP 协议运行，供 IDE 接入".into() },
        CommandItem { cat: "子命令 / acp".into(), cmd: "kimi acp --login".into(), desc: "仅执行 device-code 登录流程".into() },

        // ── 子命令 / export ─────────────────────────────────────────────────────
        CommandItem { cat: "子命令 / export".into(), cmd: "kimi export [sessionId]".into(), desc: "导出会话为 ZIP".into() },
        CommandItem { cat: "子命令 / export".into(), cmd: "kimi export -o <path>".into(), desc: "指定输出 ZIP 路径".into() },
        CommandItem { cat: "子命令 / export".into(), cmd: "kimi export -y".into(), desc: "跳过确认直接导出".into() },
        CommandItem { cat: "子命令 / export".into(), cmd: "kimi export --no-include-global-log".into(), desc: "不包含全局诊断日志".into() },

        // ── 子命令 / vis ────────────────────────────────────────────────────────
        CommandItem { cat: "子命令 / vis".into(), cmd: "kimi vis [sessionId]".into(), desc: "启动会话可视化浏览器".into() },
        CommandItem { cat: "子命令 / vis".into(), cmd: "kimi vis --port <number>".into(), desc: "指定 vis 服务端口（默认自动）".into() },
        CommandItem { cat: "子命令 / vis".into(), cmd: "kimi vis --host <host>".into(), desc: "指定 vis 绑定主机（默认 127.0.0.1）".into() },
        CommandItem { cat: "子命令 / vis".into(), cmd: "kimi vis --no-open".into(), desc: "不自动打开浏览器".into() },

        // ── 子命令 / web ────────────────────────────────────────────────────────
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web".into(), desc: "启动 Web UI 并打开浏览器（默认端口 58627）".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --no-open".into(), desc: "启动 Web UI 但不打开浏览器".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --port <port>".into(), desc: "指定绑定端口".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --host <host>".into(), desc: "指定绑定主机；--host 单独使用表示 0.0.0.0".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --foreground".into(), desc: "前台运行并保持终端附着".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --keep-alive".into(), desc: "无客户端 60 秒后仍保持运行".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --allowed-host <host>".into(), desc: "允许额外 Host 头（可重复/逗号分隔）".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --insecure-no-tls".into(), desc: "非 loopback 绑定时不强制 TLS".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --allow-remote-shutdown".into(), desc: "保留远程 shutdown 接口".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --allow-remote-terminals".into(), desc: "保留远程 PTY 终端接口".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --dangerous-bypass-auth".into(), desc: "关闭 bearer token 鉴权（仅可信网络）".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --log-level <level>".into(), desc: "服务日志级别 fatal|error|warn|info|debug|trace|silent".into() },
        CommandItem { cat: "子命令 / web".into(), cmd: "kimi web --debug-endpoints".into(), desc: "挂载 /api/v1/debug/*".into() },

        // ── 子命令 / server ─────────────────────────────────────────────────────
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run".into(), desc: "启动本地 Kimi 服务（后台守护进程）".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --foreground".into(), desc: "前台运行本地服务".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --open".into(), desc: "启动服务并打开浏览器".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --port <port>".into(), desc: "指定绑定端口（默认 58627）".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --host [host]".into(), desc: "绑定主机，--host 单独使用表示 0.0.0.0".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --allowed-host <host>".into(), desc: "允许额外 Host 头".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --keep-alive".into(), desc: "无客户端连接 60 秒后仍保持运行".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --insecure-no-tls".into(), desc: "非 loopback 绑定时不强制 TLS".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --allow-remote-shutdown".into(), desc: "保留远程 shutdown 接口".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --allow-remote-terminals".into(), desc: "保留远程 PTY 终端接口".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --dangerous-bypass-auth".into(), desc: "关闭 bearer token 鉴权".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --log-level <level>".into(), desc: "服务日志级别".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server run --debug-endpoints".into(), desc: "挂载 /api/v1/debug/*".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server ps".into(), desc: "列出当前连接到 Kimi server 的客户端".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server ps --json".into(), desc: "以 JSON 输出连接列表".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server kill".into(), desc: "停止正在运行的 Kimi server / Web UI".into() },
        CommandItem { cat: "子命令 / server".into(), cmd: "kimi server rotate-token".into(), desc: "重新生成持久 server token".into() },

        // ── 子命令 / provider ───────────────────────────────────────────────────
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider list".into(), desc: "列出已配置供应商".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider list --json".into(), desc: "以 JSON 输出供应商列表".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider add <url>".into(), desc: "从 registry 添加供应商".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider add <url> --api-key <key>".into(), desc: "指定 registry 访问 token".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider remove <id>".into(), desc: "删除供应商".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog list".into(), desc: "浏览公开模型目录".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog list <providerId>".into(), desc: "查看指定供应商的模型".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog list --filter <substring>".into(), desc: "按关键字过滤模型目录".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog list --url <url>".into(), desc: "覆盖默认目录地址".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog list --json".into(), desc: "以 JSON 输出目录".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog add <id>".into(), desc: "从目录导入供应商".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog add <id> --api-key <key>".into(), desc: "导入时指定 API key".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog add <id> --default-model <modelId>".into(), desc: "导入后设为默认模型".into() },
        CommandItem { cat: "子命令 / provider".into(), cmd: "kimi provider catalog add <id> --url <url>".into(), desc: "覆盖默认目录地址".into() },

        // ── 子命令 / 其他 ───────────────────────────────────────────────────────
        CommandItem { cat: "子命令 / 其他".into(), cmd: "kimi doctor".into(), desc: "校验配置文件".into() },
        CommandItem { cat: "子命令 / 其他".into(), cmd: "kimi doctor config [path]".into(), desc: "校验 config.toml".into() },
        CommandItem { cat: "子命令 / 其他".into(), cmd: "kimi doctor tui [path]".into(), desc: "校验 tui.toml".into() },
        CommandItem { cat: "子命令 / 其他".into(), cmd: "kimi migrate".into(), desc: "从旧版 kimi-cli 迁移数据".into() },
        CommandItem { cat: "子命令 / 其他".into(), cmd: "kimi upgrade".into(), desc: "检查并升级 CLI 到最新版".into() },
        CommandItem { cat: "子命令 / 其他".into(), cmd: "kimi update".into(), desc: "upgrade 别名".into() },

        // ═══════════════════════════════════════════════════════════════════════
        // 新版预览（0.23.1 不支持，仅作参考）
        // ═══════════════════════════════════════════════════════════════════════

        // ── 新版预览 / 全局启动参数 ───────────────────────────────────────────────
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --verbose".into(), desc: "输出详细运行时信息".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --debug".into(), desc: "记录调试日志".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --agent <name>".into(), desc: "使用内置 agent：default|okabe".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --agent-file <path>".into(), desc: "使用自定义 agent 文件".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --config <toml|json>".into(), desc: "加载 TOML/JSON 配置字符串".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --config-file <path>".into(), desc: "加载配置文件".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --work-dir <path>".into(), desc: "指定工作目录".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi -w <path>".into(), desc: "--work-dir 简写".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --resume [id]".into(), desc: "恢复指定会话（--session 别名）".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi -r [id]".into(), desc: "--resume 简写".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --command \"...\"".into(), desc: "--prompt 别名".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --input-format <format>".into(), desc: "输入格式：text|stream-json".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --final-message-only".into(), desc: "只输出最终助手消息".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --print".into(), desc: "以 print 模式运行".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --quiet".into(), desc: "--print --output-format text --final-message-only".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --acp".into(), desc: "以 ACP 服务器模式运行".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --wire".into(), desc: "以 Wire 服务器模式运行".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --yes".into(), desc: "--yolo 别名".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --auto-approve".into(), desc: "--yolo 别名".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --afk".into(), desc: "离座模式".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --thinking".into(), desc: "启用思考模式".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --no-thinking".into(), desc: "禁用思考模式".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --max-steps-per-turn <n>".into(), desc: "每轮最大步数".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --max-retries-per-step <n>".into(), desc: "每步最大重试次数".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --max-ralph-iterations <n>".into(), desc: "Ralph Loop 迭代次数".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --mcp-config-file <path>".into(), desc: "加载 MCP 配置文件".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi --mcp-config <json>".into(), desc: "加载 MCP 配置 JSON 字符串".into() },

        // ── 新版预览 / 子命令 ───────────────────────────────────────────────────
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi logout".into(), desc: "退出登录并清除 OAuth 凭证".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi info".into(), desc: "显示版本和协议信息".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi info --json".into(), desc: "以 JSON 输出版本信息".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi mcp add <name> -- <cmd>".into(), desc: "添加 stdio MCP 服务器".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi mcp list".into(), desc: "列出已配置 MCP 服务器".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi plugin install <path|url|git>".into(), desc: "安装插件".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi term".into(), desc: "启动 Toad 图形终端 UI".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web --network".into(), desc: "绑定 0.0.0.0 并显示 LAN IP".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web -n".into(), desc: "--network 简写".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web --reload".into(), desc: "开发模式自动重载".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web --auth-token <token>".into(), desc: "设置 API Bearer Token".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web --allowed-origins <origins>".into(), desc: "允许跨域 Origin 列表".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web --lan-only".into(), desc: "仅允许局域网访问".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web --public".into(), desc: "允许公网访问".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web --restrict-sensitive-apis".into(), desc: "限制敏感 API".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi web --dangerously-omit-auth".into(), desc: "完全禁用认证".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi vis --network".into(), desc: "绑定 0.0.0.0 并显示 LAN IP".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi vis -n".into(), desc: "--network 简写".into() },
        CommandItem { cat: "新版预览（0.23.1 不支持）".into(), cmd: "kimi vis --reload".into(), desc: "开发模式自动重载".into() },

        // ── TUI 斜杠命令 / 账号与配置 ─────────────────────────────────────────────
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/login".into(), desc: "登录".into() },
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/logout".into(), desc: "退出登录".into() },
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/provider".into(), desc: "管理模型供应商".into() },
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/model".into(), desc: "切换当前模型".into() },
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/settings".into(), desc: "打开设置面板".into() },
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/config".into(), desc: "/settings 别名".into() },
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/permission".into(), desc: "切换权限模式".into() },
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/theme".into(), desc: "切换主题".into() },
        CommandItem { cat: "TUI 斜杠命令 / 账号与配置".into(), cmd: "/editor".into(), desc: "配置外部编辑器".into() },

        // ── TUI 斜杠命令 / 会话管理 ───────────────────────────────────────────────
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/new".into(), desc: "开启新会话".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/clear".into(), desc: "/new 别名".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/sessions".into(), desc: "浏览历史会话".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/resume".into(), desc: "/sessions 别名".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/fork".into(), desc: "复制当前会话为新会话".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/title [text]".into(), desc: "查看或设置会话标题".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/rename".into(), desc: "/title 别名".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/compact [instruction]".into(), desc: "压缩上下文释放 token".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/undo [count]".into(), desc: "撤销最近几条消息".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/init".into(), desc: "分析代码库并生成 AGENTS.md".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/export-md [path]".into(), desc: "导出会话为 Markdown".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/export".into(), desc: "/export-md 别名".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/export-debug-zip".into(), desc: "导出调试 ZIP".into() },
        CommandItem { cat: "TUI 斜杠命令 / 会话管理".into(), cmd: "/add-dir [path]".into(), desc: "添加工作目录".into() },

        // ── TUI 斜杠命令 / 模式与运行控制 ─────────────────────────────────────────
        CommandItem { cat: "TUI 斜杠命令 / 模式与运行控制".into(), cmd: "/yolo [on/off]".into(), desc: "切换 YOLO 自动批准模式".into() },
        CommandItem { cat: "TUI 斜杠命令 / 模式与运行控制".into(), cmd: "/yes".into(), desc: "/yolo 别名".into() },
        CommandItem { cat: "TUI 斜杠命令 / 模式与运行控制".into(), cmd: "/auto [on/off]".into(), desc: "切换 auto 权限模式".into() },
        CommandItem { cat: "TUI 斜杠命令 / 模式与运行控制".into(), cmd: "/plan [on/off]".into(), desc: "切换 Plan 模式".into() },
        CommandItem { cat: "TUI 斜杠命令 / 模式与运行控制".into(), cmd: "/plan clear".into(), desc: "清除当前计划".into() },
        CommandItem { cat: "TUI 斜杠命令 / 模式与运行控制".into(), cmd: "/swarm on/off".into(), desc: "开启/关闭子代理群".into() },
        CommandItem { cat: "TUI 斜杠命令 / 模式与运行控制".into(), cmd: "/swarm <task>".into(), desc: "用子代理群执行单次任务".into() },

        // ── TUI 斜杠命令 / 目标模式 ───────────────────────────────────────────────
        CommandItem { cat: "TUI 斜杠命令 / 目标模式".into(), cmd: "/goal <目标>".into(), desc: "创建持久目标，自动续跑".into() },
        CommandItem { cat: "TUI 斜杠命令 / 目标模式".into(), cmd: "/goal status".into(), desc: "查看目标状态".into() },
        CommandItem { cat: "TUI 斜杠命令 / 目标模式".into(), cmd: "/goal pause".into(), desc: "暂停目标".into() },
        CommandItem { cat: "TUI 斜杠命令 / 目标模式".into(), cmd: "/goal resume".into(), desc: "继续目标".into() },
        CommandItem { cat: "TUI 斜杠命令 / 目标模式".into(), cmd: "/goal cancel".into(), desc: "取消目标".into() },
        CommandItem { cat: "TUI 斜杠命令 / 目标模式".into(), cmd: "/goal replace <目标>".into(), desc: "替换当前目标".into() },
        CommandItem { cat: "TUI 斜杠命令 / 目标模式".into(), cmd: "/goal next <目标>".into(), desc: "安排后续目标".into() },

        // ── TUI 斜杠命令 / 信息与状态 ─────────────────────────────────────────────
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/help".into(), desc: "显示快捷键和命令".into() },
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/h".into(), desc: "/help 别名".into() },
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/btw [问题]".into(), desc: "旁路对话（不影响主会话）".into() },
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/usage".into(), desc: "查看 token 用量和配额".into() },
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/status".into(), desc: "查看当前会话状态".into() },
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/mcp".into(), desc: "查看 MCP server 连接状态".into() },
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/plugins".into(), desc: "管理插件".into() },
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/version".into(), desc: "显示版本号".into() },
        CommandItem { cat: "TUI 斜杠命令 / 信息与状态".into(), cmd: "/feedback".into(), desc: "提交反馈".into() },

        // ── TUI 斜杠命令 / 退出 ───────────────────────────────────────────────────
        CommandItem { cat: "TUI 斜杠命令 / 退出".into(), cmd: "/exit".into(), desc: "退出 CLI".into() },
        CommandItem { cat: "TUI 斜杠命令 / 退出".into(), cmd: "/quit".into(), desc: "/exit 别名".into() },
        CommandItem { cat: "TUI 斜杠命令 / 退出".into(), cmd: "/q".into(), desc: "/exit 别名".into() },

        // ── 内置 Skill 命令 ───────────────────────────────────────────────────────
        CommandItem { cat: "内置 Skill 命令".into(), cmd: "/mcp-config".into(), desc: "配置 MCP server".into() },
        CommandItem { cat: "内置 Skill 命令".into(), cmd: "/update-config".into(), desc: "编辑 config.toml / tui.toml".into() },
        CommandItem { cat: "内置 Skill 命令".into(), cmd: "/custom-theme".into(), desc: "创建自定义主题".into() },
        CommandItem { cat: "内置 Skill 命令".into(), cmd: "/import-from-cc-codex".into(), desc: "从 Claude Code/Codex 导入配置".into() },
        CommandItem { cat: "内置 Skill 命令".into(), cmd: "/sub-skill".into(), desc: "重组本地 skill 库".into() },
    ]
}

fn position_window_at_tray(app: &AppHandle, window: &tauri::WebviewWindow) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(Some(tray_bounds)) = tray.rect() {
            let window_size = window.outer_size().unwrap_or(tauri::PhysicalSize::new(520, 720));
            let (tray_x, tray_y) = match tray_bounds.position {
                tauri::Position::Physical(p) => (p.x as i32, p.y as i32),
                tauri::Position::Logical(p) => (p.x as i32, p.y as i32),
            };
            let (tray_w, tray_h) = match tray_bounds.size {
                tauri::Size::Physical(s) => (s.width as i32, s.height as i32),
                tauri::Size::Logical(s) => (s.width as i32, s.height as i32),
            };
            let x = tray_x + (tray_w / 2) - (window_size.width as i32 / 2);
            let y = tray_y + tray_h + 4;
            let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
        }
    }
}

fn load_tray_icon() -> tauri::image::Image<'static> {
    let rgba = include_bytes!("../icons/tray-icon.rgba");
    tauri::image::Image::new(rgba, 32, 32)
}

fn load_red_tray_icon() -> tauri::image::Image<'static> {
    let rgba = include_bytes!("../icons/tray-icon.rgba");
    let mut red = rgba.to_vec();
    for chunk in red.chunks_exact_mut(4) {
        let alpha = chunk[3];
        if alpha > 0 {
            chunk[0] = 255;
            chunk[1] = 0;
            chunk[2] = 0;
            chunk[3] = alpha;
        }
    }
    tauri::image::Image::new(Box::leak(red.into_boxed_slice()), 32, 32)
}

fn update_tray_icon_for_version(app: &AppHandle, info: &VersionInfo) {
    if let Some(tray) = app.tray_by_id("main") {
        let icon = if info.has_update {
            load_red_tray_icon()
        } else {
            load_tray_icon()
        };
        let _ = tray.set_icon(Some(icon));
    }
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        position_window_at_tray(app, &window);
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("reset-ui", ());
    }
}

fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            position_window_at_tray(app, &window);
            let _ = window.show();
            let _ = window.set_focus();
            let _ = app.emit("reset-ui", ());
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            load_commands,
            save_commands,
            export_commands,
            import_commands,
            get_autostart_status,
            set_autostart,
            load_quota_settings,
            save_quota_settings,
            refresh_quota,
            load_last_quota,
            load_app_settings,
            save_app_settings,
            get_launch_state,
            get_web_server_info,
            launch_cli,
            launch_web,
            open_web_ui,
            list_browsers,
            stop_cli,
            stop_web,
            check_kimi_version,
            load_last_version,
            upgrade_kimi
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                let _ = app.set_dock_visibility(false);
            }

            let show_i = MenuItem::with_id(app, "show", "显示速查窗口", true, None::<&str>)?;
            let launch_cli_i = MenuItem::with_id(app, "launch_cli", "启动 CLI (检测中...)", true, None::<&str>)?;
            let launch_web_i = MenuItem::with_id(app, "launch_web", "启动 Web (检测中...)", true, None::<&str>)?;
            let edit_i = MenuItem::with_id(app, "edit", "编辑命令", true, None::<&str>)?;
            let export_i = MenuItem::with_id(app, "export", "导出命令", true, None::<&str>)?;
            let import_i = MenuItem::with_id(app, "import", "导入命令", true, None::<&str>)?;
            let refresh_quota_i = MenuItem::with_id(app, "refresh_quota", "刷新额度", true, None::<&str>)?;
            let quota_settings_i = MenuItem::with_id(app, "quota_settings", "用量查询设置...", true, None::<&str>)?;
            let launch_settings_i = MenuItem::with_id(app, "launch_settings", "启动设置...", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let sep3 = PredefinedMenuItem::separator(app)?;
            let autostart_i = MenuItem::with_id(app, "autostart", "开机启动", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

            let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            let _ = autostart_i.set_text(if autostart_enabled {
                "开机启动 ✓"
            } else {
                "开机启动"
            });

            let menu = Menu::with_items(
                app,
                &[
                    &show_i,
                    &launch_cli_i,
                    &launch_web_i,
                    &edit_i,
                    &export_i,
                    &import_i,
                    &sep,
                    &refresh_quota_i,
                    &quota_settings_i,
                    &launch_settings_i,
                    &sep2,
                    &autostart_i,
                    &sep3,
                    &quit_i,
                ],
            )?;

            let webview_window = app
                .get_webview_window("main")
                .expect("main window not found");
            let main_window = webview_window.as_ref().window();

            #[cfg(target_os = "macos")]
            set_macos_window_auto_hide(&webview_window);

            let window_for_close = webview_window.clone();
            let window_for_focus = webview_window.clone();
            webview_window.on_window_event(move |event| {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = window_for_close.hide();
                    }
                    tauri::WindowEvent::Focused(false) => {
                        let _ = window_for_focus.hide();
                    }
                    _ => {}
                }
            });

            // 启动后台自动刷新
            start_quota_refresher(app.handle().clone());

            // 启动时根据上次版本检测结果设置托盘图标颜色
            let last_version = load_last_version(app.handle().clone()).unwrap_or_default();
            update_tray_icon_for_version(app.handle(), &last_version);

            let menu_for_popup = menu.clone();
            let launch_cli_for_popup = launch_cli_i.clone();
            let launch_web_for_popup = launch_web_i.clone();
            let launch_cli_for_menu = launch_cli_i.clone();
            let launch_web_for_menu = launch_web_i.clone();

            let tray_icon = load_tray_icon();

            let tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(false)
                .title("Kimi")
                .tooltip("Kimi 命令速查")
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        match button {
                            MouseButton::Left => {
                                // 左键点击立即刷新额度（不受自动刷新间隔限制）
                                let app_clone = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    let _ = refresh_quota(app_clone).await;
                                });
                                show_window(app);
                            }
                            MouseButton::Right => {
                                // 如果主窗口正显示，先隐藏它
                                if let Some(window) = app.get_webview_window("main") {
                                    if window.is_visible().unwrap_or(false) {
                                        let _ = window.hide();
                                    }
                                }
                                // 更新启动项状态与菜单文字后再弹出菜单
                                let state = get_launch_state();
                                let cli_text = if state.cli_running {
                                    "停止 CLI (运行中)".to_string()
                                } else {
                                    "启动 CLI (未运行)".to_string()
                                };
                                let web_text = if state.web_running {
                                    "停止 Web (运行中)".to_string()
                                } else {
                                    "启动 Web (未运行)".to_string()
                                };
                                let _ = launch_cli_for_popup.set_text(&cli_text);
                                let _ = launch_web_for_popup.set_text(&web_text);
                                let _ = menu_for_popup.popup(main_window.clone());
                            }
                            _ => {}
                        }
                    }
                })
                .on_menu_event(move |app, event| {
                    let id = event.id.as_ref();
                    match id {
                        "show" => show_window(app),
                        "launch_cli" => {
                            let app_clone = app.clone();
                            let item = launch_cli_for_menu.clone();
                            tauri::async_runtime::spawn(async move {
                                let state = get_launch_state();
                                let result = if state.cli_running {
                                    stop_cli().await
                                } else {
                                    launch_cli(app_clone).await
                                };
                                match result {
                                    Ok(state) => {
                                        let text = if state.cli_running {
                                            "停止 CLI (运行中)".to_string()
                                        } else {
                                            "启动 CLI (未运行)".to_string()
                                        };
                                        let _ = item.set_text(&text);
                                    }
                                    Err(e) => {
                                        log::warn!("toggle cli failed: {}", e);
                                        let _ = item.set_text("CLI (操作失败)");
                                    }
                                }
                            });
                        }
                        "launch_web" => {
                            let app_clone = app.clone();
                            let item = launch_web_for_menu.clone();
                            tauri::async_runtime::spawn(async move {
                                let state = get_launch_state();
                                let result = if state.web_running {
                                    stop_web().await
                                } else {
                                    launch_web(app_clone).await
                                };
                                match result {
                                    Ok(state) => {
                                        let text = if state.web_running {
                                            "停止 Web (运行中)".to_string()
                                        } else {
                                            "启动 Web (未运行)".to_string()
                                        };
                                        let _ = item.set_text(&text);
                                    }
                                    Err(e) => {
                                        log::warn!("toggle web failed: {}", e);
                                        let _ = item.set_text("Web (操作失败)");
                                    }
                                }
                            });
                        }
                        "edit" => {
                            toggle_window(app);
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.eval("window.openEditor && window.openEditor()");
                            }
                        }
                        "export" => {
                            let _ = app.emit("menu-export", ());
                        }
                        "import" => {
                            let _ = app.emit("menu-import", ());
                        }
                        "refresh_quota" => {
                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = refresh_quota(app_clone).await;
                            });
                        }
                        "quota_settings" => {
                            toggle_window(app);
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.eval("window.openQuotaSettings && window.openQuotaSettings()");
                            }
                        }
                        "launch_settings" => {
                            toggle_window(app);
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.eval("window.openLaunchSettings && window.openLaunchSettings()");
                            }
                        }
                        "autostart" => {
                            let manager = app.autolaunch();
                            let currently_enabled = manager.is_enabled().unwrap_or(false);
                            let new_state = !currently_enabled;
                            let result = if new_state {
                                manager.enable()
                            } else {
                                manager.disable()
                            };
                            if result.is_ok() {
                                let _ = autostart_i.set_text(if new_state {
                                    "开机启动 ✓"
                                } else {
                                    "开机启动"
                                });
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .build(app);
            if let Err(e) = tray {
                log::error!("failed to create tray icon: {}", e);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
