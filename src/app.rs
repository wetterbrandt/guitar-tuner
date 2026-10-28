use crate::audio::AudioInput;
use crate::pitch::{self, Pitch};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use std::time::{Duration, Instant};

const BG: Color32 = Color32::from_rgb(13, 15, 18);
const PANEL: Color32 = Color32::from_rgb(23, 26, 31);
const TEXT: Color32 = Color32::from_rgb(232, 235, 238);
const MUTED: Color32 = Color32::from_rgb(121, 128, 138);
const ACCENT: Color32 = Color32::from_rgb(89, 224, 170);
const WARM: Color32 = Color32::from_rgb(255, 186, 92);

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Needle,
    Strobe,
}

pub struct TunerApp {
    audio: AudioInput,
    mode: Mode,
    pitch: Option<Pitch>,
    smoothed_cents: f32,
    strobe_phase: f32,
    last_analysis: Instant,
    last_frame: Instant,
}

impl TunerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = BG;
        visuals.extreme_bg_color = PANEL;
        visuals.selection.bg_fill = ACCENT;
        cc.egui_ctx.set_visuals(visuals);
        Self {
            audio: AudioInput::new(),
            mode: Mode::Needle,
            pitch: None,
            smoothed_cents: 0.0,
            strobe_phase: 0.0,
            last_analysis: Instant::now(),
            last_frame: Instant::now(),
        }
    }

    fn analyze(&mut self) {
        let interval = if self.mode == Mode::Strobe { 25 } else { 45 };
        if self.last_analysis.elapsed() < Duration::from_millis(interval) {
            return;
        }
        self.last_analysis = Instant::now();
        let wanted = (self.audio.sample_rate as usize / 6).clamp(4096, 12_000);
        self.pitch = pitch::detect(&self.audio.samples.snapshot(wanted), self.audio.sample_rate);
        if let Some(pitch) = self.pitch {
            let alpha = if self.mode == Mode::Strobe {
                0.38
            } else {
                0.22
            };
            self.smoothed_cents += (pitch.cents - self.smoothed_cents) * alpha;
        }
    }

    fn toggle(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("NEEDLE")
                    .color(if self.mode == Mode::Needle {
                        TEXT
                    } else {
                        MUTED
                    })
                    .size(11.0),
            );
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(54.0, 28.0), egui::Sense::click());
            if response.clicked() {
                self.mode = if self.mode == Mode::Needle {
                    Mode::Strobe
                } else {
                    Mode::Needle
                };
            }
            let t = if self.mode == Mode::Strobe { 1.0 } else { 0.0 };
            ui.painter()
                .rect_filled(rect, 14.0, Color32::from_rgb(42, 47, 55));
            ui.painter().circle_filled(
                Pos2::new(rect.left() + 14.0 + t * 26.0, rect.center().y),
                10.0,
                ACCENT,
            );
            ui.label(
                RichText::new("STROBE")
                    .color(if self.mode == Mode::Strobe {
                        TEXT
                    } else {
                        MUTED
                    })
                    .size(11.0),
            );
        });
    }

    fn needle(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter();
        let center = Pos2::new(rect.center().x, rect.bottom() - 28.0);
        let radius = rect.width().min(520.0) * 0.43;
        for cents in [-50, -25, 0, 25, 50] {
            let angle = std::f32::consts::PI * (1.0 - (cents as f32 + 50.0) / 100.0);
            let dir = Vec2::new(angle.cos(), -angle.sin());
            let outer = center + dir * radius;
            let inner = center + dir * (radius - if cents == 0 { 18.0 } else { 10.0 });
            painter.line_segment(
                [inner, outer],
                Stroke::new(
                    if cents == 0 { 2.0_f32 } else { 1.0_f32 },
                    if cents == 0 { ACCENT } else { MUTED },
                ),
            );
        }
        let cents = self.smoothed_cents.clamp(-50.0, 50.0);
        let angle = std::f32::consts::PI * (1.0 - (cents + 50.0) / 100.0);
        let tip = center + Vec2::new(angle.cos(), -angle.sin()) * (radius - 22.0);
        painter.line_segment(
            [center, tip],
            Stroke::new(3.0_f32, if cents.abs() < 2.0 { ACCENT } else { WARM }),
        );
        painter.circle_filled(center, 7.0, TEXT);
    }

    fn strobe(&mut self, ui: &mut egui::Ui, rect: Rect, dt: f32) {
        let cents = if self.pitch.is_some() {
            self.smoothed_cents
        } else {
            0.0
        };
        self.strobe_phase = (self.strobe_phase + cents * dt * 0.035).rem_euclid(1.0);
        let painter = ui.painter();
        let rows = 4;
        let gap = 10.0;
        let row_height = (rect.height() - gap * (rows - 1) as f32) / rows as f32;
        painter.rect_filled(rect, 12.0, PANEL);
        for row in 0..rows {
            let y = rect.top() + row as f32 * (row_height + gap);
            let stripe_width = 34.0 + row as f32 * 8.0;
            let offset = (self.strobe_phase * stripe_width * 2.0 * (row as f32 + 1.0))
                .rem_euclid(stripe_width * 2.0);
            let mut x = rect.left() - stripe_width * 2.0 + offset;
            while x < rect.right() {
                let stripe =
                    Rect::from_min_size(Pos2::new(x, y), Vec2::new(stripe_width, row_height));
                painter.rect_filled(
                    stripe.intersect(rect),
                    4.0,
                    if cents.abs() < 0.5 {
                        ACCENT
                    } else {
                        Color32::from_rgb(88, 102, 115)
                    },
                );
                x += stripe_width * 2.0;
            }
        }
        painter.line_segment(
            [
                Pos2::new(rect.center().x, rect.top()),
                Pos2::new(rect.center().x, rect.bottom()),
            ],
            Stroke::new(2.0_f32, TEXT),
        );
    }
}

