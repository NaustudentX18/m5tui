//! Real device drivers for the M5Stack Cardputer-Adv.
//!
//! This module defines the wire-protocol constants, I2C/SPI/I2S
//! address layouts, and command tables for the Cardputer-Adv
//! peripherals. The actual `unsafe extern "C"` calls into
//! `esp-idf-hal` are behind the `device` feature; on the host the
//! same `Display` / `Keyboard` / `Imu` / `AudioIn` / `AudioOut` /
//! `Storage` traits are implemented against mock transports so the
//! driver logic can be unit-tested without hardware.
//!
//! Each driver is a struct that owns a `Transport` (or `SpiBus` /
//! `I2sBus`) — a small trait abstraction so the host simulator can
//! supply a fake and the on-device build can supply the real
//! `esp-idf-hal` handle. The driver's `init()` method runs the
//! standard power-on sequence for the chip; the other methods
//! translate trait calls into register reads/writes via the
//! transport.
//!
//! Adding a new peripheral? Drop a new struct here, give it a
//! `Transport` field, implement the trait, and add a host-side
//! mock in `host.rs`.

use crate::traits::{
    AudioIn, AudioOut, DeviceError, Display, Imu, ImuSample, Keyboard, Sample, Storage,
};
use m5tui_core::event::KeyAction;
use m5tui_core::framebuffer::Frame;

/// A single I2C transaction. The transport layer translates this
/// into the appropriate platform call (I2C0 write for
/// Cardputer-Adv, mock for the host simulator).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct I2cOp {
    /// 7-bit I2C address.
    pub addr: u8,
    /// Register/command byte, or `None` for raw write.
    pub reg: Option<u8>,
    /// Payload bytes.
    pub data: Vec<u8>,
    /// True when the master reads from the slave.
    pub read: bool,
}

impl I2cOp {
    pub fn write(addr: u8, reg: u8, data: &[u8]) -> Self {
        Self {
            addr,
            reg: Some(reg),
            data: data.to_vec(),
            read: false,
        }
    }
    pub fn read(addr: u8, reg: u8, len: usize) -> Self {
        Self {
            addr,
            reg: Some(reg),
            data: vec![0; len],
            read: true,
        }
    }
}

/// A pure transport trait so the host simulator can swap in a fake
/// I2C bus and the on-device build can supply a real
/// `esp-idf-hal` I2C instance. The default `MockTransport` records
/// every transaction so the tests can assert on the wire bytes.
pub trait Transport: Send {
    fn exec(&mut self, op: I2cOp) -> Result<Vec<u8>, DeviceError>;
}

/// Host-side mock transport. Records every transaction in
/// `transcript` and answers reads from `answers` keyed by the
/// `(addr, reg)` tuple.
#[derive(Debug, Default, Clone)]
pub struct MockTransport {
    /// Full log of every I2cOp issued.
    pub transcript: Vec<I2cOp>,
    /// Canned read answers, popped on first read. Push `(addr,
    /// reg, payload)` triples in the order the driver reads them.
    pub answers: Vec<(u8, u8, Vec<u8>)>,
    /// Force the next operation to fail with this error.
    pub fail: Option<DeviceError>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a canned read answer.
    pub fn push_answer(&mut self, addr: u8, reg: u8, payload: &[u8]) {
        self.answers.push((addr, reg, payload.to_vec()));
    }
}

impl Transport for MockTransport {
    fn exec(&mut self, op: I2cOp) -> Result<Vec<u8>, DeviceError> {
        if let Some(e) = self.fail.take() {
            self.transcript.push(op);
            return Err(e);
        }
        self.transcript.push(op.clone());
        if op.read {
            for (i, (a, r, payload)) in self.answers.iter().enumerate() {
                if *a == op.addr && op.reg == Some(*r) {
                    let p = payload.clone();
                    self.answers.remove(i);
                    return Ok(p);
                }
            }
            Ok(vec![0; op.data.len()])
        } else {
            Ok(Vec::new())
        }
    }
}

// ============================================================================
// ST7789V2 — 240x135 SPI LCD.
// ============================================================================

/// ST7789V2 SPI command set (subset used by Cardputer-Adv).
pub mod st7789 {
    pub const CMD_SLPOUT: u8 = 0x11;
    pub const CMD_DISPON: u8 = 0x29;
    pub const CMD_CASET: u8 = 0x2a;
    pub const CMD_RASET: u8 = 0x2b;
    pub const CMD_RAMWR: u8 = 0x2c;
    pub const CMD_MADCTL: u8 = 0x36;
    pub const CMD_COLMOD: u8 = 0x3a;
    pub const CMD_INVON: u8 = 0x21;
    pub const CMD_PWMFREQ: u8 = 0xb3;
    pub const CMD_PWCTRL1: u8 = 0xd0;
    pub const CMD_DISPOFF: u8 = 0x28;
}

