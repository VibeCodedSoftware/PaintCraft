// PaintCraft studio shell: theme, viewport bar, docks (Properties, Layers, Shelf) and status bar.
// The painting engine underneath is 3D Paintcraft by Ras-Maciej; this module is the new workspace around it.

use crate::app::{PaintcraftApp, RightPanelTab, ShelfPickerState, ShelfPickerTarget, ViewLayout};
use crate::brush::{AlphaType, PaintTarget, PolygonFillMode, ToolMode};
use crate::renderer::ViewportChannelMode;
use eframe::egui::{self, Color32, RichText, Rounding, Stroke};

/// Cheetah blue, the PaintCraft accent.
pub const ACCENT: Color32 = Color32::from_rgb(56, 152, 255);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(32, 74, 128);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(138, 144, 156);
pub const TEXT_HEADING: Color32 = Color32::from_rgb(186, 192, 204);

const PANEL: Color32 = Color32::from_rgb(28, 30, 35);
const WINDOW: Color32 = Color32::from_rgb(33, 35, 41);
const SUNKEN: Color32 = Color32::from_rgb(18, 19, 23);
const FAINT: Color32 = Color32::from_rgb(23, 25, 29);
const BORDER: Color32 = Color32::from_rgb(46, 50, 58);

const ICON_PNG: &[u8] = include_bytes!("../assets/branding/paintcraft-256.png");

pub fn window_icon() -> egui::IconData {
    match image::load_from_memory(ICON_PNG) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            egui::IconData { rgba: rgba.into_raw(), width, height }
        }
        Err(_) => egui::IconData::default(),
    }
}

pub fn apply_studio_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.window_fill = WINDOW;
    v.panel_fill = PANEL;
    v.faint_bg_color = FAINT;
    v.extreme_bg_color = SUNKEN;
    v.code_bg_color = SUNKEN;
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_rounding = Rounding::same(6.0_f32);
    v.menu_rounding = Rounding::same(5.0_f32);
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(56, 152, 255, 90);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);

    let round = Rounding::same(4.0_f32);
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(40, 43, 50));
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(178, 184, 196));
    v.widgets.noninteractive.rounding = round;
    v.widgets.inactive.bg_fill = Color32::from_rgb(41, 44, 51);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(41, 44, 51);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(208, 213, 224));
    v.widgets.inactive.rounding = round;
    v.widgets.hovered.bg_fill = Color32::from_rgb(52, 57, 67);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(52, 57, 67);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(78, 86, 100));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.hovered.rounding = round;
    v.widgets.active.bg_fill = ACCENT_DIM;
    v.widgets.active.weak_bg_fill = ACCENT_DIM;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.active.rounding = round;
    v.widgets.open.rounding = round;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 5.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style.spacing.window_margin = egui::Margin::same(10.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    style.spacing.interact_size.y = 22.0;
    style.spacing.slider_width = 150.0;
    ctx.set_style(style);
}

/// Small caps heading used at the top of every dock section.
pub fn section_title(text: &str) -> RichText {
    RichText::new(text).size(11.0).strong().color(TEXT_HEADING)
}

/// A flat tab that lights up in the accent colour when selected.
pub fn tab_button(selected: bool, label: &str) -> egui::Button<'static> {
    let text = if selected {
        RichText::new(label.to_owned()).color(Color32::WHITE)
    } else {
        RichText::new(label.to_owned()).color(TEXT_MUTED)
    };
    let button = egui::Button::new(text).rounding(Rounding::same(4.0_f32));
    if selected {
        button.fill(ACCENT_DIM).stroke(Stroke::new(1.0_f32, ACCENT))
    } else {
        button.frame(false)
    }
}

fn toggle(ui: &mut egui::Ui, on: &mut bool, label: &str, tip: &str) -> bool {
    let clicked = ui.add(tab_button(*on, label)).on_hover_text(tip).clicked();
    if clicked {
        *on = !*on;
    }
    clicked
}

