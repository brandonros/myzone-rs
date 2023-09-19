use core::fmt;

#[derive(Debug)]
pub enum HeartRateError {
    InvalidLength,
}

impl fmt::Display for HeartRateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            HeartRateError::InvalidLength => "Invalid length".to_string(),
        };
        write!(f, "Heart rate error: {}", msg)
    }
}

fn bytes_to_data(data: &[u8], len: usize) -> i32 {
    if len == 3 {
        let mut buf = [0u8; 4];
        buf[..len].copy_from_slice(&data[..len]);
        // Check if most significant byte is negative and retain that negative
        if (data[len - 1] & 0x80) > 0 {
            for i in buf[len..].iter_mut() {
                *i = 0xff;
            }
        }
        i32::from_le_bytes(buf)
    } else if len == 2 {
        let mut buf = [0u8; 2];
        buf[..len].copy_from_slice(&data[..len]);
        if (data[len - 1] & 0x80) > 0 {
            for i in buf[len..].iter_mut() {
                *i = 0xff;
            }
        }
        let small = i16::from_le_bytes(buf);
        i32::from(small)
    } else {
        let mut buf = [0u8; 1];
        buf[..len].copy_from_slice(&data[..len]);
        let small = i8::from_le_bytes(buf);
        i32::from(small)
    }
}

#[derive(Debug)]
pub struct HeartRate {
    bpm: u8,
    rr: Option<Vec<u16>>,
}

impl HeartRate {
    /// Create new instance of [`HeartRate`]
    pub fn new(data: &Vec<u8>) -> Result<HeartRate, HeartRateError> {
        if data.len() < 2 {
            eprintln!(
                "Heart rate expects atleast 2 bytes of data, got {}",
                data.len()
            );
            return Err(HeartRateError::InvalidLength);
        }
        let flags = data[0];
        let samples = if flags & 0b00010000 == 16 {
            (data.len() - 2) / 2
        } else {
            0
        };
        let bpm = data[1];
        let mut rr_samp = vec![];
        for i in 0..samples {
            // rr values are stored as 1024ths of a second, convert to ms
            rr_samp.push(((bytes_to_data(&data[i * 2 + 2..i * 2 + 4], 2) as u32 * 128) / 125) as u16);
        }
        let rr = if !rr_samp.is_empty() {
            Some(rr_samp)
        } else {
            None
        };
        Ok(HeartRate { bpm, rr, })
    }

    /// Get BPM of heart rate measurement
    pub fn bpm(&self) -> u8 {
        self.bpm
    }

    /// Get RR interval as a tuple
    pub fn rr(&self) -> &Option<Vec<u16>> {
        &self.rr
    }    
}