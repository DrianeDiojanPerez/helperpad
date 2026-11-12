use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui::{self, Color32, Key, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

use crate::pad::{self, Ink, Point, X_MAX, X_MIN, Y_MAX, Y_MIN};

pub enum Outcome {
    Accepted(Vec<Vec<Point>>),
    Cancelled,
}

struct SigningWindow {
    ink: Arc<Mutex<Ink>>,
    pad_error: Arc<Mutex<Option<String>>>,
    min_points: usize,
    outcome: Rc<RefCell<Outcome>>,
}

pub fn run(device: PathBuf, min_points: usize) -> Result<Outcome, String> {
    let outcome = Rc::new(RefCell::new(Outcome::Cancelled));
    let app_outcome = outcome.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Topaz signature")
            .with_app_id("helperpad")
            .with_inner_size([720.0, 320.0])
            .with_resizable(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop),
        ..Default::default()
    };

    eframe::run_native(
        "helperpad",
        options,
        Box::new(move |cc| {
            let ink = Arc::new(Mutex::new(Ink::default()));
            let pad_error = Arc::new(Mutex::new(None));
            spawn_reader(device, ink.clone(), pad_error.clone(), cc.egui_ctx.clone());

            Ok(Box::new(SigningWindow {
                ink,
                pad_error,
                min_points,
                outcome: app_outcome,
            }))
        }),
    )
    .map_err(|e| e.to_string())?;

    Ok(outcome.replace(Outcome::Cancelled))
}

fn spawn_reader(
    device: PathBuf,
    ink: Arc<Mutex<Ink>>,
    pad_error: Arc<Mutex<Option<String>>>,
    ctx: egui::Context,
) {
    thread::spawn(move || {
        let result = pad::listen(&device, |event| {
            ink.lock().unwrap().apply(event);
            ctx.request_repaint();
        });

        let message = match result {
            Ok(()) => "The pad was unplugged.".to_string(),
            Err(e) => format!("Can't read the pad at {}: {e}", device.display()),
        };
        *pad_error.lock().unwrap() = Some(message);
        ctx.request_repaint();
    });
}

impl SigningWindow {
    fn finish(&self, ui: &egui::Ui, outcome: Outcome) {
        *self.outcome.borrow_mut() = outcome;
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl eframe::App for SigningWindow {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ink = self.ink.lock().unwrap().clone();
        let points = ink.point_count();
        let enough = points >= self.min_points;
        let pad_error = self.pad_error.lock().unwrap().clone();

        let (enter, escape) = ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));

        let button = |text: &str| egui::Button::new(RichText::new(text).size(16.0)).min_size(Vec2::new(90.0, 34.0));

        egui::Panel::bottom("toolbar").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add(button("Clear")).clicked() {
                    self.ink.lock().unwrap().clear();
                }
                if ui.add_enabled(enough, button("OK")).clicked() || (enter && enough) {
                    self.finish(ui, Outcome::Accepted(ink.strokes.clone()));
                }
                if ui.add(button("Cancel")).clicked() || escape {
                    self.finish(ui, Outcome::Cancelled);
                }

                ui.separator();
                let status = match &pad_error {
                    Some(error) => RichText::new(error).color(Color32::from_rgb(220, 60, 60)),
                    None if points == 0 => RichText::new("Pad initialized, start signing"),
                    None if !enough => RichText::new(format!("Keep signing ({points}/{} points)", self.min_points)),
                    None => RichText::new("Press OK to accept"),
                };
                ui.label(status.size(15.0));
            });
            ui.add_space(8.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            let available = ui.available_rect_before_wrap();
            let pad_size = Vec2::new(X_MAX - X_MIN, Y_MAX - Y_MIN);
            let scale = (available.width() / pad_size.x).min(available.height() / pad_size.y);
            let area = Rect::from_center_size(available.center(), pad_size * scale);
            ui.allocate_rect(available, Sense::hover());

            let painter = ui.painter_at(available);
            painter.rect_filled(area, 6.0, Color32::WHITE);

            let to_screen = |p: &Point| Pos2::new(area.left() + (p.x - X_MIN) * scale, area.top() + (p.y - Y_MIN) * scale);
            let pen = Stroke::new(2.5, Color32::BLACK);

            for stroke in &ink.strokes {
                let line: Vec<Pos2> = stroke.iter().map(to_screen).collect();
                if line.len() == 1 {
                    painter.circle_filled(line[0], pen.width / 2.0, pen.color);
                } else {
                    painter.add(Shape::line(line, pen));
                }
            }
        });
    }
}