fn group_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(FAINT)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 41, 48)))
        .rounding(Rounding::same(5.0_f32))
        .inner_margin(egui::Margin::same(8.0))
}

/// A titled card inside a dock.
fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    group_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(section_title(title));
        ui.add_space(2.0);
        add(ui);
    });
    ui.add_space(6.0);
}

fn slider_row(ui: &mut egui::Ui, label: &str, slider: egui::Slider<'_>) {
    ui.label(RichText::new(label).color(TEXT_MUTED));
    ui.add(slider);
    ui.end_row();
}

fn tool_name(mode: ToolMode) -> &'static str {
    match mode {
        ToolMode::Brush => "Paint",
        ToolMode::Eraser => "Eraser",
        ToolMode::Projection => "Projection",
        ToolMode::FillBucket => "Polygon fill",
        ToolMode::Smudge => "Smudge",
        ToolMode::Clone => "Clone",
        ToolMode::Eyedropper => "Material picker",
    }
}

#[derive(Clone, Copy)]
enum StencilPreset {
    Grunge,
    Scratches,
    Splatter,
    Checker,
}

impl StencilPreset {
    const ALL: [StencilPreset; 4] = [Self::Grunge, Self::Scratches, Self::Splatter, Self::Checker];

    fn name(self) -> &'static str {
        match self {
            Self::Grunge => "Grunge",
            Self::Scratches => "Scratches",
            Self::Splatter => "Spots & splatter",
            Self::Checker => "Checkerboard",
        }
    }

    fn build(self) -> crate::channel::RgbaBuffer {
        const N: u32 = 512;
        let mut buf = crate::channel::RgbaBuffer::new(N, N, glam::Vec4::ONE);
        for y in 0..N {
            for x in 0..N {
                let u = x as f32 / N as f32;
                let v = y as f32 / N as f32;
                let val = match self {
                    Self::Grunge => crate::utils::NoiseGenerator::grunge_map_2d(u * 6.0, v * 6.0),
                    Self::Scratches => crate::utils::NoiseGenerator::directional_noise_2d(u * 8.0, v * 8.0, 0.4),
                    Self::Splatter => crate::utils::NoiseGenerator::bnw_spots_1_2d(u * 5.0, v * 5.0),
                    Self::Checker => {
                        if ((x / 32) % 2) ^ ((y / 32) % 2) == 0 { 1.0 } else { 0.0 }
                    }
                };
                buf.set_pixel(x, y, glam::Vec4::new(val, val, val, 1.0));
            }
        }
        buf
    }
}