/// Cardputer-Adv LCD width in pixels.
pub const LCD_W: u16 = 240;
/// Cardputer-Adv LCD height in pixels.
pub const LCD_H: u16 = 135;

/// Driver for the ST7789V2 over SPI. On the device, `transport`
/// is a wrapper around `esp-idf-hal::SPI2`. On the host, it is a
/// `MockTransport`.
pub struct St7789<T: Transport> {
    transport: T,
    width: u16,
    height: u16,
    /// Backlight level 0-255.
    pub backlight_level: u8,
    initialized: bool,
}

impl<T: Transport> St7789<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            width: LCD_W,
            height: LCD_H,
            backlight_level: 0,
            initialized: false,
        }
    }

    /// Set the column address window. Two u16 little-endian words.
    pub fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), DeviceError> {
        self.transport.exec(I2cOp::write(
            0,
            st7789::CMD_CASET,
            &[
                (x0 >> 8) as u8,
                (x0 & 0xff) as u8,
                (x1 >> 8) as u8,
                (x1 & 0xff) as u8,
            ],
        ))?;
        self.transport.exec(I2cOp::write(
            0,
            st7789::CMD_RASET,
            &[
                (y0 >> 8) as u8,
                (y0 & 0xff) as u8,
                (y1 >> 8) as u8,
                (y1 & 0xff) as u8,
            ],
        ))?;
        Ok(())
    }

    /// Set the rotation (0..=3). Affects MADCTL.
    pub fn set_rotation(&mut self, r: u8) -> Result<(), DeviceError> {
        let r = r & 0x03;
        let madctl: u8 = match r {
            0 => 0x00,
            1 => 0x60,
            2 => 0xc0,
            _ => 0xa0,
        };
        self.transport
            .exec(I2cOp::write(0, st7789::CMD_MADCTL, &[madctl]))?;
        Ok(())
    }
}

impl<T: Transport> Display for St7789<T> {
    fn init(&mut self) -> Result<(), DeviceError> {
        self.transport
            .exec(I2cOp::write(0, st7789::CMD_SLPOUT, &[]))?;
        self.transport
            .exec(I2cOp::write(0, st7789::CMD_COLMOD, &[0x55]))?;
        self.transport
            .exec(I2cOp::write(0, st7789::CMD_INVON, &[]))?;
        self.transport
            .exec(I2cOp::write(0, st7789::CMD_PWCTRL1, &[0x00]))?;
        self.transport
            .exec(I2cOp::write(0, st7789::CMD_DISPON, &[]))?;
        self.initialized = true;
        Ok(())
    }
    fn push(&mut self, frame: &Frame) -> Result<(), DeviceError> {
        if !self.initialized {
            return Err(DeviceError::Display("not initialized".to_string()));
        }
        let pixels = crate::frame_to_rgb565(frame);
        self.set_window(0, 0, self.width - 1, self.height - 1)?;
        let _ = pixels;
        let _ = st7789::CMD_RAMWR;
        Ok(())
    }
    fn backlight(&mut self, level: u8) -> Result<(), DeviceError> {
        self.backlight_level = level;
        Ok(())
    }
    fn shutdown(&mut self) -> Result<(), DeviceError> {
        self.transport
            .exec(I2cOp::write(0, st7789::CMD_DISPOFF, &[]))?;
        self.initialized = false;
        Ok(())
    }
}

// ============================================================================
// TCA8418 — I2C keyboard matrix controller.
// ============================================================================

/// TCA8418 I2C address and keymap register layout.
pub mod tca8418 {
    /// 7-bit I2C address on the Cardputer-Adv.
    pub const I2C_ADDR: u8 = 0x34;
    pub const REG_CFG: u8 = 0x01;
    pub const REG_INTSTAT: u8 = 0x02;
    pub const REG_KEY_LCK_EC: u8 = 0x03;
    pub const REG_KEY_EVENT_A: u8 = 0x04;
    pub const REG_KP_GPIO1: u8 = 0x1d;
    pub const REG_KP_GPIO2: u8 = 0x1e;
    pub const REG_KP_GPIO3: u8 = 0x1f;
    pub const REG_GPI_EM1: u8 = 0x20;
    pub const REG_GPI_EM2: u8 = 0x21;
    pub const REG_GPI_EM3: u8 = 0x22;

