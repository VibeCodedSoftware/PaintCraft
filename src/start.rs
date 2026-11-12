// The start screen shown at launch: identity, real startup checks (run on a worker thread so the
// window appears at once), credits and licence statement, and "don't show this screen again".

use eframe::egui::{self, pos2, vec2, Color32, Rect, RichText, Sense, Stroke};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    Ok,
    Warn,
    Fail,
}

struct Check {
    name: &'static str,
    level: Level,
    detail: String,
}

const NAMES: [&str; 7] = [
    "Configuration",
    "Data folder",
    "Hardware",
    "Graphics",
    "Painting engine",
    "Asset library",
    "Licence notices",
];

const LOGO_PNG: &[u8] = include_bytes!("../assets/branding/paintcraft-256.png");
const NOTICE: &str = include_str!("../NOTICE.md");

/// What the engine has ready, counted on the UI thread where it lives.
pub struct EngineFacts {
    pub gpu: String,
    pub software_gpu: bool,
    pub shelf_assets: usize,
    pub generators: usize,
    pub hdris: usize,
    pub materials: usize,
}

/// The user's PaintCraft data folder (`%LOCALAPPDATA%\PaintCraft` on Windows).
fn data_root() -> Option<PathBuf> {
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        return Some(PathBuf::from(local).join("PaintCraft"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share/paintcraft"))
}

fn settings_path() -> Option<PathBuf> {
    data_root().map(|root| root.join("settings.json"))
}

/// Whether the start screen should open (the user has not turned it off).
pub fn wanted() -> bool {
    settings_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get("show_start_screen").and_then(serde_json::Value::as_bool))
        .unwrap_or(true)
}

fn remember(show: bool) {
    let Some(path) = settings_path() else {
        return;
    };
    let mut value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert("show_start_screen".to_owned(), serde_json::Value::Bool(show));
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&value) {
        let _ = std::fs::write(path, text);
    }
}

pub struct StartScreen {
    rx: Receiver<Check>,
    checks: Vec<Check>,
    dont_show: bool,
    show_licences: bool,
    logo: Option<egui::TextureHandle>,
}

