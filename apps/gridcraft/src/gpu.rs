//! A lost graphics device (the driver reset, `DXGI_ERROR_DEVICE_REMOVED`, a GPU switch) must not
//! take the workbooks with it. wgpu's default error handler panics, so the app used to close and
//! lose every unsaved change. Here errors are logged instead, and when the device is lost every
//! workbook with changes is written as a recovered copy and the app exits with [`EXIT_GPU_LOST`],
//! which Zenda Work tells apart from a close (0) or a panic (101) and answers by reopening it.
//!
//! eframe can't recreate a device under a running window, so exiting cleanly is the recovery.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub const EXIT_GPU_LOST: i32 = 75;

static LOST: AtomicBool = AtomicBool::new(false);

pub fn lost() -> bool {
    LOST.load(Ordering::SeqCst)
}

/// What a device-lost callback does: only a loss the app did not ask for counts.
fn on_lost(reason: eframe::wgpu::DeviceLostReason, message: &str, wake: &dyn Fn()) {
    if reason == eframe::wgpu::DeviceLostReason::Destroyed {
        return;
    }
    log::error!("graphics: device lost ({reason:?}): {message}");
    LOST.store(true, Ordering::SeqCst);
    wake();
}

/// Installs the handlers on the device eframe created.
pub fn install(cc: &eframe::CreationContext<'_>) {
    let Some(rs) = &cc.wgpu_render_state else { return };
    rs.device.on_uncaptured_error(Arc::new(|e| log::error!("graphics: {e}")));
    let ctx = cc.egui_ctx.clone();
    rs.device.set_device_lost_callback(move |reason, message| on_lost(reason, &message, &|| ctx.request_repaint()));
}

/// Where a recovered copy of a workbook goes: beside its file when it has one, else in
/// `fallback` (the Documents folder), never over the file itself.
pub fn recovered_path(path: Option<&str>, title: &str, stamp: &str, fallback: &Path) -> PathBuf {
    let clean = |s: &str| s.chars().map(|c| if r#"\/:*?"<>|"#.contains(c) || c.is_control() { '_' } else { c }).collect::<String>();
    match path.map(Path::new) {
        Some(p) => {
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| clean(title));
            p.with_file_name(format!("{stem} (recuperado {stamp}).xlsx"))
        }
        None => fallback.join(format!("{} (recuperado {stamp}).xlsx", clean(title))),
    }
}

/// The Documents folder, or the settings folder when there is none.
fn documents() -> Option<PathBuf> {
    let docs = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| PathBuf::from(h).join("Documents"));
    docs.filter(|d| d.is_dir()).or_else(crate::config_dir)
}

/// Writes every workbook with changes as a recovered `.xlsx`. Returns the files written.
pub fn save_recovered(session: &gridcraft_engine::Session) -> Vec<PathBuf> {
    let stamp = chrono::Local::now().format("%Y-%m-%d %H.%M").to_string();
    let Some(fallback) = documents() else { return Vec::new() };
    let mut written = Vec::new();
    for doc in session.documents().iter().filter(|d| d.is_dirty()) {
        let to = recovered_path(doc.path.as_deref(), &doc.title, &stamp, &fallback);
        match gridcraft_xlsx::write_xlsx(&doc.wb).map_err(|e| e.to_string()).and_then(|bytes| std::fs::write(&to, bytes).map_err(|e| e.to_string())) {
            Ok(()) => {
                log::info!("graphics: saved a recovered copy at {}", to.display());
                written.push(to);
            }
            Err(e) => log::error!("graphics: could not save a recovered copy of {}: {e}", doc.title),
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_unasked_loss_makes_the_app_leave() {
        let woke = std::cell::Cell::new(0);
        on_lost(eframe::wgpu::DeviceLostReason::Destroyed, "dropped at exit", &|| woke.set(woke.get() + 1));
        assert!(!lost());
        on_lost(eframe::wgpu::DeviceLostReason::Unknown, "DXGI_ERROR_DEVICE_REMOVED", &|| woke.set(woke.get() + 1));
        assert!(lost());
        assert_eq!(woke.get(), 1);
    }

    #[test]
    fn a_recovered_copy_never_overwrites_the_workbook() {
        let docs = Path::new(r"C:\Users\Ana\Documents");
        assert_eq!(
            recovered_path(Some(r"C:\Users\Ana\Desktop\Presupuesto.xlsx"), "Presupuesto", "2026-11-20 18.05", docs),
            Path::new(r"C:\Users\Ana\Desktop\Presupuesto (recuperado 2026-11-20 18.05).xlsx")
        );
        assert_eq!(recovered_path(None, "Book1", "2026-11-20 18.05", docs), docs.join("Book1 (recuperado 2026-11-20 18.05).xlsx"));
        assert_eq!(recovered_path(None, "a/b:c", "x", docs), docs.join("a_b_c (recuperado x).xlsx"));
    }

    #[test]
    fn dirty_workbooks_are_written_as_xlsx() {
        let dir = std::env::temp_dir().join(format!("gridcraft-gpu-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = gridcraft_engine::Session::default();
        s.execute("file.new", serde_json::json!({})).unwrap();
        s.execute("cell.set", serde_json::json!({"cell": "A1", "input": "42"})).unwrap();
        let doc = s.documents().iter().find(|d| d.is_dirty()).unwrap();
        let to = recovered_path(None, &doc.title, "t", &dir);
        std::fs::write(&to, gridcraft_xlsx::write_xlsx(&doc.wb).unwrap()).unwrap();
        assert!(std::fs::metadata(&to).unwrap().len() > 100);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
