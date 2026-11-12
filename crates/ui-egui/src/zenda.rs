//! Zenda patches for Work. Events carry metadata only; Work owns accounts and Theo.
use serde_json::{Value, json};
pub const ON: bool = cfg!(feature = "zenda");

/// Events kept for Work when it does not drain them; older ones are dropped first.
const OUTBOX_MAX: usize = 32;

/// What the person asked of Work, oldest first, until Work drains it (`work.events`).
#[derive(Default, Debug)]
pub struct Outbox {
    events: Vec<Value>,
    seq: u64,
}

impl Outbox {
    /// Queue `event` (`work.theo`, `work.share`) with its params.
    pub fn push(&mut self, event: &str, params: Value) {
        self.seq = self.seq.saturating_add(1);
        if self.events.len() == OUTBOX_MAX {
            self.events.remove(0);
        }
        self.events.push(json!({"seq": self.seq, "event": event, "params": params}));
    }

    /// Every queued event, oldest first; the outbox is empty afterwards.
    pub fn drain(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.events)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Where help was requested, without workbook content.
pub fn theo_context(app: &crate::SheetApp) -> Value {
    json!({"from": "button", "tab": app.ui.ribbon_tab, "hasDocument": app.session.active().is_some()})
}

/// The saved workbook, including whether its disk copy is stale.
pub fn share_params(app: &crate::SheetApp) -> Value {
    let doc = app.session.active();
    json!({"path": doc.and_then(|d| d.path.as_deref()), "dirty": doc.is_some_and(|d| d.is_dirty()), "name": doc.map(|d| d.display_title())})
}

/// Work's mascot and orange brand button, shared by the curated app patches.
pub fn theo_button(ui: &mut egui::Ui, label: &str, hint: &str) -> egui::Response {
    let key = egui::Id::unique("zenda.theo.texture");
    let texture = ui.ctx().data_mut(|d| d.get_temp::<egui::TextureHandle>(key)).or_else(|| {
        let image = image::load_from_memory(include_bytes!("../../../assets/zenda/theo-icon-64.png")).ok()?.to_rgba8();
        let color = egui::ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], image.as_raw());
        let texture = ui.ctx().load_texture("Theo mascot", color, egui::TextureOptions::LINEAR);
        ui.ctx().data_mut(|d| d.insert_temp(key, texture.clone()));
        Some(texture)
    });
    let text = egui::RichText::new("Theo").color(egui::Color32::BLACK).size(11.5);
    let button = if let Some(texture) = texture {
        egui::Button::image_and_text(egui::Image::new((texture.id(), egui::vec2(18.0, 18.0))), text)
    } else {
        egui::Button::new(text)
    };
    let response = ui.add(button.fill(egui::Color32::from_rgb(234, 108, 30)).corner_radius(11.0).min_size(egui::vec2(74.0, 24.0)));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    response.on_hover_text(hint)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> crate::SheetApp {
        let mut session = gridcraft_engine::Session::new();
        session.new_workbook();
        crate::SheetApp::new(session, crate::Services::default())
    }
    #[test]
    fn events_drain_in_order_and_are_bounded() {
        let mut out = Outbox::default();
        for _ in 0..40 {
            out.push("work.theo", json!({"from":"button"}));
        }
        let got = out.drain();
        assert_eq!(got.len(), 32);
        assert_eq!(got[0]["seq"], 9);
        assert!(out.is_empty());
    }
    #[test]
    fn only_zenda_exposes_work_events() {
        let mut app = app();
        app.zenda.push("work.theo", theo_context(&app));
        let outcome = crate::control::handle(&mut app, &egui::Context::default(), "work.events", &json!({}));
        let crate::control::Outcome::Done(reply) = outcome else { panic!("immediate response") };
        assert_eq!(reply["ok"], ON);
        if ON {
            assert_eq!(reply["result"]["events"][0]["event"], "work.theo");
        }
    }
    #[test]
    fn share_preserves_unsaved_state_and_only_queues_in_zenda() {
        let mut app = app();
        app.run("cell.set", json!({"cell":"A1", "input":"42"})).unwrap();
        let result = app.run("file.share", json!({}));
        if ON {
            assert_eq!(result.unwrap()["work"], "share");
            let got = app.zenda.drain();
            assert_eq!(got[0]["params"]["path"], Value::Null);
            assert_eq!(got[0]["params"]["dirty"], true);
        } else {
            assert!(app.zenda.is_empty());
        }
    }
    #[test]
    fn theo_button_has_mascot_orange_and_accessible_names() {
        use egui_kittest::kittest::Queryable;
        for (language, label) in [
            (crate::i18n::Language::En, "Ask Theo for help"),
            (crate::i18n::Language::Es, "Pedile ayuda a Theo"),
            (crate::i18n::Language::PtBr, "Peça ajuda ao Theo"),
        ] {
            let mut h = egui_kittest::Harness::builder().build_ui_state(
                |ui, count: &mut usize| {
                    if theo_button(ui, language.tr("Ask Theo for help"), "Theo").clicked() {
                        *count += 1;
                    }
                },
                0usize,
            );
            h.get_by_label(label).click();
            h.run_steps(2);
            assert_eq!(*h.state(), 1);
        }
        let image = image::load_from_memory(include_bytes!("../../../assets/zenda/theo-icon-64.png")).unwrap();
        assert_eq!((image.width(), image.height()), (64, 64));
    }
    #[test]
    fn title_bar_theo_and_share_reach_work() {
        use egui_kittest::kittest::Queryable;
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut crate::SheetApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app(),
        );
        h.run_steps(14);
        if !ON {
            assert!(h.query_by_label("Ask Theo for help").is_none());
            return;
        }
        h.get_by_label("Ask Theo for help").click();
        h.run_steps(2);
        h.get_by_label("Share").click();
        h.run_steps(2);
        let events = h.state_mut().zenda.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["event"], "work.theo");
        assert_eq!(events[1]["event"], "work.share");
        for (language, theo, share, chart) in [
            (crate::i18n::Language::En, "Ask Theo for help", "Share", "Pie or Doughnut"),
            (crate::i18n::Language::Es, "Pedile ayuda a Theo", "Compartir", "Circular o anillo"),
            (crate::i18n::Language::PtBr, "Peça ajuda ao Theo", "Compartilhar", "Pizza ou Rosca"),
        ] {
            h.state_mut().ui.language = language;
            h.state_mut().ui.ribbon_tab = "Insert".into();
            h.run_steps(6);
            assert!(h.query_by_label(theo).is_some(), "{}: Theo", language.code());
            assert!(h.query_by_label(share).is_some(), "{}: Share", language.code());
            assert!(h.query_by_label(chart).is_some(), "{}: chart control", language.code());
            if let Some(dir) = std::env::var_os("ZENDA_TREE_DIR") {
                let path = std::path::PathBuf::from(dir).join(format!("gridcraft-{}.txt", language.code()));
                std::fs::write(path, format!("{:#?}", h.kittest_state().root())).unwrap();
            }
        }
    }

    #[test]
    fn theo_mascot_preview() {
        let Some(path) = std::env::var_os("ZENDA_THEO_PREVIEW") else { return };
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(160.0, 50.0)).with_pixels_per_point(3.0).wgpu().build_ui(|ui| {
            let _ = theo_button(ui, "Ask Theo for help", "Theo");
        });
        h.run_steps(2);
        h.render().unwrap().save(path).unwrap();
    }
}