impl PaintcraftApp {
    /// One slim bar above the viewport: layout, channel, display toggles and the main actions.
    pub(crate) fn context_bar_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("context_bar")
            .exact_height(32.0)
            .frame(egui::Frame::none().fill(FAINT).inner_margin(egui::Margin::symmetric(8.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (layout, label, tip) in [
                        (ViewLayout::View3DOnly, "3D", "3D viewport only [F2]"),
                        (ViewLayout::SplitDualView, "3D + UV", "3D viewport beside the UV canvas [F1]"),
                        (ViewLayout::View2DOnly, "UV", "UV canvas only [F3]"),
                    ] {
                        if ui.add(tab_button(self.layout == layout, label)).on_hover_text(tip).clicked() {
                            self.layout = layout;
                        }
                    }
                    ui.separator();

                    ui.label(RichText::new("Channel").color(TEXT_MUTED));
                    egui::ComboBox::from_id_source("viewport_mode_select")
                        .selected_text(self.view_mode.name())
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for (mode, label) in [
                                (ViewportChannelMode::Material, "Material (lit PBR)  M"),
                                (ViewportChannelMode::BaseColor, "Base color"),
                                (ViewportChannelMode::Roughness, "Roughness"),
                                (ViewportChannelMode::Metallic, "Metallic"),
                                (ViewportChannelMode::Normal, "Normal"),
                                (ViewportChannelMode::Height, "Height"),
                                (ViewportChannelMode::Emissive, "Emissive"),
                                (ViewportChannelMode::Opacity, "Opacity"),
                                (ViewportChannelMode::Clearcoat, "Clearcoat"),
                                (ViewportChannelMode::Anisotropy, "Anisotropy"),
                                (ViewportChannelMode::AmbientOcclusion, "Mesh map: ambient occlusion"),
                                (ViewportChannelMode::Curvature, "Mesh map: curvature"),
                                (ViewportChannelMode::Position, "Mesh map: position"),
                                (ViewportChannelMode::WorldNormal, "Mesh map: world normal"),
                                (ViewportChannelMode::Thickness, "Mesh map: thickness"),
                            ] {
                                ui.selectable_value(&mut self.view_mode, mode, label);
                            }
                        });
                    ui.separator();

                    let mut wire = self.renderer.show_wireframe;
                    if toggle(ui, &mut wire, "Wireframe", "Show mesh wireframe [F4]") {
                        self.renderer.show_wireframe = wire;
                        self.show_uv_wireframe = wire;
                    }
                    toggle(ui, &mut self.shader_settings.shadows_enabled, "Shadows", "Real-time sun shadows");
                    toggle(ui, &mut self.camera.is_orthographic, "Ortho", "Orthographic camera");
                    if ui.add(egui::Button::new("Frame").frame(false)).on_hover_text("Reset the camera to frame the mesh [F]").clicked() {
                        self.camera.reset();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let export = egui::Button::new(RichText::new("Export").color(Color32::WHITE).strong())
                            .fill(ACCENT_DIM)
                            .stroke(Stroke::new(1.0_f32, ACCENT));
                        if ui.add(export).on_hover_text("Export texture maps [Ctrl+E]").clicked() {
                            self.show_export_modal = true;
                        }
                        if ui.button("Bake").on_hover_text("Bake mesh maps [Ctrl+Shift+B]").clicked() {
                            self.show_bake_modal = true;
                        }
                        ui.menu_button("Display", |ui| {
                            if ui.button("Viewport, camera and environment...").clicked() {
                                self.show_viewport_settings_modal = true;
                                ui.close_menu();
                            }
                            if ui.button("Shading and post-processing...").clicked() {
                                self.show_shader_settings_modal = true;
                                ui.close_menu();
                            }
                            if ui.button("Environment from shelf...").clicked() {
                                self.shelf_picker = Some(ShelfPickerState::new(ShelfPickerTarget::HdriEnvironment));
                                ui.close_menu();
                            }
                        });
                        ui.separator();

                        let mut iray = self.iray_mode;
                        if toggle(ui, &mut iray, "Path trace", "Progressive path-traced preview [F10]") {
                            self.iray_mode = iray;
                            if iray {
                                if let Some(pt) = &mut self.path_tracer {
                                    pt.reset();
                                }
                                self.status_message = "Path tracer on (F10 or Esc to leave)".to_string();
                            } else {
                                self.status_message = "Path tracer off".to_string();
                            }
                        }
                        if self.iray_mode {
                            if ui.add(egui::Button::new("Restart").frame(false)).on_hover_text("Restart accumulation").clicked() {
                                if let Some(pt) = &mut self.path_tracer {
                                    pt.reset();
                                }
                            }
                            if ui.add(egui::Button::new("Save render").frame(false)).clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("PNG image", &["png"])
                                    .set_file_name("paintcraft_render.png")
                                    .save_file()
                                {
                                    if let Some(pt) = &self.path_tracer {
                                        self.status_message = match pt.save_to_png(&path) {
                                            Ok(()) => format!("Saved render to {}", path.display()),
                                            Err(err) => format!("Could not save render: {}", err),
                                        };
                                    }
                                }
                            }
                            let mut denoise = self.iray_denoise;
                            if toggle(ui, &mut denoise, "Denoise", "Normal-guided denoiser") {
                                self.iray_denoise = denoise;
                                if let Some(pt) = &mut self.path_tracer {
                                    pt.denoiser_enabled = denoise;
                                    pt.update_color_buffer();
                                }
                            }
                        }
                    });
                });
            });
    }

    pub(crate) fn status_bar_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .frame(egui::Frame::none().fill(SUNKEN).inner_margin(egui::Margin::symmetric(10.0, 3.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new(tool_name(self.brush_engine.settings.mode)).color(ACCENT).size(11.5));
                    ui.label(RichText::new(&self.status_message).color(TEXT_MUTED).size(11.5));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let small = |t: String| RichText::new(t).color(TEXT_MUTED).size(11.0);
                        ui.label(small(format!("{:.0} fps", self.perf_metrics.fps)));
                        ui.separator();
                        ui.label(small(format!("{} × {}", self.project.texture_width, self.project.texture_height)));
                        ui.separator();
                        ui.label(small(format!("{} tris", self.project.mesh.indices.len() / 3)));
                        ui.separator();
                        ui.label(small("Tab hides panels".to_string()));
                    });
                });
            });
    }

    /// Far-right dock: texture sets on top, the layer stack below.
    pub(crate) fn layers_dock(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("layers_dock")
            .resizable(true)
            .default_width(340.0)
            .width_range(290.0..=560.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(section_title("TEXTURE SETS"));
                self.texture_sets_ui(ui);
                ui.add_space(6.0);
                ui.label(section_title("LAYERS"));
                self.layer_stack_ui(ui);
            });
    }

    /// Properties dock between the viewport and the layers: what the active tool, layer or mesh maps can do.
    pub(crate) fn properties_dock(&mut self, ctx: &egui::Context) {
        // Follow the user's focus: picking a tool shows its settings, picking a layer shows the layer's.
        let mode = self.brush_engine.settings.mode;
        let layer = self.project.active_layer_idx;
        let key = egui::Id::new("studio_properties_follow");
        let prev: Option<(ToolMode, usize)> = ctx.data(|d| d.get_temp(key));
        if let Some((prev_mode, prev_layer)) = prev {
            if prev_mode != mode {
                self.right_panel_tab = RightPanelTab::Tool;
            } else if prev_layer != layer {
                self.right_panel_tab = RightPanelTab::Layers;
            }
        }
        ctx.data_mut(|d| d.insert_temp(key, (mode, layer)));

        egui::SidePanel::right("properties_dock")
            .resizable(true)
            .default_width(320.0)
            .width_range(270.0..=520.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(section_title("PROPERTIES"));
                    ui.add_space(8.0);
                    for (tab, label) in [
                        (RightPanelTab::Tool, "Tool"),
                        (RightPanelTab::Layers, "Layer"),
                        (RightPanelTab::MeshMaps, "Mesh maps"),
                    ] {
                        if ui.add(tab_button(self.right_panel_tab == tab, label)).clicked() {
                            self.right_panel_tab = tab;
                        }
                    }
                });
                ui.separator();
                match self.right_panel_tab {
                    RightPanelTab::Tool => {
                        egui::ScrollArea::vertical()
                            .id_source("tool_properties_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.tool_properties_ui(ui));
                    }
                    RightPanelTab::Layers => self.layer_properties_ui(ui),
                    RightPanelTab::MeshMaps => self.mesh_maps_ui(ui),
                }
            });
    }

    /// Bottom dock under the viewport: materials, smart materials, masks, generators, alphas, environments.
    pub(crate) fn shelf_dock(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("shelf_dock")
            .resizable(true)
            .default_height(236.0)
            .height_range(150.0..=520.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.render_assets_panel(ui);
            });
    }

    fn tool_properties_ui(&mut self, ui: &mut egui::Ui) {
        let mode = self.brush_engine.settings.mode;
        ui.add_space(4.0);
        ui.label(RichText::new(tool_name(mode)).size(16.0).strong().color(Color32::WHITE));
        ui.add_space(4.0);

        match mode {
            ToolMode::Brush | ToolMode::Eraser | ToolMode::Projection | ToolMode::Smudge | ToolMode::Clone => {
                self.stroke_card(ui);
                match mode {
                    ToolMode::Smudge => card(ui, "SMUDGE", |ui| {
                        egui::Grid::new("smudge_grid").num_columns(2).show(ui, |ui| {
                            slider_row(ui, "Strength", egui::Slider::new(&mut self.brush_engine.settings.smudge_strength, 0.05..=1.0));
                        });
                    }),
                    ToolMode::Clone => card(ui, "CLONE SOURCE", |ui| match self.brush_engine.settings.clone_source_uv {
                        Some(src) => {
                            ui.horizontal(|ui| {
                                ui.label(format!("UV {:.2}, {:.2}", src.x, src.y));
                                if ui.button("Clear").clicked() {
                                    self.brush_engine.settings.clone_source_uv = None;
                                }
                            });
                        }
                        None => {
                            ui.label(RichText::new("Hold V or Alt and click the surface to set the source.").color(TEXT_MUTED));
                        }
                    }),
                    ToolMode::Projection => self.stencil_card(ui),
                    _ => {}
                }
                self.target_card(ui);
                self.symmetry_card(ui);
                self.dynamics_card(ui);
            }
            ToolMode::FillBucket => self.polygon_fill_card(ui),
            ToolMode::Eyedropper => card(ui, "PICKER", |ui| {
                ui.label(RichText::new("Click the model to pick its material into the brush.").color(TEXT_MUTED));
                let c = self.brush_engine.settings.paint_color;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::hover());
                let to8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u8;
                ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(to8(c.x), to8(c.y), to8(c.z)));
            }),
        }
    }

    fn stroke_card(&mut self, ui: &mut egui::Ui) {
        card(ui, "STROKE", |ui| {
            let s = &mut self.brush_engine.settings;
            egui::Grid::new("stroke_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                slider_row(ui, "Size", egui::Slider::new(&mut s.radius, 1.0..=180.0).logarithmic(true).suffix(" px"));
                slider_row(ui, "Flow", egui::Slider::new(&mut s.flow, 0.01..=1.0));
                slider_row(ui, "Opacity", egui::Slider::new(&mut s.opacity, 0.01..=1.0));
                slider_row(ui, "Hardness", egui::Slider::new(&mut s.hardness, 0.0..=1.0));
                slider_row(ui, "Spacing", egui::Slider::new(&mut s.spacing, 0.02..=0.5));
                ui.label(RichText::new("Alpha").color(TEXT_MUTED));
                egui::ComboBox::from_id_source("tool_alpha_select")
                    .selected_text(s.alpha.name())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for a in [
                            AlphaType::SoftRound,
                            AlphaType::HardRound,
                            AlphaType::NoiseSplatter,
                            AlphaType::Chisel,
                            AlphaType::Stipple,
                            AlphaType::Scratches,
                        ] {
                            ui.selectable_value(&mut s.alpha, a, a.name());
                        }
                    });
                ui.end_row();
            });
            if ui.button("Choose alpha from shelf...").clicked() {
                self.shelf_picker = Some(ShelfPickerState::new(ShelfPickerTarget::BrushAlpha));
            }
            ui.label(RichText::new("[ and ] change the size while painting.").small().color(TEXT_MUTED));
        });
    }

    fn target_card(&mut self, ui: &mut egui::Ui) {
        let idx = self.project.active_layer_idx;
        let has_mask = self.project.layers.get(idx).map(|l| l.has_mask).unwrap_or(false);
        if !has_mask {
            return;
        }
        card(ui, "PAINT ONTO", |ui| {
            ui.horizontal(|ui| {
                let s = &mut self.brush_engine.settings;
                if ui.add(tab_button(s.target == PaintTarget::Material, "Layer material")).clicked() {
                    s.target = PaintTarget::Material;
                }
                if ui.add(tab_button(s.target == PaintTarget::Mask, "Layer mask")).clicked() {
                    s.target = PaintTarget::Mask;
                }
            });
            if self.brush_engine.settings.target == PaintTarget::Mask {
                ui.label(RichText::new(format!(
                    "Painting {} into the mask. X swaps.",
                    if self.brush_engine.settings.mask_paint_value > 0.5 { "white (reveal)" } else { "black (hide)" }
                ))
                .color(TEXT_MUTED));
            }
        });
    }

    fn symmetry_card(&mut self, ui: &mut egui::Ui) {
        card(ui, "SYMMETRY", |ui| {
            ui.horizontal(|ui| {
                let s = &mut self.brush_engine.settings;
                let any = s.symmetry_x || s.symmetry_y || s.symmetry_z;
                if ui.add(tab_button(!any, "Off")).clicked() {
                    s.symmetry_x = false;
                    s.symmetry_y = false;
                    s.symmetry_z = false;
                }
                toggle(ui, &mut s.symmetry_x, "X", "Mirror left / right");
                toggle(ui, &mut s.symmetry_y, "Y", "Mirror top / bottom");
                toggle(ui, &mut s.symmetry_z, "Z", "Mirror front / back");
            });
            let s = &self.brush_engine.settings;
            if (s.symmetry_x || s.symmetry_y || s.symmetry_z) && ui.button("Center on mesh").clicked() {
                self.brush_engine.settings.symmetry_center = (self.project.mesh.aabb_min + self.project.mesh.aabb_max) * 0.5;
                self.status_message = "Symmetry plane centered on the mesh".to_string();
            }
        });
    }

    fn dynamics_card(&mut self, ui: &mut egui::Ui) {
        card(ui, "STROKE DYNAMICS", |ui| {
            let s = &mut self.brush_engine.settings;
            ui.checkbox(&mut s.lazy_mouse_enabled, "Lazy mouse (stabilizer)");
            if s.lazy_mouse_enabled {
                ui.add(egui::Slider::new(&mut s.lazy_mouse_radius, 0.01..=0.2).text("Distance"));
            }
            ui.checkbox(&mut s.pressure_size, "Pen pressure controls size");
            ui.checkbox(&mut s.pressure_opacity, "Pen pressure controls flow");
            ui.checkbox(&mut s.follow_path, "Follow path (rotate along the stroke)");
            if s.follow_path {
                ui.add(egui::Slider::new(&mut s.follow_path_offset, -std::f32::consts::PI..=std::f32::consts::PI).text("Angle offset"));
            }
            ui.add(egui::Slider::new(&mut s.size_jitter, 0.0..=1.0).text("Size jitter"));
            ui.add(egui::Slider::new(&mut s.position_jitter, 0.0..=1.0).text("Scatter"));
        });
    }

    fn stencil_card(&mut self, ui: &mut egui::Ui) {
        card(ui, "STENCIL", |ui| {
            ui.checkbox(&mut self.brush_engine.settings.stencil.enabled, "Project through a stencil");
            if !self.brush_engine.settings.stencil.enabled {
                return;
            }
            let st = &mut self.brush_engine.settings.stencil;
            egui::Grid::new("stencil_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                slider_row(ui, "Scale", egui::Slider::new(&mut st.scale, 0.1..=4.0));
                slider_row(ui, "Angle", egui::Slider::new(&mut st.angle, -std::f32::consts::PI..=std::f32::consts::PI));
                slider_row(ui, "Opacity", egui::Slider::new(&mut st.opacity, 0.05..=1.0));
            });
            ui.checkbox(&mut st.mask_only, "Stencil only masks the stroke");
            ui.horizontal_wrapped(|ui| {
                if ui.button("From shelf...").clicked() {
                    self.shelf_picker = Some(ShelfPickerState::new(ShelfPickerTarget::Stencil));
                }
                if ui.button("Load image...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Image", &["png", "jpg", "jpeg", "tga", "bmp", "exr", "hdr"])
                        .pick_file()
                    {
                        if let Ok(id) = self.project.import_texture_resource(&path, crate::project::ProjectResourceType::Texture) {
                            if let Some(res) = self.project.get_resource(&id).cloned() {
                                self.assign_resource_to_target(&res, ShelfPickerTarget::Stencil);
                            }
                        }
                    }
                }
                egui::ComboBox::from_id_source("stencil_preset_combo")
                    .selected_text("Built-in...")
                    .show_ui(ui, |ui| {
                        for preset in StencilPreset::ALL {
                            if ui.selectable_label(false, preset.name()).clicked() {
                                self.brush_engine.stencil_buffer = Some(preset.build());
                                self.status_message = format!("Stencil: {}", preset.name());
                            }
                        }
                    });
            });
        });
    }

    fn polygon_fill_card(&mut self, ui: &mut egui::Ui) {
        card(ui, "FILL BY", |ui| {
            ui.horizontal_wrapped(|ui| {
                let s = &mut self.brush_engine.settings;
                for (m, label, tip) in [
                    (PolygonFillMode::Triangle, "Triangle", "The triangle under the cursor"),
                    (PolygonFillMode::Polygon, "Polygon", "The quad or coplanar polygon"),
                    (PolygonFillMode::UvIsland, "UV island", "The whole connected UV island"),
                    (PolygonFillMode::MeshElement, "Mesh element", "The connected piece of the mesh"),
                ] {
                    if ui.add(tab_button(s.polygon_fill_mode == m, label)).on_hover_text(tip).clicked() {
                        s.polygon_fill_mode = m;
                    }
                }
            });
        });
        card(ui, "FILL WITH", |ui| {
            ui.horizontal(|ui| {
                let s = &mut self.brush_engine.settings;
                if ui.add(tab_button(s.target == PaintTarget::Material, "Material")).clicked() {
                    s.target = PaintTarget::Material;
                }
                if ui.add(tab_button(s.target == PaintTarget::Mask, "Mask")).clicked() {
                    s.target = PaintTarget::Mask;
                }
            });
            ui.add_space(4.0);
            if self.brush_engine.settings.target == PaintTarget::Mask {
                ui.horizontal(|ui| {
                    let s = &mut self.brush_engine.settings;
                    if ui.button("White (reveal)").clicked() {
                        s.polygon_fill_value = 1.0;
                        s.mask_paint_value = 1.0;
                    }
                    if ui.button("Black (hide)").clicked() {
                        s.polygon_fill_value = 0.0;
                        s.mask_paint_value = 0.0;
                    }
                });
                ui.add(egui::Slider::new(&mut self.brush_engine.settings.polygon_fill_value, 0.0..=1.0).text("Value"));
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Color").color(TEXT_MUTED));
                    crate::color_picker::color_picker_button_vec4(
                        ui,
                        egui::Id::new("poly_fill_color_btn"),
                        &mut self.brush_engine.settings.paint_color,
                    );
                });
                let s = &mut self.brush_engine.settings;
                egui::Grid::new("fill_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    slider_row(ui, "Roughness", egui::Slider::new(&mut s.paint_roughness, 0.0..=1.0));
                    slider_row(ui, "Metallic", egui::Slider::new(&mut s.paint_metallic, 0.0..=1.0));
                    slider_row(ui, "Height", egui::Slider::new(&mut s.paint_height, 0.0..=1.0));
                });
            }
        });
    }
}