impl StartScreen {
    pub fn new(facts: EngineFacts) -> Self {
        let (tx, rx) = channel();
        let _ = tx.send(Check {
            name: "Graphics",
            level: if facts.software_gpu { Level::Warn } else { Level::Ok },
            detail: if facts.software_gpu {
                format!("{} · painting will be slow without a GPU", facts.gpu)
            } else {
                format!("{} · wgpu viewport ready", facts.gpu)
            },
        });
        std::thread::spawn(move || {
            let send = |name, level, detail: String| {
                let _ = tx.send(Check { name, level, detail });
            };
            match settings_path() {
                Some(path) if path.exists() => {
                    let readable = std::fs::read_to_string(&path)
                        .ok()
                        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                        .is_some();
                    if readable {
                        send("Configuration", Level::Ok, format!("Settings loaded from {}", tidy(&path)));
                    } else {
                        send("Configuration", Level::Warn, "Settings file unreadable — using defaults".into());
                    }
                }
                _ => send("Configuration", Level::Ok, "Default settings (first run)".into()),
            }
            match data_root() {
                Some(root) => {
                    let probe = root.join(".write-test");
                    let written = std::fs::create_dir_all(&root)
                        .and_then(|()| std::fs::write(&probe, b"ok"))
                        .and_then(|()| std::fs::remove_file(&probe));
                    match written {
                        Ok(()) => send("Data folder", Level::Ok, format!("Writable · {}", tidy(&root))),
                        Err(error) => send("Data folder", Level::Fail, format!("Not writable ({error}): {}", tidy(&root))),
                    }
                }
                None => send("Data folder", Level::Warn, "No user data folder found".into()),
            }
            let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
            send(
                "Hardware",
                Level::Ok,
                format!("{threads} threads · {} · {}", std::env::consts::ARCH, std::env::consts::OS),
            );
            let started = Instant::now();
            let decoded = image::load_from_memory(LOGO_PNG).is_ok();
            send(
                "Painting engine",
                if decoded { Level::Ok } else { Level::Fail },
                format!(
                    "{} shelf assets · {} procedural generators · bakers and path tracer ready ({} ms)",
                    facts.shelf_assets,
                    facts.generators,
                    started.elapsed().as_millis()
                ),
            );
            let level = if facts.hdris > 0 { Level::Ok } else { Level::Warn };
            send(
                "Asset library",
                level,
                format!("{} HDRI environments · {} saved materials", facts.hdris, facts.materials),
            );
            let beside = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join("NOTICE.md")))
                .is_some_and(|path| path.exists());
            if beside {
                send("Licence notices", Level::Ok, "Credits and licence notice found next to the program".into());
            } else {
                send("Licence notices", Level::Warn, "NOTICE.md not next to the program · credits are built in".into());
            }
        });
        Self {
            rx,
            checks: Vec::new(),
            dont_show: false,
            show_licences: false,
            logo: None,
        }
    }

    fn done(&self) -> bool {
        self.checks.len() >= NAMES.len()
    }

    /// Draws the screen; `true` once the user moves on.
    pub fn ui(&mut self, ctx: &egui::Context, accent: Color32) -> bool {
        while let Ok(check) = self.rx.try_recv() {
            self.checks.push(check);
        }
        if !self.done() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        let logo = self
            .logo
            .get_or_insert_with(|| {
                let image = image::load_from_memory(LOGO_PNG).map_or_else(
                    |_| egui::ColorImage::new([1, 1], Color32::TRANSPARENT),
                    |img| {
                        let rgba = img.to_rgba8();
                        egui::ColorImage::from_rgba_unmultiplied(
                            [rgba.width() as usize, rgba.height() as usize],
                            rgba.as_raw(),
                        )
                    },
                );
                ctx.load_texture("start-logo", image, egui::TextureOptions::LINEAR)
            })
            .clone();

        let mut go = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(14, 15, 18)))
            .show(ctx, |ui| {
                let avail = ui.max_rect();
                let size = vec2(980.0_f32.min(avail.width() - 24.0), 600.0_f32.min(avail.height() - 24.0));
                let card = Rect::from_center_size(avail.center(), size);
                let painter = ui.painter().clone();
                painter.rect_filled(card.expand(1.0), 10.0, Color32::from_black_alpha(90));
                painter.rect_filled(card, 8.0, ui.visuals().panel_fill);
                let art = Rect::from_min_size(card.min, vec2((size.x * 0.38).round(), size.y));
                mosaic(&painter.with_clip_rect(art), art);
                let content = Rect::from_min_max(pos2(art.right() + 34.0, card.top() + 30.0), card.max - vec2(30.0, 24.0));
                ui.allocate_ui_at_rect(content, |cu| {
                    cu.horizontal(|ui| {
                        ui.add(egui::Image::new(&logo).fit_to_exact_size(vec2(68.0, 68.0)).rounding(14.0));
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            ui.label(RichText::new("PaintCraft").size(30.0).strong());
                            let kind = if cfg!(debug_assertions) { "development build" } else { "release build" };
                            ui.label(
                                RichText::new(format!(
                                    "Version {} · {kind} · 3D texture painting",
                                    env!("CARGO_PKG_VERSION")
                                ))
                                .weak(),
                            );
                        });
                    });
                    cu.add_space(18.0);
                    egui::Grid::new("start_checks")
                        .num_columns(3)
                        .spacing([10.0, 9.0])
                        .show(cu, |ui| {
                            for name in NAMES {
                                let check = self.checks.iter().find(|check| check.name == name);
                                match check {
                                    Some(check) => status_icon(ui, check.level),
                                    None => {
                                        ui.add(egui::Spinner::new().size(15.0));
                                    }
                                }
                                ui.label(RichText::new(name).size(14.5));
                                let detail = check.map_or_else(|| "Checking…".to_owned(), |check| check.detail.clone());
                                let max = (content.right() - ui.cursor().left()).max(80.0);
                                ui.allocate_ui_with_layout(
                                    vec2(max, 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.set_max_width(max);
                                        ui.add(egui::Label::new(RichText::new(detail).weak()).truncate());
                                    },
                                );
                                ui.end_row();
                            }
                        });
                    cu.add_space(16.0);
                    cu.label(
                        RichText::new(
                            "PaintCraft is built on the 3D Paintcraft engine by Ras-Maciej \
                             (github.com/Ras-Maciej/3DPaintCraft), with a redesigned studio workspace. It is \
                             released under MIT OR Apache-2.0 and uses third-party open-source libraries that \
                             remain subject to their own licences; see View licences and NOTICE.md.",
                        )
                        .small()
                        .weak(),
                    );
                    cu.add_space(6.0);
                    cu.label(
                        RichText::new("Everything runs locally on your GPU: nothing you paint is uploaded.")
                            .small()
                            .weak(),
                    );
                });

                let bottom = Rect::from_min_max(
                    pos2(content.left(), card.bottom() - 92.0),
                    pos2(content.right(), card.bottom() - 18.0),
                );
                ui.allocate_ui_at_rect(bottom, |bu| {
                    bu.horizontal(|ui| {
                        let failed = self.checks.iter().any(|check| check.level == Level::Fail);
                        let label = if failed { "Continue anyway  →" } else { "Continue to PaintCraft  →" };
                        let start = egui::Button::new(RichText::new(label).size(14.5).color(Color32::WHITE))
                            .fill(accent)
                            .min_size(vec2(270.0, 36.0));
                        if ui.add(start).clicked() {
                            go = true;
                        }
                        let licences = egui::Button::new(RichText::new("View licences").size(14.5)).min_size(vec2(0.0, 36.0));
                        if ui.add(licences).clicked() {
                            self.show_licences = true;
                        }
                    });
                    bu.add_space(8.0);
                    bu.checkbox(&mut self.dont_show, "Don't show this screen again");
                });
            });

        if self.show_licences {
            let mut open = true;
            egui::Window::new("Credits and licences")
                .open(&mut open)
                .default_size(vec2(620.0, 460.0))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(RichText::new(NOTICE).size(13.0));
                    });
                });
            self.show_licences = open;
        } else if ctx.input(|input| input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Escape)) {
            go = true;
        }
        if go {
            remember(!self.dont_show);
        }
        go
    }
}

