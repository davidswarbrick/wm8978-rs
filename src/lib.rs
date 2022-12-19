#![cfg_attr(not(test), no_std)]

use embedded_hal::i2c::I2c;

const ADDR: u8 = 0;
#[cfg(feature = "register-buffer")]
static WM8978_DEFAULT_REG: [u16; 58] = [
    0x0000, 0x0000, 0x0000, 0x0000, 0x0050, 0x0000, 0x0140, 0x0000, 0x0000, 0x0000, 0x0000, 0x00FF,
    0x00FF, 0x0000, 0x0100, 0x00FF, 0x00FF, 0x0000, 0x012C, 0x002C, 0x002C, 0x002C, 0x002C, 0x0000,
    0x0032, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0038, 0x000B, 0x0032, 0x0000,
    0x0008, 0x000C, 0x0093, 0x00E9, 0x0000, 0x0000, 0x0000, 0x0000, 0x0003, 0x0010, 0x0010, 0x0100,
    0x0100, 0x0002, 0x0001, 0x0001, 0x0039, 0x0039, 0x0039, 0x0039, 0x0001, 0x0001,
];

/// Error type combining SPI, I2C, and Pin errors
/// You can remove anything you don't need / add anything you do
/// (as well as additional driver-specific values) here
#[derive(Debug, Clone, PartialEq)]
pub enum DriverError {
    /// Underlying I2C device error
    I2c,
    OutOfBounds,
    /// Device failed to resume from reset
    ResetTimeout,
}

pub enum Eq {
    One,
    Two,
    Three,
    Four,
    Five,
}

pub enum SampleRate {
    FortyEightkHz,
    ThirtyTwokHz,
    TwentyFourkHz,
    SixteenkHz,
    TwelvekHz,
    EightkHz,
}

pub enum I2SWordLength {
    SixteenBits,
    TwentyBits,
    TwentyFourBits,
    ThirtyTwoBits,
}

pub enum DataFormat {
    RightJustified,
    LeftJustified,
    I2S,
    DspPCM,
}

pub struct Wm8978Driver<I2C, Mode>
where
    I2C: I2c,
{
    i2c: I2C,
    registers: [u16; 58],
    _mode: Mode,
}

