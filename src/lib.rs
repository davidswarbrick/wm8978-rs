// use embedded_hal::i2c;
// #[cfg(not(test))] no_std

static WM8978_DEFAULT_REG: [u16; 58] = [0x0000, 0x0000, 0x0000, 0x0000, 0x0050, 0x0000, 0x0140, 0x0000, 0x0000, 0x0000,
                0x0000, 0x00FF, 0x00FF, 0x0000, 0x0100, 0x00FF, 0x00FF, 0x0000, 0x012C, 0x002C,
                0x002C, 0x002C, 0x002C, 0x0000, 0x0032, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
                0x0000, 0x0000, 0x0038, 0x000B, 0x0032, 0x0000, 0x0008, 0x000C, 0x0093, 0x00E9,
                0x0000, 0x0000, 0x0000, 0x0000, 0x0003, 0x0010, 0x0010, 0x0100, 0x0100, 0x0002,
                0x0001, 0x0001, 0x0039, 0x0039, 0x0039, 0x0039, 0x0001, 0x0001,];

pub struct Wm8978Driver<I2C, Mode>
// where
// I2C: i2c::Write + i2c::Read,
{
    i2c: I2C,
    registers: [u16; 58],
    _mode: Mode,
}

impl<I2C, Mode> Wm8978Driver<I2C, Mode> {
    // ToDo: raise appropriate error rather than Option
    pub fn read_reg(&self, reg: usize) -> Option<&u16> {
        self.registers.get(reg)
    }
    pub fn write_reg(&self, reg: usize, val: u16) {
        // ToDo: Implement i2c write, using trait.
    }
}

#[cfg(test)]
mod wm8978drivertest {
    use super::*;
    fn init_drv() -> Wm8978Driver<u8,bool> {
        Wm8978Driver {
            i2c: 0,
            registers: WM8978_DEFAULT_REG,
            _mode: false,
        }
    }
    #[test]
    fn test_read_good_register() {
        let a = init_drv();
        assert_eq!(a.read_reg(0).unwrap(), &0);
        assert_eq!(a.read_reg(57).unwrap(), &1);
    }
    #[test]
    #[should_panic]
    fn test_read_bad_register() {
        let a = init_drv();
        a.read_reg(58).unwrap();
    }
}