/// A round status mark: a check, an exclamation or a cross.
fn status_icon(ui: &mut egui::Ui, level: Level) {
    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    let color = match level {
        Level::Ok => Color32::from_rgb(76, 194, 128),
        Level::Warn => Color32::from_rgb(230, 170, 60),
        Level::Fail => Color32::from_rgb(226, 82, 82),
    };
    let painter = ui.painter();
    let c = rect.center();
    painter.circle_stroke(c, 7.5, Stroke::new(1.6, color));
    let stroke = Stroke::new(1.8, color);
    match level {
        Level::Ok => {
            painter.line_segment([c + vec2(-3.5, 0.2), c + vec2(-1.0, 2.8)], stroke);
            painter.line_segment([c + vec2(-1.0, 2.8), c + vec2(3.8, -2.6)], stroke);
        }
        Level::Warn => {
            painter.line_segment([c + vec2(0.0, -3.8), c + vec2(0.0, 1.2)], stroke);
            painter.circle_filled(c + vec2(0.0, 3.6), 1.0, color);
        }
        Level::Fail => {
            painter.line_segment([c + vec2(-3.0, -3.0), c + vec2(3.0, 3.0)], stroke);
            painter.line_segment([c + vec2(3.0, -3.0), c + vec2(-3.0, 3.0)], stroke);
        }
    }
}

/// Shows the home folder as `~`, so screenshots don't reveal the account name.
fn tidy(path: &std::path::Path) -> String {
    let shown = path.display().to_string();
    match std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        Ok(home) if !home.is_empty() && shown.to_lowercase().starts_with(&home.to_lowercase()) => shown
            .get(home.len()..)
            .map_or_else(|| shown.clone(), |rest| format!("~{rest}")),
        _ => shown,
    }
}

/// The decorative mosaic in PaintCraft's cheetah blues: deterministic tiles with the odd paint
/// drop and spot, fading toward the content side.
fn mosaic(painter: &egui::Painter, rect: Rect) {
    let cell = 36.0;
    let cols = (rect.width() / cell).ceil() as i32;
    let rows = (rect.height() / cell).ceil() as i32;
    let palette: [[u8; 3]; 6] = [
        [22, 70, 160],
        [34, 104, 214],
        [56, 152, 255],
        [20, 44, 96],
        [40, 60, 120],
        [70, 90, 140],
    ];
    painter.rect_filled(rect, 0.0, Color32::from_rgb(12, 16, 26));
    for y in 0..rows {
        for x in 0..cols {
            let mut h = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
            h ^= h >> 13;
            h = h.wrapping_mul(0x5bd1_e995);
            h ^= h >> 15;
            let base = palette[(h % 6) as usize];
            let fade = 1.0 - (x as f32 / cols as f32) * 0.45;
            let k = (0.45 + ((h >> 8) % 100) as f32 / 260.0) * fade;
            let color = Color32::from_rgb(
                (f32::from(base[0]) * k) as u8,
                (f32::from(base[1]) * k) as u8,
                (f32::from(base[2]) * k) as u8,
            );
            let tile = Rect::from_min_size(
                rect.min + vec2(x as f32 * cell, y as f32 * cell),
                vec2(cell - 2.0, cell - 2.0),
            );
            painter.rect_filled(tile, 2.0, color);
            if (h >> 20) % 9 == 0 {
                // A cheetah spot or a paint drop.
                let spot = Color32::from_rgba_unmultiplied(8, 10, 18, 120);
                if (h >> 3) & 1 == 0 {
                    painter.circle_filled(tile.center(), 5.0 + ((h >> 5) % 4) as f32, spot);
                } else {
                    painter.circle_filled(tile.center() + vec2(0.0, 2.0), 4.0, Color32::from_white_alpha(40));
                    painter.circle_filled(tile.center() + vec2(0.0, -3.0), 2.0, Color32::from_white_alpha(40));
                }
            }
        }
    }
}