impl<I2C, Mode> Wm8978Driver<I2C, Mode>
where
    I2C: I2c,
{
    #[cfg(feature = "register-buffer")]
    fn read_reg(&self, reg: usize) -> Result<&u16, DriverError> {
        self.registers.get(reg).ok_or(DriverError::OutOfBounds)
    }
    #[cfg(feature = "register-buffer")]
    fn write_reg(&mut self, reg: usize, val: u16) -> Result<(), DriverError> {
        let r = self
            .registers
            .get_mut(reg)
            .ok_or(DriverError::OutOfBounds)?;
        *r = val;
        Ok(())
        // ToDo: Implement i2c write, using trait.
    }

    fn init(&mut self) -> Result<(), DriverError> {
        // ToDo: Configure I2C correctly
        self.write_reg(0, 0)?; // WM8978 Reset
        self.write_reg(1, 0x9b)?; // R1, OUT4MIXEN, MICEN (MIC), BIASEN, VMIDSEL[1:0]
        self.write_reg(2, 0x1b0)?; // R2, ROUT1, LOUT1, BOOSTENR, BOOSTENL
        self.write_reg(3, 0x16c)?; // R3, OUT4EN, LOUT2EN, ROUT2EN, RMIXEN, LMIXEN
        self.write_reg(6, 0)?; // R6, MCLK
        self.write_reg(43, 1 << 4)?; // R43, INVROUT2
        self.write_reg(47, 1 << 8)?; // R47, PGABOOSTL, MIC
        self.write_reg(48, 1 << 8)?; // R48, PGABOOSTR, MIC
        self.write_reg(49, 1 << 1)?; // R49, TSDEN
        self.write_reg(10, 1 << 3)?; // R10, DACOSR
        self.write_reg(14, 1 << 3)?; // R14, ADCOSR
        Ok(())
    }

    pub fn set_adc_dac(&mut self, dac: bool, adc: bool) -> Result<(), DriverError> {
        let reg_val = self.read_reg(3).unwrap();
        let new_val: u16 = match dac {
            true => *reg_val | 3,
            false => *reg_val & !(3),
        };
        self.write_reg(3, new_val)?;

        let reg_val = self.read_reg(2).unwrap();
        let new_val: u16 = match adc {
            true => *reg_val | 3,
            false => *reg_val & !(3),
        };
        self.write_reg(2, new_val)?;
        Ok(())
    }

    pub fn set_mic_gain(&mut self, gain: u8) -> Result<(), DriverError> {
        // WM8978 MIC (BOOST 20dB,MIC-->ADC)
        // gain: 0~63, -12dB~35.25dB, 0.75dB/Step
        self.write_reg(45, (gain & 0x3F) as u16)?;
        // Write same gain to L/R, with 1 in bit 8 to perform volume update
        self.write_reg(46, (gain & 0x3F) as u16 | 1 << 8)?;
        Ok(())
    }

    pub fn set_linein_gain(&mut self, gain: u8) -> Result<(), DriverError> {
        // WM8978 L2/R2 (Line In) (L2/R2-->ADC)
        // gain: 0~7, 0ֹ, 1~7, -12dB~6dB, 3dB/Step
        // Get current R47 values, clear volume at bit 4 onwards
        let regval: u16 = *self.read_reg(47).unwrap() & !(7 << 4);
        // Set volume to `gain`.
        self.write_reg(47, regval | ((gain & 7) << 4) as u16)?;

        // Get current R48 values, clear volume
        let regval: u16 = *self.read_reg(48).unwrap() & !(7 << 4);
        // Set volume to `gain`.
        self.write_reg(48, regval | ((gain & 7) << 4) as u16)?;
        Ok(())
    }

    pub fn set_aux_gain(&mut self, gain: u8) -> Result<(), DriverError> {
        // WM8978 AUXR, AUXL(PWM) (AUXR/L-->ADC)
        // gain:0~7, 0ֹ, 1~7, -12dB~6dB, 3dB/Step
        // Get current R47 values, clear volume
        let regval: u16 = *self.read_reg(47).unwrap() & !(7);
        // Set volume to `gain`.
        self.write_reg(47, regval | (gain & 7) as u16)?;

        // Get current R48 values, clear volume
        let regval: u16 = *self.read_reg(48).unwrap() & !(7);
        // Set volume to `gain`.
        self.write_reg(48, regval | (gain & 7) as u16)?;
        Ok(())
    }

    pub fn set_inputs(&mut self, mic: bool, linein: bool, aux: bool) -> Result<(), DriverError> {
        // R2 INPPGAENR, INPPGAENL (MIC/PGA)
        let reg_val = self.read_reg(2).unwrap();
        let new_val: u16 = match mic {
            true => *reg_val | 3 << 2,
            false => *reg_val & !(3 << 2),
        };
        self.write_reg(2, new_val)?;

        // R44 LIN2INPPGA, LIP2INPGA, RIN2INPPGA, RIP2INPGA
        let reg_val = self.read_reg(44).unwrap();
        let new_val: u16 = match mic {
            true => *reg_val | 3 << 4 | 3 << 0,
            false => *reg_val & !(3 << 4 | 3 << 0),
        };
        self.write_reg(44, new_val)?;

        match linein {
            true => self.set_linein_gain(5)?, // 0dB
            false => self.set_linein_gain(0)?,
        };
        match aux {
            true => self.set_aux_gain(7)?, // 6dB
            false => self.set_aux_gain(0)?,
        };
        Ok(())
    }

    pub fn set_eq(&mut self, eq: Eq, cfreq: u8, gain: u8) -> Result<(), DriverError> {
        let mut reg_val = 0;
        if gain < 24 {
            reg_val |= 24 - gain as u16;
        }
        reg_val |= (cfreq as u16 & 3) << 5;
        match eq {
            Eq::One => self.write_reg(18, reg_val & 0x100),
            Eq::Two => self.write_reg(19, reg_val),
            Eq::Three => self.write_reg(20, reg_val),
            Eq::Four => self.write_reg(21, reg_val),
            Eq::Five => self.write_reg(22, reg_val),
        }
    }

    pub fn set_outputs(&mut self, dac: bool, bypass: bool) -> Result<(), DriverError> {
        let mut reg_val = match dac {
            true => 1,
            false => 0,
        };
        if bypass {
            reg_val |= 1 << 1; // BYPASS
            reg_val |= 5 << 2; // 0dB
        }
        self.write_reg(50, reg_val)?;
        self.write_reg(51, reg_val)
    }

    pub fn set_volume_headphone(&mut self, vol_l: u8, vol_r: u8) -> Result<(), DriverError> {
        let reg_l = match vol_l {
            0 => 1 << 6,
            _ => vol_l & 0x3F,
        };
        let reg_r = match vol_r {
            0 => 1 << 6,
            _ => vol_r & 0x3F,
        };
        self.write_reg(52, reg_l as u16)?;
        self.write_reg(53, (reg_r as u16) | 1 << 8) // HPVU = 1
    }
    pub fn set_volume_speaker(&mut self, vol: u8) -> Result<(), DriverError> {
        let reg_val = match vol {
            0 => 1 << 6,
            _ => vol & 0x3F,
        };
        self.write_reg(54, reg_val as u16)?;
        self.write_reg(55, (reg_val as u16) | 1 << 8) // SPKVU = 1
    }
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) -> Result<(), DriverError> {
        let reg_val = match sample_rate {
            SampleRate::FortyEightkHz => 0x0,
            SampleRate::ThirtyTwokHz => 0x2,
            SampleRate::TwentyFourkHz => 0x4,
            SampleRate::SixteenkHz => 0x6,
            SampleRate::TwelvekHz => 0x8,
            SampleRate::EightkHz => 0xA,
        };
        self.write_reg(7, reg_val)
    }
    pub fn set_i2s_configuration(
        &mut self,
        format: DataFormat,
        len: I2SWordLength,
    ) -> Result<(), DriverError> {
        let reg_len = match len {
            I2SWordLength::SixteenBits => 0x0,
            I2SWordLength::TwentyBits => 0x1,
            I2SWordLength::TwentyFourBits => 0x10,
            I2SWordLength::ThirtyTwoBits => 0x11,
        };
        let reg_fmt = match format {
            DataFormat::RightJustified => 0x0,
            DataFormat::LeftJustified => 0x1,
            DataFormat::I2S => 0x10,
            DataFormat::DspPCM => 0x11,
        };
        self.write_reg(4, (reg_fmt << 3) | (reg_len << 5))
    }
}

#[cfg(test)]
mod wm8978drivertest {
    use super::*;
    // use embedded_hal::i2c::{I2c, ErrorType};
    // struct I2cmock;
    // impl I2c for I2cmock {}
    // impl ErrorType for I2cmock {
    //     type Error = u8;
    // }

    // fn init_drv() -> Wm8978Driver<I2cmock,bool> {
    //     let i2c;
    //     Wm8978Driver {
    //         i2c: i2c,
    //         registers: WM8978_DEFAULT_REG,
    //         _mode: false,
    //     }
    // }
    // #[test]
    // fn test_read_good_register() {
    //     let a = init_drv();
    //     assert_eq!(a.read_reg(0).unwrap(), &0);
    //     assert_eq!(a.read_reg(57).unwrap(), &1);
    // }
    // #[test]
    // #[should_panic]
    // fn test_read_bad_register() {
    //     let a = init_drv();
    //     a.read_reg(58).unwrap();
    // }
    // #[test]
    // fn test_write_register() {
    //     let mut a = init_drv();
    //     a.write_reg(0,2).unwrap();
    //     assert_eq!(a.read_reg(0).unwrap(), &2);
    // }
}