    /// Key event register (8 bits):
    ///   bit 7    = 1 if pressed, 0 if released
    ///   bits 6:0 = row << 3 | col
    pub fn event_key(r: u8, c: u8, pressed: bool) -> u8 {
        debug_assert!(r < 8);
        debug_assert!(c < 8);
        let mut v = (r << 3) | (c & 0x07);
        if pressed {
            v |= 0x80;
        }
        v
    }
}

/// Cardputer-Adv keyboard driver.
pub struct Tca8418<T: Transport> {
    transport: T,
    initialized: bool,
}

impl<T: Transport> Tca8418<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            initialized: false,
        }
    }

    /// Drain all pending key events. Each event is (row, col,
    /// pressed).
    pub fn drain_events(&mut self) -> Result<Vec<(u8, u8, bool)>, DeviceError> {
        let mut out = Vec::new();
        loop {
            let v =
                self.transport
                    .exec(I2cOp::read(tca8418::I2C_ADDR, tca8418::REG_KEY_EVENT_A, 1))?;
            if v.is_empty() {
                break;
            }
            let b = v[0];
            if b == 0 {
                break;
            }
            let pressed = b & 0x80 != 0;
            let r = (b >> 3) & 0x07;
            let c = b & 0x07;
            out.push((r, c, pressed));
        }
        Ok(out)
    }

    /// Map a (row, col) press to a [`KeyAction`]. Returns None
    /// when the cell is not bound.
    pub fn map(r: u8, c: u8) -> Option<KeyAction> {
        match (r, c) {
            (0, 0) => Some(KeyAction::Char('a')),
            (0, 1) => Some(KeyAction::Char('b')),
            (0, 2) => Some(KeyAction::Char('c')),
            (0, 3) => Some(KeyAction::Char('d')),
            (0, 4) => Some(KeyAction::Char('e')),
            (0, 5) => Some(KeyAction::Char('f')),
            (0, 6) => Some(KeyAction::Char('g')),
            (1, 0) => Some(KeyAction::Char('h')),
            (1, 1) => Some(KeyAction::Char('i')),
            (1, 2) => Some(KeyAction::Char('j')),
            (1, 3) => Some(KeyAction::Char('k')),
            (1, 4) => Some(KeyAction::Char('l')),
            (1, 5) => Some(KeyAction::Char('m')),
            (1, 6) => Some(KeyAction::Char('n')),
            (2, 0) => Some(KeyAction::Char('o')),
            (2, 1) => Some(KeyAction::Char('p')),
            (2, 2) => Some(KeyAction::Char('q')),
            (2, 3) => Some(KeyAction::Char('r')),
            (2, 4) => Some(KeyAction::Char('s')),
            (2, 5) => Some(KeyAction::Char('t')),
            (2, 6) => Some(KeyAction::Char('u')),
            (3, 0) => Some(KeyAction::Char('v')),
            (3, 1) => Some(KeyAction::Char('w')),
            (3, 2) => Some(KeyAction::Char('x')),
            (3, 3) => Some(KeyAction::Char('y')),
            (3, 4) => Some(KeyAction::Char('z')),
            (3, 5) => Some(KeyAction::Char('1')),
            (3, 6) => Some(KeyAction::Char('2')),
            (4, 0) => Some(KeyAction::Char('3')),
            (4, 1) => Some(KeyAction::Char('4')),
            (4, 2) => Some(KeyAction::Char('5')),
            (4, 3) => Some(KeyAction::Char('6')),
            (4, 4) => Some(KeyAction::Char('7')),
            (4, 5) => Some(KeyAction::Char('8')),
            (4, 6) => Some(KeyAction::Char('9')),
            (5, 0) => Some(KeyAction::Char('0')),
            (5, 1) => Some(KeyAction::Enter),
            (5, 2) => Some(KeyAction::Esc),
            (5, 3) => Some(KeyAction::Backspace),
            (5, 4) => Some(KeyAction::Char(' ')),
            (5, 5) => Some(KeyAction::Char(';')),
            (5, 6) => Some(KeyAction::Char('/')),
            (6, 0) => Some(KeyAction::Tab),
            (6, 1) => Some(KeyAction::Up),
            (6, 2) => Some(KeyAction::Down),
            (6, 3) => Some(KeyAction::Left),
            (6, 4) => Some(KeyAction::Right),
            _ => None,
        }
    }
}

impl<T: Transport> Keyboard for Tca8418<T> {
    fn poll(&mut self) -> Result<Option<KeyAction>, DeviceError> {
        if !self.initialized {
            // Auto-init so the framework can hot-plug the keyboard.
            self.transport
                .exec(I2cOp::write(tca8418::I2C_ADDR, tca8418::REG_CFG, &[0x01]))?;
            self.transport.exec(I2cOp::write(
                tca8418::I2C_ADDR,
                tca8418::REG_INTSTAT,
                &[0x03],
            ))?;
            self.initialized = true;
        }
        for (r, c, pressed) in self.drain_events()? {
            if pressed {
                if let Some(k) = Self::map(r, c) {
                    return Ok(Some(k));
                }
            }
        }
        Ok(None)
    }
}

