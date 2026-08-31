use image::ImageEncoder;
use tauri::Manager;
use tauri::PhysicalPosition;

/// 解码图片字节（png/jpg/bmp/webp），返回识别到的条码文本列表
#[tauri::command]
fn decode_image(bytes: Vec<u8>) -> Result<Vec<String>, String> {
    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("图片解析失败: {e}"))?;
    let mut zimg = zedbar::Image::from_dynamic(&img)
        .map_err(|e| format!("图像转换失败: {e}"))?;
    let mut scanner = zedbar::Scanner::with_config(zedbar::DecoderConfig::all());
    let result = scanner.scan(&mut zimg);
    let symbols: Vec<String> = result
        .symbols()
        .iter()
        .filter_map(|s| s.data_string().map(|d| d.to_string()))
        .collect();
    if symbols.is_empty() {
        return Err("未识别到条码".into());
    }
    Ok(symbols)
}

/// 截取主显示器全屏，返回 PNG 字节 + 尺寸
#[tauri::command]
fn capture_screen() -> Result<(Vec<u8>, u32, u32), String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("获取显示器失败: {e}"))?;
    let monitor = monitors
        .first()
        .ok_or("未找到显示器")?;
    let img = monitor.capture_image().map_err(|e| format!("截图失败: {e}"))?;
    let (w, h) = (img.width(), img.height());
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| format!("PNG 编码失败: {e}"))?;
    Ok((png, w, h))
}

/// 返回主显示器物理像素尺寸 (width, height)
#[tauri::command]
fn screen_size() -> Result<(u32, u32), String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("获取显示器失败: {e}"))?;
    let monitor = monitors.first().ok_or("未找到显示器")?;
    let w = monitor.width().map_err(|e| format!("获取尺寸失败: {e}"))?;
    let h = monitor.height().map_err(|e| format!("获取尺寸失败: {e}"))?;
    Ok((w, h))
}

/// 退出整个应用进程
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // 单例：已有实例运行时，再次启动会把现有窗口移到屏幕中央并显示，提示用户
        .plugin(
            tauri_plugin_single_instance::init(|app, _args, _cwd| {
                if let Some(window) = app.get_webview_window("main") {
                    // 移到窗口所在显示器中央
                    if let Ok(Some(monitor)) = window.current_monitor() {
                        let size = monitor.size();
                        if let Ok(win_size) = window.outer_size() {
                            let x = (size.width.saturating_sub(win_size.width)) / 2;
                            let y = (size.height.saturating_sub(win_size.height)) / 2;
                            let _ = window.set_position(PhysicalPosition::new(x, y));
                        }
                    }
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }),
        )
        .invoke_handler(tauri::generate_handler![decode_image, capture_screen, screen_size, quit_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
