use crate::instr;
use crate::instr::MEAS_COUNT;
use crate::instr::NOTE_COUNT;
use crate::instr::Note;
use crate::synth;
use crate::synth::Oscillator;
use std::sync::mpsc;
use egui::RichText;
use egui::Color32;

pub enum AudioState {
    Playing,
    Off
}

pub enum Messages {
    Play(Vec<(Note, usize, usize)>),
    Stop,
    Record,
    Tempo(u32),
    Oscillator(Oscillator)
}


/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct StepSequencer {
    
    instr: instr::Instrument,
    osc: Oscillator,
    #[serde(skip)]
    tempo: u32,
    #[serde(skip)]
    audio_state: AudioState,
    #[serde(skip)]
    tx: mpsc::Sender<Messages>,
    #[serde(skip)]
    recording: bool,
}

impl Default for StepSequencer {
    fn default() -> Self {
        Self {
            // Example stuff:
            audio_state: AudioState::Off,
            instr: instr::Instrument::default(),
            tempo: 60,
            tx: {
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    synth::process_audio(rx);
                });
                tx
            },
            recording: false,
            osc: Oscillator::Sin,
        }
    }
}

impl StepSequencer {
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

impl eframe::App for StepSequencer {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self { 
            audio_state,
            instr, 
            tx,
            recording,
            tempo,
            mut osc,
        } = self;


        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                if ui.button(if let AudioState::Playing = audio_state {"⏹"} else {"▶"}).clicked() {
                    match audio_state {
                        AudioState::Playing => { 
                            *audio_state = AudioState::Off;
                            tx.send(Messages::Stop).unwrap();
                            *recording = false;
                        }
                        AudioState::Off => { 
                            *audio_state = AudioState::Playing;
                            send_instrument_state(tx, instr);
                        }
                    }
                }
                if ui.add_enabled(!*recording, egui::Button::new("⏺")).clicked() {
                    *recording = true;
                    tx.send(Messages::Record).unwrap();
                    *audio_state = AudioState::Playing;
                    send_instrument_state(tx, instr);
                }
                if ui.add(egui::DragValue::new(tempo)
                .clamp_range(1..=240).prefix("Tempo: ").suffix(" bpm")).changed() {
                    tx.send(Messages::Tempo(*tempo)).unwrap();
                };
                let old_osc = osc;
                egui::ComboBox::from_id_source("instrument")
                .selected_text(format!("{:?}", osc))
                .show_ui(ui, |ui| {
                    ui.selectable_value( &mut osc, Oscillator::Sin, "Sin");
                    ui.selectable_value( &mut osc, Oscillator::Sawtooth, "Sawtooth");
                    ui.selectable_value( &mut osc, Oscillator::Triangle, "Triangle");
                    ui.selectable_value( &mut osc, Oscillator::Pulse, "Pulse");
                });
                if osc != old_osc {
                    tx.send(Messages::Oscillator(osc)).unwrap();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            
            //Add instruments
            draw_instrument(ui, instr, audio_state);

            egui::warn_if_debug_build(ui);
        });
    }
}

fn draw_instrument(ui: &mut egui::Ui, instr: &mut instr::Instrument, audio_state: &AudioState) {
    egui::Frame::group(ui.style())
    .fill(egui::Color32::LIGHT_BLUE)
    .show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 20.0);
                draw_note_grid(ui, instr, audio_state);                
            });
    });
}

fn draw_note_grid(ui: &mut egui::Ui, instr: &mut instr::Instrument, audio_state: &AudioState) {
    egui::Grid::new("Instrument1").striped(true).show(ui, |ui| {
        for note_num in (0..instr::NOTE_COUNT).rev() {
            ui.label(instr::note_num_to_str(note_num+21));
            for meas_num in 0..instr::MEAS_COUNT {
                if ui.add_enabled(if let AudioState::Playing = audio_state {false} else {true}, 
                egui::Button::new(get_note_button_text(instr, note_num, meas_num))).clicked() {
                    handle_note_button_click(instr, note_num, meas_num);
                }
            }
            ui.end_row();
        }
    });
}

fn get_note_button_text(instr: &instr::Instrument, note_num: usize, meas_num: usize) -> egui::RichText {
    let note = instr.get_note(note_num, meas_num);
    let button_text =
        if let Some(start_ind) = note.starts_at {
            if instr.get_note(note_num, start_ind).duration + start_ind - 1 != meas_num {"▶"}
            else {"■"}
        }
        else if instr.last_clicked.is_some() 
            && note_num == instr.last_clicked.unwrap().0 
            && meas_num == instr.last_clicked.unwrap().1 
            {"☐"}
        else {"■"};
    RichText::new(button_text)
    .color(
        if note.duration > 0 || note.starts_at.is_some() {Color32::GREEN}
        else if instr.last_clicked.is_some() 
            && note_num == instr.last_clicked.unwrap().0 
            && meas_num == instr.last_clicked.unwrap().1 
            {Color32::GREEN}
        else {Color32::TRANSPARENT}
    )
}

fn handle_note_button_click(instr: &mut instr::Instrument, note_num: usize, meas_num: usize) {

    //remove any existing intersecting notes
    if let Some(start_ind) = instr.get_note(note_num, meas_num).starts_at {
        for i in 0..instr.get_note(note_num, start_ind).duration {
            *instr.get_note_mut(note_num, start_ind+i) = instr::Note::default();
        }
    }

    if let Some(last) = instr.last_clicked {
        if last.0 == note_num {
            if last.1 <= meas_num {
                instr.get_note_mut(note_num, last.1).duration = meas_num - last.1 + 1;
                instr.last_clicked = None;
                for i in last.1..=meas_num {
                    instr.get_note_mut(note_num, i).starts_at = Some(last.1);
                }
            }
            else {
                instr.last_clicked = Some((note_num, meas_num));
            }
        }
        else {
            instr.last_clicked = None;
        }
    }
    else {
        instr.last_clicked = Some((note_num, meas_num));
    }
}

fn send_instrument_state(
    tx: &mut mpsc::Sender<Messages>, 
    instr: &mut instr::Instrument, 
) {

    let mut changed_notes = Vec::new();
    for i in 0..NOTE_COUNT {
        for j in 0..MEAS_COUNT {
            let note = instr.get_note(i, j);
            if note != Note::default() {
                changed_notes.push((note, i, j));
            }
        }
    }

    tx.send(Messages::Play(changed_notes)).unwrap();
}