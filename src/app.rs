use crate::audio::AudioInput;
use crate::pitch::{self, Pitch};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use std::time::{Duration, Instant};

const BG: Color32 = Color32::from_rgb(13, 15, 18);
const PANEL: Color32 = Color32::from_rgb(22, 25, 30);
const PANEL_EDGE: Color32 = Color32::from_rgb(48, 54, 63);
const TEXT: Color32 = Color32::from_rgb(238, 241, 244);
const MUTED: Color32 = Color32::from_rgb(125, 134, 146);
const ACCENT: Color32 = Color32::from_rgb(89, 224, 170);
const WARM: Color32 = Color32::from_rgb(255, 181, 82);

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
        visuals.widgets.inactive.bg_fill = PANEL;
        visuals.widgets.inactive.fg_stroke.color = MUTED;
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

    fn mode_toggle(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let response = ui.interact(rect, ui.id().with("mode-toggle"), egui::Sense::click());
        if response.clicked() {
            self.mode = if self.mode == Mode::Needle {
                Mode::Strobe
            } else {
                Mode::Needle
            };
        }

        let painter = ui.painter();
        let switch = Rect::from_center_size(rect.center(), Vec2::new(52.0, 28.0));
        let knob_x = if self.mode == Mode::Needle {
            switch.left() + 14.0
        } else {
            switch.right() - 14.0
        };
        painter.rect_filled(switch, 14.0, Color32::from_rgb(42, 48, 57));
        painter.circle_filled(Pos2::new(knob_x, switch.center().y), 10.0, ACCENT);
        painter.text(
            Pos2::new(switch.left() - 12.0, switch.center().y),
            Align2::RIGHT_CENTER,
            "NEEDLE",
            FontId::proportional(11.0),
            if self.mode == Mode::Needle {
                TEXT
            } else {
                MUTED
            },
        );
        painter.text(
            Pos2::new(switch.right() + 12.0, switch.center().y),
            Align2::LEFT_CENTER,
            "STROBE",
            FontId::proportional(11.0),
            if self.mode == Mode::Strobe {
                TEXT
            } else {
                MUTED
            },
        );
    }

    fn note_display(&self, ui: &mut egui::Ui, area: Rect) {
        let painter = ui.painter();
        let card_size = area.height().min(118.0);
        let card = Rect::from_center_size(
            Pos2::new(area.center().x, area.top() + card_size * 0.5),
            Vec2::splat(card_size),
        );
        let tuned = self.pitch.is_some() && self.smoothed_cents.abs() < 2.0;
        painter.rect_filled(card, 14.0, PANEL);
        let edge = if tuned { ACCENT } else { PANEL_EDGE };
        for (a, b) in [
            (card.left_top(), card.right_top()),
            (card.right_top(), card.right_bottom()),
            (card.right_bottom(), card.left_bottom()),
            (card.left_bottom(), card.left_top()),
        ] {
            painter.line_segment([a, b], Stroke::new(1.5_f32, edge));
        }

        let (note, color) = match self.pitch {
            Some(pitch) => (pitch.note_name(), if tuned { ACCENT } else { TEXT }),
            None => ("—", MUTED),
        };
        painter.text(
            card.center(),
            Align2::CENTER_CENTER,
            note,
            FontId::proportional(if note.len() > 1 { 60.0 } else { 72.0 }),
            color,
        );
        if let Some(pitch) = self.pitch {
            painter.text(
                Pos2::new(card.right() - 15.0, card.bottom() - 13.0),
                Align2::RIGHT_BOTTOM,
                pitch.octave().to_string(),
                FontId::proportional(14.0),
                MUTED,
            );
        }

        let readout_y = card.bottom() + 24.0;
        if let Some(pitch) = self.pitch {
            painter.text(
                Pos2::new(area.center().x, readout_y),
                Align2::CENTER_CENTER,
                format!(
                    "{:+.1} CENTS   ·   {:.2} HZ",
                    self.smoothed_cents, pitch.frequency
                ),
                FontId::proportional(13.0),
                MUTED,
            );
        } else {
            painter.text(
                Pos2::new(area.center().x, readout_y),
                Align2::CENTER_CENTER,
                "PLAY A STRING",
                FontId::proportional(12.0),
                MUTED,
            );
        }
    }

    fn needle(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 16.0, PANEL);

        let center = Pos2::new(rect.center().x, rect.bottom() - 24.0);
        let radius = (rect.width() * 0.40).min(rect.height() - 34.0).max(72.0);

        let mut previous = None;
        for step in 0..=64 {
            let angle = std::f32::consts::PI * (1.0 - step as f32 / 64.0);
            let point = center + Vec2::new(angle.cos(), -angle.sin()) * radius;
            if let Some(last) = previous {
                painter.line_segment([last, point], Stroke::new(1.0_f32, PANEL_EDGE));
            }
            previous = Some(point);
        }

        for cents in (-50..=50).step_by(10) {
            let angle = std::f32::consts::PI * (1.0 - (cents as f32 + 50.0) / 100.0);
            let direction = Vec2::new(angle.cos(), -angle.sin());
            let major = cents % 25 == 0 || cents == 0;
            let outer = center + direction * radius;
            let inner = center + direction * (radius - if major { 16.0 } else { 8.0 });
            painter.line_segment(
                [inner, outer],
                Stroke::new(
                    if cents == 0 { 2.0_f32 } else { 1.0_f32 },
                    if cents == 0 { ACCENT } else { MUTED },
                ),
            );
            if cents % 25 == 0 {
                let label_pos = center + direction * (radius - 34.0);
                painter.text(
                    label_pos,
                    Align2::CENTER_CENTER,
                    if cents > 0 {
                        format!("+{cents}")
                    } else {
                        cents.to_string()
                    },
                    FontId::proportional(10.0),
                    MUTED,
                );
            }
        }

        let cents = self.smoothed_cents.clamp(-50.0, 50.0);
        let angle = std::f32::consts::PI * (1.0 - (cents + 50.0) / 100.0);
        let tip = center + Vec2::new(angle.cos(), -angle.sin()) * (radius - 22.0);
        let needle_color = if self.pitch.is_none() {
            MUTED
        } else if cents.abs() < 2.0 {
            ACCENT
        } else {
            WARM
        };
        painter.line_segment([center, tip], Stroke::new(3.0_f32, needle_color));
        painter.circle_filled(center, 8.0, TEXT);
        painter.circle_filled(center, 3.0, needle_color);

        let status = if self.pitch.is_none() {
            "WAITING FOR SIGNAL"
        } else if cents.abs() < 2.0 {
            "IN TUNE"
        } else if cents < 0.0 {
            "FLAT  ·  TUNE UP"
        } else {
            "SHARP  ·  TUNE DOWN"
        };
        painter.text(
            Pos2::new(rect.center().x, rect.top() + 20.0),
            Align2::CENTER_CENTER,
            status,
            FontId::proportional(11.0),
            if cents.abs() < 2.0 && self.pitch.is_some() {
                ACCENT
            } else {
                MUTED
            },
        );
    }

    fn strobe(&mut self, ui: &mut egui::Ui, rect: Rect, dt: f32) {
        let cents = if self.pitch.is_some() {
            self.smoothed_cents
        } else {
            0.0
        };
        self.strobe_phase = (self.strobe_phase + cents * dt * 0.035).rem_euclid(1.0);
        let painter = ui.painter();
        painter.rect_filled(rect, 16.0, PANEL);

        let stripe_area = rect.shrink2(Vec2::new(18.0, 38.0));
        let rows = 4;
        let gap = 9.0;
        let row_height = (stripe_area.height() - gap * (rows - 1) as f32) / rows as f32;
        for row in 0..rows {
            let y = stripe_area.top() + row as f32 * (row_height + gap);
            let stripe_width = 30.0 + row as f32 * 8.0;
            let offset = (self.strobe_phase * stripe_width * 2.0 * (row as f32 + 1.0))
                .rem_euclid(stripe_width * 2.0);
            let mut x = stripe_area.left() - stripe_width * 2.0 + offset;
            while x < stripe_area.right() {
                let stripe =
                    Rect::from_min_size(Pos2::new(x, y), Vec2::new(stripe_width, row_height));
                painter.rect_filled(
                    stripe.intersect(stripe_area),
                    4.0,
                    if cents.abs() < 0.5 {
                        ACCENT
                    } else {
                        Color32::from_rgb(82, 96, 110)
                    },
                );
                x += stripe_width * 2.0;
            }
        }
        painter.line_segment(
            [
                Pos2::new(rect.center().x, stripe_area.top()),
                Pos2::new(rect.center().x, stripe_area.bottom()),
            ],
            Stroke::new(2.0_f32, TEXT),
        );
        painter.text(
            Pos2::new(rect.center().x, rect.top() + 18.0),
            Align2::CENTER_CENTER,
            if self.pitch.is_none() {
                "WAITING FOR SIGNAL"
            } else {
                "STROBE SPEED = PITCH ERROR"
            },
            FontId::proportional(10.0),
            MUTED,
        );
    }

    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .exact_height(68.0)
            .frame(egui::Frame::none().fill(BG).inner_margin(20.0))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().text(
                    Pos2::new(rect.left(), rect.center().y),
                    Align2::LEFT_CENTER,
                    "POLAR",
                    FontId::proportional(17.0),
                    TEXT,
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 58.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    "TUNER",
                    FontId::proportional(17.0),
                    ACCENT,
                );
                let toggle = Rect::from_center_size(
                    Pos2::new(rect.right() - 95.0, rect.center().y),
                    Vec2::new(190.0, 36.0),
                );
                self.mode_toggle(ui, toggle);
            });
    }
}

impl eframe::App for TunerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.analyze();
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        ctx.request_repaint_after(Duration::from_millis(16));

        self.header(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(20.0))
            .show(ctx, |ui| {
                let canvas = ui.max_rect();
                let note_height = (canvas.height() * 0.34).clamp(150.0, 168.0);
                let note_area = Rect::from_min_max(
                    canvas.min,
                    Pos2::new(canvas.right(), canvas.top() + note_height),
                );
                let meter = Rect::from_min_max(
                    Pos2::new(canvas.left(), note_area.bottom() + 10.0),
                    canvas.max,
                );

                self.note_display(ui, note_area);
                if self.mode == Mode::Needle {
                    self.needle(ui, meter);
                } else {
                    self.strobe(ui, meter, dt);
                }
            });
    }
}
