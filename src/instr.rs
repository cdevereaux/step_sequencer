
#[derive(Clone, Copy)]
pub struct Note {
    pub duration: u32, //in sixteenth notes
    pub starts_at: Option<u32>,
} 

impl Default for Note {
    fn default() -> Self {
        Note { duration: 0, starts_at: None }
    }
}

pub struct Instrument {
    pub notes: Box<[[Note; 64]; 107]>,
    pub last_clicked: Option<(usize, usize)>,

}

impl Default for Instrument {
    fn default() -> Self {
        Instrument { 
            notes: Box::new([[Note::default(); 64]; 107]),
            last_clicked: None,
        }
    }
}