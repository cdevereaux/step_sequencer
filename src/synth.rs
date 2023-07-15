use std::f32::consts::PI;
use std::time::Duration;

use rodio;
use hound;

use crate::{instr::{Note, MEAS_COUNT, NOTE_COUNT}, app::Messages};

#[derive(Clone, Debug, PartialEq)]
enum EnvelopeState {
    Attack,
    Decay,
    Sustain,
    Release,
    ToBeReleased,
    Dead,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum Oscillator {
    Sin,
    Sawtooth,
    Pulse,
    Triangle,
}

impl std::fmt::Display for Oscillator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone, Debug)]
pub struct MidiNote {
    amplitude: f32,
    freq: f32,
    num_sample: usize,
    state: EnvelopeState,
    osc: Oscillator
}

pub const SR: u32 = 44100;

impl MidiNote {
    #[inline]
    pub fn new(note: usize, oscillator: Oscillator) -> MidiNote {
        MidiNote {
            amplitude: 0.0,
            freq: 440.0 * ( ( (note as f32) - 69.0 ) / 12.0 ).exp2(),
            num_sample: 0,
            state: EnvelopeState::Attack,
            osc: oscillator,
        }
    }

    pub fn get_buffer(&mut self, buffer_len: usize) -> Vec<f32> {
        let mut buffer = vec![0_f32; buffer_len];
        for i in 0..buffer.len() {
            buffer[i] = self.next().unwrap();
        }
        buffer
    }

    pub fn release(&mut self) {
        self.state = 
            if self.state == EnvelopeState::Attack {EnvelopeState::ToBeReleased}
            else {EnvelopeState::Release};
    }

    pub fn press(&mut self) {
        self.state = EnvelopeState::Attack;
    }

    pub fn is_alive(&self) -> bool {
        self.state != EnvelopeState::Dead
    }

    fn apply_envelope(&mut self) {
        match self.state {
            EnvelopeState::Attack => {
                self.amplitude += 16.0/(SR as f32);
                if self.amplitude >= 1.0 {
                    self.amplitude = 1.0;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                self.amplitude -= 6.0/(SR as f32);
                if self.amplitude <= 0.8 {
                    self.amplitude = 0.8;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {}
            EnvelopeState::ToBeReleased => {
                self.amplitude += 16.0/(SR as f32);
                if self.amplitude >= 1.0 {
                    self.amplitude = 1.0;
                    self.state = EnvelopeState::Release;
                }
            }
            EnvelopeState::Release => {
                self.amplitude -= 4.0/(SR as f32);
                if self.amplitude <= 0.0 {
                    self.amplitude = 0.0;
                    self.state = EnvelopeState::Dead;
                }
            }
            EnvelopeState::Dead => {}
        }
    }

    fn sawtooth(&self, x: f32) -> f32 {
        ((x%(2.0*PI))-PI)/PI
    }
        
    fn oscillator(&self, x: f32) -> f32 {
        match self.osc {
            Oscillator::Sin => {
                x.sin()
            }
            Oscillator::Sawtooth => {
                self.sawtooth(x)
            }
            Oscillator::Pulse => {
                self.sawtooth(x).signum()
            }
            Oscillator::Triangle => {
                1.0 - 2.0*self.sawtooth(x).abs()
            }
        }
    }

    pub fn set_oscillator(&mut self, oscillator: &Oscillator) {
        self.osc = *oscillator;
    } 

}

impl Iterator for MidiNote {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        self.num_sample = self.num_sample.wrapping_add(1);
        
        self.apply_envelope();

        let value = 2.0 * PI * self.freq * self.num_sample as f32 / (SR as f32);
        Some(self.amplitude * self.oscillator(value))
    }
}

impl rodio::Source for MidiNote {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        1
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        SR
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}


pub fn process_audio (
    rx: std::sync::mpsc::Receiver<Messages>,
) {
    let (_stream, stream_handle) = 
        rodio::OutputStream::try_default().expect("Could not get output device.");
    let sink = rodio::Sink::try_new(&stream_handle).expect("Could not get output device.");    
    let mut midi_notes: std::collections::HashMap<usize, MidiNote> = std::collections::HashMap::new();

    let mut instr_state = vec![vec![Note::default(); MEAS_COUNT]; NOTE_COUNT];
    let mut tempo = 60;
    let mut clock = 0;

    let mut recording = false;
    let mut recorded_data: Vec<f32> = vec![];

    let mut osc = Oscillator::Sin;
    
    let mut active = false; 
    loop {
        if let Ok(msg) = rx.try_recv() {
            match msg {
                Messages::Play(changed_notes) => {
                    for (new_note, i, j) in changed_notes {
                        instr_state[i][j] = new_note;
                    }
                    active = true;
                    sink.play();
                }
                Messages::Stop => {
                    active = false;
                    if recording {
                        export_wav(recorded_data.clone());
                        recorded_data.clear();
                    }
                    //reset instrument
                    for row in instr_state.iter_mut() {
                        for note in row.iter_mut() {
                            *note = Note::default();
                        }
                    }
                    clock = 0;
                }
                Messages::Record => {
                    recording = true;
                }
                Messages::Tempo(new_tempo) => {
                    tempo = new_tempo;
                }
                Messages::Oscillator(new_osc) => {
                    osc = new_osc;
                    for note in midi_notes.values_mut() {
                        note.set_oscillator(&osc);
                    }
                }
            }
        }
    
        if !active || sink.len() > 2 {continue;}

        for ind in 0..NOTE_COUNT {
            let note = instr_state[ind][clock];
            let note_num = ind + 21;
            if note.duration != 0 {
                match midi_notes.get_mut(&note_num) {
                    Some(midi_note) => {
                        midi_note.press();
                    }
                    None => {
                        midi_notes.insert(note_num, MidiNote::new(note_num, osc.clone()));
                    }
                }
            }
            if let Some(start) = note.starts_at {
                if instr_state[ind][start].duration + start - 1 == clock{
                    if let Some(midi_note) = midi_notes.get_mut(&note_num) {
                        midi_note.release();
                    }
                }
            }
        }
        let mut counter = 0.0;
        let buffer_len = (SR as f32 * 0.125 * 60.0/tempo as f32).round() as usize;
        let mut data = vec![0_f32; buffer_len];
        for midi_note in midi_notes.values_mut() {
            if midi_note.is_alive() {
                data.iter_mut().zip(midi_note.get_buffer(buffer_len).iter()).for_each(|(d, b)| {
                    *d += b;
                });
                counter += 1.0;
            }
        }
        data.iter_mut().for_each(|d| {
            *d /= counter;
        });

        if recording {recorded_data.extend_from_slice(&data);}
        sink.append(rodio::buffer::SamplesBuffer::new(1, SR, data));
        clock = (clock + 1) % MEAS_COUNT;
    }
}

fn export_wav(data: Vec<f32>) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create("step_sequencer_recording.wav", spec).unwrap();
    for datum in data {
        writer.write_sample(datum).unwrap();
    }
    writer.finalize().unwrap();
}