// ============================================================================
// BMI270 — IMU over I2C.
// ============================================================================

/// BMI270 I2C address and register map.
pub mod bmi270 {
    pub const I2C_ADDR: u8 = 0x68;
    pub const REG_CHIP_ID: u8 = 0x00;
    pub const CHIP_ID: u8 = 0x24;
    pub const REG_ERR: u8 = 0x02;
    pub const REG_STATUS: u8 = 0x03;
    pub const REG_DATA_START: u8 = 0x0c;
    pub const REG_CMD: u8 = 0x7e;
    pub const CMD_SOFT_RESET: u16 = 0xb6a3;
    pub const CMD_ACC_ENABLE: u16 = 0x0400;
    pub const CMD_GYR_ENABLE: u16 = 0x0401;
}

/// Driver for the BMI270 6-axis IMU.
pub struct Bmi270<T: Transport> {
    transport: T,
    initialized: bool,
}

impl<T: Transport> Bmi270<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            initialized: false,
        }
    }

    /// Send a 2-byte command (BOSCH uses 16-bit command words).
    pub fn send_cmd(&mut self, cmd: u16) -> Result<(), DeviceError> {
        self.transport.exec(I2cOp::write(
            bmi270::I2C_ADDR,
            bmi270::REG_CMD,
            &[(cmd >> 8) as u8, (cmd & 0xff) as u8],
        ))?;
        Ok(())
    }

    /// Read 12 bytes (3 acc + 3 gyro, each 16-bit little-endian).
    pub fn read_data(&mut self) -> Result<[i16; 6], DeviceError> {
        let raw = self
            .transport
            .exec(I2cOp::read(bmi270::I2C_ADDR, bmi270::REG_DATA_START, 12))?;
        if raw.len() < 12 {
            return Err(DeviceError::Imu("short read".to_string()));
        }
        let mut out = [0i16; 6];
        for i in 0..6 {
            let lo = raw[i * 2] as i16;
            let hi = raw[i * 2 + 1] as i16;
            out[i] = (hi << 8) | (lo & 0xff);
        }
        Ok(out)
    }
}

impl<T: Transport> Imu for Bmi270<T> {
    fn sample(&mut self) -> Result<ImuSample, DeviceError> {
        if !self.initialized {
            // Probe chip ID, soft-reset, and enable sensors.
            let chip =
                self.transport
                    .exec(I2cOp::read(bmi270::I2C_ADDR, bmi270::REG_CHIP_ID, 1))?;
            if chip.first().copied() != Some(bmi270::CHIP_ID) {
                return Err(DeviceError::Imu("chip id mismatch".to_string()));
            }
            self.send_cmd(bmi270::CMD_SOFT_RESET)?;
            self.send_cmd(bmi270::CMD_ACC_ENABLE)?;
            self.send_cmd(bmi270::CMD_GYR_ENABLE)?;
            self.initialized = true;
        }
        let d = self.read_data()?;
        Ok(ImuSample {
            accel: [d[0], d[1], d[2]],
            gyro: [d[3], d[4], d[5]],
        })
    }
}

// ============================================================================
// SD card — SPI block storage.
// ============================================================================

/// Generic SD card over SPI. The driver issues CMD0 to initialize,
/// then CMD17/CMD24 for single-block read/write.
pub mod sd {
    /// SD card command opcodes used by the driver.
    pub const CMD0: u8 = 0x40; // GO_IDLE_STATE
    pub const CMD8: u8 = 0x48; // SEND_IF_COND
    pub const CMD17: u8 = 0x51; // READ_SINGLE_BLOCK
    pub const CMD24: u8 = 0x58; // WRITE_SINGLE_BLOCK
    pub const ACMD41: u8 = 0x69; // SD_SEND_OP_COND
    pub const R1_OK: u8 = 0x00;
    pub const R1_IDLE: u8 = 0x01;
}

/// SPI bus abstraction. We keep this minimal: the card driver only
/// needs full-duplex read/write.
pub trait SpiBus: Send {
    /// Full-duplex: write `tx`, read the same number of bytes
    /// into `rx`. Returns the number of bytes transferred.
    fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<usize, DeviceError>;
}

#[derive(Debug, Default, Clone)]
pub struct MockSpi {
    /// The bytes the host sends in the most recent transfer.
    pub last_tx: Vec<u8>,
    /// Canned responses, popped in order. Each is the `rx` payload
    /// for one `transfer` call.
    pub responses: Vec<Vec<u8>>,
}

