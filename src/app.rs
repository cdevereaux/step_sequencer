use crate::instr;
use egui::RichText;
use egui::Color32;
use egui_extras::{TableBuilder, Column};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)]
    instr: instr::Instrument,

    // this how you opt-out of serialization of a member
    #[serde(skip)]
    value: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            instr: instr::Instrument::default(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self { label, value, instr } = self;

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        //#[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {

            ui.heading("eframe template");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            
            //Add instruments
            egui::Frame::group(ui.style())
                .fill(egui::Color32::LIGHT_BLUE)
                .show(ui, |ui| 
                {
                ui.collapsing("Instrument1", |ui| { 
                    egui::ScrollArea::both().show(ui, |ui| {
                        egui::Grid::new("Instrument1").show(ui, |ui| {
                            for note_num in (0..instr.notes.len()).rev() {
                                ui.label((note_num+21).to_string());
                                for meas_count in 0..instr.notes[note_num].len() {
                                    let note = instr.notes[note_num][meas_count];
                                    let button_text =
                                        if note.starts_at.is_some() {"▶"}
                                        else {"■"};
                                    if ui.button(RichText::new(button_text)
                                    .color(
                                        if note.duration > 0 || note.starts_at.is_some() {Color32::GREEN}
                                        else {Color32::TRANSPARENT}
                                    ))
                                    .clicked() {
                                        if let Some(last) = instr.last_clicked {
                                            if last.0 == note_num {
                                                if last.1 == meas_count {
                                                    instr.notes[note_num][meas_count].duration = 1;
                                                    instr.last_clicked = None;
                                                }
                                            }
                                            else {
                                                instr.last_clicked = None;
                                            }
                                        }
                                        else {
                                            instr.last_clicked = Some((note_num, meas_count));
                                        }
                                    }
                                }
                                ui.end_row();
                            }
                        });
                    });
                });
            });
            

            egui::warn_if_debug_build(ui);
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally choose either panels OR windows.");
            });
        }
    }
}



// use rodio::source::{SineWave, Source};

// fn play_sine() {
//     let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
//     let source = SineWave::new(440.0).take_duration(std::time::Duration::from_secs_f32(3.0));
//     stream_handle.play_raw(source).unwrap();
//     std::thread::sleep(std::time::Duration::from_secs_f32(3.0));
// }