impl eframe::App for TunerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.analyze();
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        ctx.request_repaint_after(Duration::from_millis(16));
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(28.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("POLAR TUNER").color(TEXT).size(14.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.toggle(ui)
                    });
                });
                ui.add_space(25.0);
                ui.vertical_centered(|ui| {
                    if let Some(pitch) = self.pitch {
                        ui.horizontal_centered(|ui| {
                            ui.label(
                                RichText::new(pitch.note_name())
                                    .font(FontId::proportional(82.0))
                                    .color(TEXT)
                                    .strong(),
                            );
                            ui.label(
                                RichText::new(pitch.octave().to_string())
                                    .size(24.0)
                                    .color(MUTED),
                            );
                        });
                        ui.label(
                            RichText::new(format!(
                                "{:+.1} cents   •   {:.2} Hz",
                                self.smoothed_cents, pitch.frequency
                            ))
                            .size(14.0)
                            .color(
                                if self.smoothed_cents.abs() < 2.0 {
                                    ACCENT
                                } else {
                                    MUTED
                                },
                            ),
                        );
                    } else {
                        ui.label(
                            RichText::new("—")
                                .font(FontId::proportional(82.0))
                                .color(MUTED),
                        );
                        ui.label(RichText::new("Play a string").size(14.0).color(MUTED));
                    }
                });
                ui.add_space(20.0);
                let height = ui.available_height() - 76.0;
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), height.max(180.0)),
                    egui::Sense::hover(),
                );
                if self.mode == Mode::Needle {
                    self.needle(ui, rect);
                } else {
                    self.strobe(ui, rect.shrink2(Vec2::new(22.0, 16.0)), dt);
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("INPUT").size(10.0).color(MUTED));
                    let current = self
                        .audio
                        .devices
                        .get(self.audio.selected)
                        .cloned()
                        .unwrap_or_else(|| "No input device".into());
                    let mut chosen = self.audio.selected;
                    egui::ComboBox::from_id_salt("input-device")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for (index, name) in self.audio.devices.iter().enumerate() {
                                ui.selectable_value(&mut chosen, index, name);
                            }
                        });
                    if chosen != self.audio.selected {
                        self.audio.start(chosen);
                    }
                    if let Some(error) = &self.audio.error {
                        ui.label(RichText::new(error).size(11.0).color(Color32::LIGHT_RED));
                    }
                });
            });
    }
}