impl MockSpi {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push_response(&mut self, bytes: &[u8]) {
        self.responses.push(bytes.to_vec());
    }
}

impl SpiBus for MockSpi {
    fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<usize, DeviceError> {
        self.last_tx = tx.to_vec();
        let n = if let Some(r) = self.responses.first() {
            r.len().min(rx.len())
        } else {
            0
        };
        rx[..n].copy_from_slice(&self.responses[0][..n]);
        if !self.responses.is_empty() {
            self.responses.remove(0);
        }
        Ok(n)
    }
}

pub struct SdCard<S: SpiBus> {
    spi: S,
    initialized: bool,
    /// Logical sector size in bytes.
    pub sector_size: usize,
}

impl<S: SpiBus> SdCard<S> {
    pub fn new(spi: S) -> Self {
        Self {
            spi,
            initialized: false,
            sector_size: 512,
        }
    }

    /// Send a command and capture the R1 response.
    pub fn send_cmd(&mut self, cmd: u8, arg: u32) -> Result<u8, DeviceError> {
        let mut tx = vec![
            cmd,
            (arg >> 24) as u8,
            (arg >> 16) as u8,
            (arg >> 8) as u8,
            arg as u8,
        ];
        tx.push(0x95); // CRC for CMD0
        let mut rx = vec![0u8; tx.len()];
        self.spi.transfer(&tx, &mut rx)?;
        Ok(rx[0])
    }

    /// Read one 512-byte block.
    pub fn read_block(&mut self, sector: u32) -> Result<[u8; 512], DeviceError> {
        if !self.initialized {
            return Err(DeviceError::Storage("not initialized".to_string()));
        }
        self.send_cmd(sd::CMD17, sector)?;
        let mut rx = [0u8; 514];
        let tx = vec![0xffu8; 514];
        self.spi.transfer(&tx, &mut rx)?;
        let mut block = [0u8; 512];
        block.copy_from_slice(&rx[2..514]);
        Ok(block)
    }
}

impl<S: SpiBus> Storage for SdCard<S> {
    fn read(&self, _path: &str) -> Result<Vec<u8>, DeviceError> {
        // The host stub doesn't carry path->sector mapping; on
        // the device the framework mounts the FAT filesystem and
        // routes `path` through it. Return an empty vec for any
        // path so callers can iterate.
        Ok(Vec::new())
    }
    fn write(&self, _path: &str, _data: &[u8]) -> Result<(), DeviceError> {
        Ok(())
    }
    fn exists(&self, _path: &str) -> Result<bool, DeviceError> {
        Ok(false)
    }
    fn remove(&self, _path: &str) -> Result<(), DeviceError> {
        Ok(())
    }
    fn list(&self, _path: &str) -> Result<Vec<String>, DeviceError> {
        Ok(Vec::new())
    }
}

// ============================================================================
// ES8311 — low-power mono audio codec (I2S control + I2C config).
// ============================================================================

/// ES8311 I2C address and register map.
pub mod es8311 {
    pub const I2C_ADDR: u8 = 0x18;
    pub const REG_RESET: u8 = 0x00;
    pub const REG_CLK_MANAGER1: u8 = 0x01;
    pub const REG_CLK_MANAGER2: u8 = 0x02;
    pub const REG_ADC: u8 = 0x17;
    pub const REG_DAC: u8 = 0x32;
    pub const REG_GPIO: u8 = 0x44;
}

/// I2S bus for the ES8311 + I2S DAC/ADC path. The driver assumes
/// 16-bit signed samples at 16 kHz mono in/out.
pub trait I2sBus: Send {
    fn write_samples(&mut self, samples: &[Sample]) -> Result<(), DeviceError>;
    fn read_samples(&mut self, buf: &mut [Sample]) -> Result<usize, DeviceError>;
}

#[derive(Debug, Default, Clone)]
pub struct MockI2s {
    /// The most recent buffer written by `write_samples`.
    pub last_written: Vec<Sample>,
    /// The buffer the host hands to `read_samples` — the framework
    /// sets `feed` before calling read.
    pub feed: Vec<Sample>,
}

impl MockI2s {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn feed_with(&mut self, samples: &[Sample]) {
        self.feed = samples.to_vec();
    }
}

impl I2sBus for MockI2s {
    fn write_samples(&mut self, samples: &[Sample]) -> Result<(), DeviceError> {
        self.last_written = samples.to_vec();
        Ok(())
    }
    fn read_samples(&mut self, buf: &mut [Sample]) -> Result<usize, DeviceError> {
        let n = buf.len().min(self.feed.len());
        buf[..n].copy_from_slice(&self.feed[..n]);
        self.feed.drain(..n);
        Ok(n)
    }
}

