use crate::{model::Sample, Error, Result};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    time::{Duration, Instant},
};
pub const FRAME: usize = 19;
pub fn crc(bytes: &[u8]) -> u16 {
    let mut value = 0xffffu16;
    for &b in bytes {
        value ^= (b as u16) << 8;
        for _ in 0..8 {
            value = if value & 0x8000 != 0 {
                (value << 1) ^ 0x1021
            } else {
                value << 1
            };
        }
    }
    value
}
pub fn packet(
    kind: u8,
    sequence: u32,
    timestamp: u32,
    value: u16,
    flags: u8,
    rate: u16,
) -> [u8; FRAME] {
    let mut b = [0; FRAME];
    b[..4].copy_from_slice(&[0xA5, 0x5A, 1, kind]);
    b[4..8].copy_from_slice(&sequence.to_le_bytes());
    b[8..12].copy_from_slice(&timestamp.to_le_bytes());
    b[12..14].copy_from_slice(&value.to_le_bytes());
    b[14] = flags;
    b[15..17].copy_from_slice(&rate.to_le_bytes());
    let c = crc(&b[2..17]);
    b[17..].copy_from_slice(&c.to_le_bytes());
    b
}
#[derive(Default)]
pub struct Parser {
    buffer: VecDeque<u8>,
    last: Option<u32>,
    pub dropped: u64,
    pub corrupt: u64,
}
impl Parser {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<[u8; FRAME]>> {
        if self.buffer.len() + bytes.len() > 8192 {
            self.buffer.clear();
            return Err(Error::Device("Serial parser overflow".into()));
        }
        self.buffer.extend(bytes);
        let mut frames = vec![];
        while self.buffer.len() >= FRAME {
            if self.buffer[0] != 0xA5 || self.buffer[1] != 0x5A {
                self.buffer.pop_front();
                continue;
            }
            let mut frame = [0; FRAME];
            for (i, b) in self.buffer.iter().take(FRAME).enumerate() {
                frame[i] = *b;
            }
            if crc(&frame[2..17]) != u16::from_le_bytes([frame[17], frame[18]]) {
                self.corrupt += 1;
                self.buffer.pop_front();
                continue;
            }
            self.buffer.drain(..FRAME);
            if frame[2] != 1 {
                return Err(Error::Device(format!(
                    "Firmware protocol {} incompatible; expected 1",
                    frame[2]
                )));
            }
            if frame[3] == 1 {
                let seq = u32::from_le_bytes(
                    frame[4..8]
                        .try_into()
                        .map_err(|_| Error::Device("Sequence field".into()))?,
                );
                if let Some(last) = self.last {
                    let diff = seq.wrapping_sub(last);
                    if diff == 0 || diff > 0x7fff_ffff {
                        return Err(Error::Device("Non-monotonic device sequence".into()));
                    }
                    self.dropped += (diff - 1) as u64;
                }
                self.last = Some(seq);
            }
            frames.push(frame);
        }
        Ok(frames)
    }
}
pub trait SignalSource: Send {
    fn read(&mut self) -> Result<Vec<Sample>>;
    fn sample_rate(&self) -> u32;
    fn stop(&mut self) -> Result<()>;
    fn health(&self) -> (u64, u64) {
        (0, 0)
    }
}
pub struct ArduinoSource {
    port: Box<dyn serialport::SerialPort>,
    parser: Parser,
    rate: u32,
    start: Instant,
    heartbeat: Instant,
    last_sample: Instant,
    pub firmware: u16,
}
impl ArduinoSource {
    pub fn connect(name: &str, baud: u32, rate: u32) -> Result<Self> {
        crate::require(
            !name.is_empty()
                && name.len() < 256
                && [115200, 230400].contains(&baud)
                && [250, 500, 1000].contains(&rate),
            "Invalid serial settings",
        )?;
        crate::require(
            rate < 1000 || baud >= 230400,
            "1000 Hz requires 230400 baud",
        )?;
        let port = serialport::new(name, baud)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|e| Error::Device(e.to_string()))?;
        let mut source = Self {
            port,
            parser: Parser::default(),
            rate,
            start: Instant::now(),
            heartbeat: Instant::now(),
            last_sample: Instant::now(),
            firmware: 0,
        };
        // Uno resets on DTR; repeat HELLO only on the explicitly selected device.
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut buf = [0; 512];
        while Instant::now() < deadline {
            source
                .port
                .write_all(&packet(0x10, 0, 0, 0, 0, rate as u16))?;
            match source.port.read(&mut buf) {
                Ok(n) => {
                    for f in source.parser.push(&buf[..n])? {
                        if f[3] == 2 {
                            source.firmware = u16::from_le_bytes([f[12], f[13]]);
                            if u16::from_le_bytes([f[15], f[16]]) != rate as u16 {
                                return Err(Error::Device(
                                    "Firmware rejected requested rate".into(),
                                ));
                            }
                            source
                                .port
                                .write_all(&packet(0x11, 0, 0, 0, 0, rate as u16))?;
                            return Ok(source);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e.into()),
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(Error::Device(
            "Handshake timed out. Flash Aiden firmware and check baud rate.".into(),
        ))
    }
}
impl SignalSource for ArduinoSource {
    fn read(&mut self) -> Result<Vec<Sample>> {
        if self.heartbeat.elapsed() > Duration::from_secs(1) {
            self.port
                .write_all(&packet(0x13, 0, 0, 0, 0, self.rate as u16))?;
            self.heartbeat = Instant::now();
        }
        let mut bytes = [0; 512];
        let n = match self.port.read(&mut bytes) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => 0,
            Err(e) => return Err(e.into()),
        };
        let mut out = vec![];
        for f in self.parser.push(&bytes[..n])? {
            if f[3] == 1 {
                let adc = u16::from_le_bytes([f[12], f[13]]);
                if adc > 1023 {
                    return Err(Error::Device("ADC outside 10-bit range".into()));
                }
                out.push(Sample {
                    sequence: u32::from_le_bytes([f[4], f[5], f[6], f[7]]),
                    hardware_us: u32::from_le_bytes([f[8], f[9], f[10], f[11]]),
                    acquired_us: self.start.elapsed().as_micros() as u64,
                    channel: 0,
                    adc,
                    leads: f[14],
                });
            }
        }
        if !out.is_empty() {
            self.last_sample = Instant::now();
        } else if self.last_sample.elapsed() > Duration::from_secs(2) {
            return Err(Error::Device("No samples for two seconds".into()));
        }
        Ok(out)
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn health(&self) -> (u64, u64) {
        (self.parser.dropped, self.parser.corrupt)
    }
    fn stop(&mut self) -> Result<()> {
        self.port
            .write_all(&packet(0x12, 0, 0, 0, 0, self.rate as u16))?;
        Ok(())
    }
}
impl Drop for ArduinoSource {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            tracing::warn!(error=%e,"serial shutdown");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc_vector() {
        assert_eq!(crc(b"123456789"), 0x29b1);
    }
    #[test]
    fn partial_garbage_crc_gaps_wrap() {
        let mut p = Parser::default();
        let a = packet(1, u32::MAX, 0, 512, 0, 500);
        assert!(p.push(&a[..7]).expect("parse").is_empty());
        assert_eq!(p.push(&a[7..]).expect("parse").len(), 1);
        let mut bad = packet(1, 0, 0, 512, 0, 500);
        bad[12] ^= 1;
        let mut bytes = vec![9, 8, 7];
        bytes.extend(bad);
        bytes.extend(packet(1, 1, 0, 512, 0, 500));
        assert_eq!(p.push(&bytes).expect("parse").len(), 1);
        assert_eq!(p.dropped, 1);
        assert_eq!(p.corrupt, 1);
    }
}
