use eframe::wasm_bindgen::{self, prelude::*};
use eframe::{egui, web_sys::HtmlCanvasElement};
use projectcraft_engine::{Session, interchange::Format, model::Project, time::Date};
use projectcraft_ui_egui::App;

struct Shell {
    app: App,
    ctx: egui::Context,
}

impl eframe::App for Shell {
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.ui(ui);
    }
}

fn error(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}

#[wasm_bindgen(wasm_bindgen = eframe::wasm_bindgen)]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

impl Default for WebHandle {
    fn default() -> Self {
        Self { runner: eframe::WebRunner::new() }
    }
}

#[wasm_bindgen(wasm_bindgen = eframe::wasm_bindgen)]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self, canvas: HtmlCanvasElement, epoch_days: i32) -> Result<(), JsValue> {
        let date = Date::EPOCH.add_days(i64::from(epoch_days)).ok_or_else(|| error("Invalid project start date"))?;
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    Ok(Box::new(Shell { app: App::new(Session::new(Project::new("New project", date)), None), ctx: cc.egui_ctx.clone() }))
                }),
            )
            .await
    }

    pub fn take_request(&self) -> Option<String> {
        self.runner.app_mut::<Shell>()?.app.take_web_request().map(str::to_owned)
    }

    pub fn dirty(&self) -> Result<bool, JsValue> {
        self.with_app(|shell| Ok(shell.app.web_dirty()))
    }

    pub fn suggested_name(&self) -> Result<String, JsValue> {
        self.with_app(|shell| Ok(shell.app.save_as_name()))
    }

    pub fn open(&self, name: &str, bytes: &[u8]) -> Result<(), JsValue> {
        self.with_app(|shell| shell.app.open_web(name, bytes).map_err(error))
    }

    pub fn snapshot(&self) -> Result<String, JsValue> {
        self.with_app(|shell| shell.app.save_web().map_err(error))
    }

    pub fn saved(&self, name: &str, snapshot: &str) -> Result<(), JsValue> {
        self.with_app(|shell| shell.app.saved_web(name, snapshot).map_err(error))
    }

    pub fn export(&self, format: &str) -> Result<String, JsValue> {
        let format = Format::from_code(format).ok_or_else(|| error("Choose XML or CSV"))?;
        self.with_app(|shell| shell.app.export_web(format).map_err(error))
    }

    pub fn destroy(&self) {
        self.runner.destroy();
    }
}

impl WebHandle {
    fn with_app<T>(&self, action: impl FnOnce(&mut Shell) -> Result<T, JsValue>) -> Result<T, JsValue> {
        let mut shell = self.runner.app_mut::<Shell>().ok_or_else(|| error("ProjectCraft is unavailable"))?;
        let result = action(&mut shell);
        shell.ctx.request_repaint();
        result
    }
}