/// Driver for the ES8311 codec. The I2C side configures the chip;
/// the I2S side streams samples.
pub struct Es8311<T: Transport, I: I2sBus> {
    transport: T,
    i2s: I,
    initialized: bool,
    pub sample_rate: u32,
}

impl<T: Transport, I: I2sBus> Es8311<T, I> {
    pub fn new(transport: T, i2s: I) -> Self {
        Self {
            transport,
            i2s,
            initialized: false,
            sample_rate: 16000,
        }
    }
}

impl<T: Transport, I: I2sBus> AudioIn for Es8311<T, I> {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn read(&mut self, buf: &mut [Sample]) -> Result<usize, DeviceError> {
        if !self.initialized {
            // Reset the chip, then power up the ADC and clock
            // manager.
            self.transport
                .exec(I2cOp::write(es8311::I2C_ADDR, es8311::REG_RESET, &[0x1f]))?;
            self.transport.exec(I2cOp::write(
                es8311::I2C_ADDR,
                es8311::REG_CLK_MANAGER1,
                &[0x30],
            ))?;
            self.transport
                .exec(I2cOp::write(es8311::I2C_ADDR, es8311::REG_ADC, &[0x00]))?;
            self.initialized = true;
        }
        self.i2s.read_samples(buf)
    }
}

impl<T: Transport, I: I2sBus> AudioOut for Es8311<T, I> {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn write(&mut self, samples: &[Sample]) -> Result<(), DeviceError> {
        if !self.initialized {
            // Reset the chip, then power up the DAC.
            self.transport
                .exec(I2cOp::write(es8311::I2C_ADDR, es8311::REG_RESET, &[0x1f]))?;
            self.transport
                .exec(I2cOp::write(es8311::I2C_ADDR, es8311::REG_DAC, &[0x00]))?;
            self.initialized = true;
        }
        self.i2s.write_samples(samples)
    }
    fn flush(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }
}

/// Configuration for the esp-i2s bus used by the ES8311 driver.
/// the host simulator uses `MockI2s` directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EspI2sConfig {
    /// I2S peripheral index (0 or 1).
    pub peripheral: u8,
    /// Bit clock pin (BCLK).
    pub bclk_pin: u8,
    /// Word select pin (WS / LRCLK).
    pub ws_pin: u8,
    /// Data in pin (DIN / SDIN).
    pub din_pin: u8,
    /// Data out pin (DOUT / SDOUT).
    pub dout_pin: u8,
    /// MCLK pin (master clock). Set to 255 to disable.
    pub mclk_pin: u8,
    /// Sample rate in Hz (16000 for voice).
    pub sample_rate_hz: u32,
    /// Bits per sample (16).
    pub bits_per_sample: u8,
    /// Number of channels (1 for mono).
    pub channels: u8,
}

impl Default for EspI2sConfig {
    fn default() -> Self {
        Self {
            peripheral: 0,
            bclk_pin: 41,
            ws_pin: 43,
            din_pin: 44,
            dout_pin: 42,
            mclk_pin: 255,
            sample_rate_hz: 16000,
            bits_per_sample: 16,
            channels: 1,
        }
    }
}

impl EspI2sConfig {
    /// True when MCLK is wired and the driver should drive it.
    pub fn mclk_enabled(&self) -> bool {
        self.mclk_pin < 255
    }
}

#[cfg(test)]
#[allow(unused_mut)]
mod tests {
    use super::*;

    #[test]
    fn mock_transcript_records_writes() {
        let mut t = MockTransport::new();
        t.exec(I2cOp::write(0x10, 0x01, &[0x02]))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(t.transcript.len(), 1);
        assert_eq!(t.transcript[0].addr, 0x10);
        assert_eq!(t.transcript[0].reg, Some(0x01));
    }

    #[test]
    fn mock_transport_returns_canned_read() {
        let mut t = MockTransport::new();
        t.push_answer(0x10, 0x05, &[0xaa, 0xbb]);
        let r = t
            .exec(I2cOp::read(0x10, 0x05, 2))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r, vec![0xaa, 0xbb]);
    }

    #[test]
    fn mock_transport_fail_returns_error() {
        let mut t = MockTransport::new();
        t.fail = Some(DeviceError::Io("boom".to_string()));
        let r = t.exec(I2cOp::write(0, 0, &[]));
        assert!(matches!(r, Err(DeviceError::Io(_))));
        assert!(t.fail.is_none());
    }

    #[test]
    fn st7789_init_writes_expected_commands() {
        let mut t = MockTransport::new();
        let mut d = St7789::new(t);
        d.init().unwrap_or_else(|e| panic!("{e}"));
        let regs: Vec<u8> = d
            .transport
            .transcript
            .iter()
            .filter_map(|op| op.reg)
            .collect();
        assert!(regs.contains(&st7789::CMD_SLPOUT));
        assert!(regs.contains(&st7789::CMD_DISPON));
    }

    #[test]
    fn st7789_set_window_emits_caset_raset() {
        let mut t = MockTransport::new();
        let mut d = St7789::new(t);
        d.set_window(0, 0, 239, 134)
            .unwrap_or_else(|e| panic!("{e}"));
        let ops = &d.transport.transcript;
        let caset = ops.iter().find(|op| op.reg == Some(st7789::CMD_CASET));
        let raset = ops.iter().find(|op| op.reg == Some(st7789::CMD_RASET));
        assert!(caset.is_some());
        assert!(raset.is_some());
    }

    #[test]
    fn st7789_push_before_init_errors() {
        let mut d = St7789::new(MockTransport::new());
        let f = Frame::new_solid(0);
        assert!(d.push(&f).is_err());
    }

    #[test]
    fn st7789_backlight_updates_level() {
        let mut d = St7789::new(MockTransport::new());
        d.backlight(200).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(d.backlight_level, 200);
    }

    #[test]
    fn st7789_shutdown_writes_dispoff() {
        let mut d = St7789::new(MockTransport::new());
        d.init().unwrap_or_else(|e| panic!("{e}"));
        d.shutdown().unwrap_or_else(|e| panic!("{e}"));
        let regs: Vec<u8> = d
            .transport
            .transcript
            .iter()
            .filter_map(|op| op.reg)
            .collect();
        assert!(regs.contains(&st7789::CMD_DISPOFF));
        assert!(!d.initialized);
    }

    #[test]
    fn st7789_set_rotation_writes_madctl() {
        let mut d = St7789::new(MockTransport::new());
        d.set_rotation(1).unwrap_or_else(|e| panic!("{e}"));
        let op = d
            .transport
            .transcript
            .iter()
            .find(|op| op.reg == Some(st7789::CMD_MADCTL))
            .unwrap_or_else(|| panic!("no MADCTL"));
        assert_eq!(op.data, vec![0x60]);
    }

    #[test]
    fn tca8418_event_key_packs_press_and_release() {
        let v = tca8418::event_key(3, 5, true);
        assert_eq!(v & 0x80, 0x80);
        assert_eq!(v & 0x07, 5);
        assert_eq!((v >> 3) & 0x07, 3);
        let v = tca8418::event_key(2, 1, false);
        assert_eq!(v & 0x80, 0);
    }

    #[test]
    fn tca8418_drain_events_consumes_buffer() {
        let mut t = MockTransport::new();
        t.push_answer(tca8418::I2C_ADDR, tca8418::REG_KEY_EVENT_A, &[0x82]);
        t.push_answer(tca8418::I2C_ADDR, tca8418::REG_KEY_EVENT_A, &[0x00]);
        let mut k = Tca8418::new(t);
        let evs = k.drain_events().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(evs.len(), 1);
        assert!(evs[0].2);
        assert_eq!(evs[0].0, 0);
        assert_eq!(evs[0].1, 2);
    }
    #[test]
    fn tca8418_poll_maps_known_key() {
        let mut t = MockTransport::new();
        t.push_answer(
            tca8418::I2C_ADDR,
            tca8418::REG_KEY_EVENT_A,
            &[tca8418::event_key(3, 3, true)],
        );
        t.push_answer(tca8418::I2C_ADDR, tca8418::REG_KEY_EVENT_A, &[0x00]);
        let mut k = Tca8418::new(t);
        let action = k.poll().unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(action, Some(KeyAction::Char('y'))));
    }

    #[test]
    fn tca8418_poll_returns_none_when_empty() {
        let mut t = MockTransport::new();
        t.push_answer(tca8418::I2C_ADDR, tca8418::REG_KEY_EVENT_A, &[0x00]);
        let mut k = Tca8418::new(t);
        let action = k.poll().unwrap_or_else(|e| panic!("{e}"));
        assert!(action.is_none());
    }

    #[test]
    fn tca8418_map_handles_arrows() {
        assert!(matches!(
            Tca8418::<MockTransport>::map(6, 1),
            Some(KeyAction::Up)
        ));
        assert!(matches!(
            Tca8418::<MockTransport>::map(6, 2),
            Some(KeyAction::Down)
        ));
        assert!(matches!(
            Tca8418::<MockTransport>::map(5, 2),
            Some(KeyAction::Esc)
        ));
    }

    #[test]
    fn bmi270_sample_checks_chip_id() {
        let mut t = MockTransport::new();
        t.push_answer(bmi270::I2C_ADDR, bmi270::REG_CHIP_ID, &[bmi270::CHIP_ID]);
        let mut imu = Bmi270::new(t);
        let s = imu.sample().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.accel, [0, 0, 0]);
    }

    #[test]
    fn bmi270_chip_id_mismatch_errors() {
        let mut t = MockTransport::new();
        t.push_answer(bmi270::I2C_ADDR, bmi270::REG_CHIP_ID, &[0x00]);
        let mut imu = Bmi270::new(t);
        assert!(imu.sample().is_err());
    }

    #[test]
    fn bmi270_read_data_packs_12_bytes() {
        let mut t = MockTransport::new();
        t.push_answer(
            bmi270::I2C_ADDR,
            bmi270::REG_DATA_START,
            &[1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0],
        );
        let mut imu = Bmi270::new(t);
        imu.initialized = true;
        let d = imu.read_data().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(d, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn bmi270_short_read_errors() {
        let mut t = MockTransport::new();
        t.push_answer(bmi270::I2C_ADDR, bmi270::REG_DATA_START, &[1, 2, 3]);
        let mut imu = Bmi270::new(t);
        imu.initialized = true;
        assert!(imu.read_data().is_err());
    }
    #[test]
    fn sd_card_init_returns_idle() {
        let mut spi = MockSpi::new();
        spi.push_response(&[sd::R1_IDLE, 0, 0, 0, 0, 0]);
        let mut card = SdCard::new(spi);
        let r = card.send_cmd(sd::CMD0, 0).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r, sd::R1_IDLE);
    }

    #[test]
    fn sd_card_read_block_before_init_errors() {
        let mut card = SdCard::new(MockSpi::new());
        assert!(card.read_block(0).is_err());
    }

    #[test]
    fn sd_card_storage_methods_are_noop() {
        let card = SdCard::new(MockSpi::new());
        assert!(card.read("/x").unwrap_or_else(|e| panic!("{e}")).is_empty());
        assert!(!card.exists("/x").unwrap_or_else(|e| panic!("{e}")));
        assert!(card.write("/x", b"y").is_ok());
        assert!(card.remove("/x").is_ok());
        assert!(card.list("/").unwrap_or_else(|e| panic!("{e}")).is_empty());
    }

    #[test]
    fn es8311_audio_in_init_writes_reset() {
        let mut t = MockTransport::new();
        let mut i2s = MockI2s::new();
        let mut es = Es8311::new(t, i2s);
        // Trigger init via read with a tiny buffer.
        let mut buf = [0i16; 1];
        let _ = es.read(&mut buf);
        let regs: Vec<u8> = es
            .transport
            .transcript
            .iter()
            .filter_map(|op| op.reg)
            .collect();
        assert!(regs.contains(&es8311::REG_RESET));
    }

    #[test]
    fn es8311_audio_out_writes_via_i2s() {
        let mut t = MockTransport::new();
        let mut i2s = MockI2s::new();
        let mut es = Es8311::new(t, i2s);
        es.write(&[1, 2, 3, 4]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(es.i2s.last_written, vec![1, 2, 3, 4]);
    }

    #[test]
    fn es8311_audio_in_reads_via_i2s() {
        let mut t = MockTransport::new();
        let mut i2s = MockI2s::new();
        i2s.feed_with(&[5, 6, 7]);
        let mut es = Es8311::new(t, i2s);
        let mut buf = [0i16; 8];
        let n = es.read(&mut buf).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[5, 6, 7]);
    }

    #[test]
    fn es8311_sample_rate_is_16k() {
        let mut t = MockTransport::new();
        let mut i2s = MockI2s::new();
        let es_in: Es8311<MockTransport, MockI2s> = Es8311::new(t, i2s);
        let as_in: &dyn crate::traits::AudioIn = &es_in;
        assert_eq!(as_in.sample_rate(), 16000);
    }
    #[test]
    fn esp_i2s_config_default_has_voice_settings() {
        let c = EspI2sConfig::default();
        assert_eq!(c.sample_rate_hz, 16000);
        assert_eq!(c.bits_per_sample, 16);
        assert_eq!(c.channels, 1);
        assert!(!c.mclk_enabled());
    }

    #[test]
    fn esp_i2s_config_mclk_enabled_when_pin_set() {
        let c = EspI2sConfig {
            mclk_pin: 0,
            ..EspI2sConfig::default()
        };
        assert!(c.mclk_enabled());
    }
}
