use std::any::Any;
use std::fs::File;
use std::io::Result;
use std::path::Path;

use i2c_linux::{Message, ReadFlags, WriteFlags};

use self::Command::*;

pub type I2c = i2c_linux::I2c<File>;

#[allow(non_camel_case_types)]
pub enum Command {
    SET_PIC_FLASH_POINTER = 0x01,
    SEND_DATA_TO_IIC = 0x02,
    READ_DATA_FROM_IIC = 0x03,
    ERASE_IIC_FLASH = 0x04,
    WRITE_DATA_INTO_PIC = 0x05,
    JUMP_FROM_LOADER_TO_APP = 0x06,
    RESET_PIC = 0x07,
    GET_PIC_FLASH_POINTER = 0x08,
    ERASE_PIC_APP_PROGRAM = 0x09,
    SET_VOLTAGE = 0x10,
    SET_VOLTAGE_TIME = 0x11,
    SET_HASH_BOARD_ID = 0x12,
    GET_HASH_BOARD_ID = 0x13,
    SET_HOST_MAC_ADDRESS = 0x14,
    ENABLE_VOLTAGE = 0x15,
    SEND_HEART_BEAT = 0x16,
    GET_PIC_SOFTWARE_VERSION = 0x17,
    GET_VOLTAGE = 0x18,
    GET_DATE = 0x19,
    GET_WHICH_MAC = 0x20,
    GET_MAC = 0x21,
    WR_TEMP_OFFSET_VALUE = 0x22,
    RD_TEMP_OFFSET_VALUE = 0x23,
}

pub fn open<T: AsRef<Path>>(path: T) -> I2c {
    let device = File::open(&path)
        .unwrap_or_else(|_| panic!(format!("can't open {}!", path.as_ref().display())));

    I2c::new(device)
}

pub trait SendCommand: Any + Sized {
    fn send_command(
        &mut self,
        addr: u16,
        cmd: Command,
        data: Option<&mut [u8]>,
        read: bool,
    ) -> Result<()> {
        let data_len = data.as_ref().map_or(0, |x| x.len());
        let mut massages = Vec::with_capacity(3 + data_len);

        let command = [[0x55], [0xaa], [cmd as u8]];
        for b in &command {
            massages.push(Message::Write {
                address: addr,
                data: b,
                flags: WriteFlags::default(),
            });
        }

        if let Some(data) = data {
            for i in 0..data_len {
                if read {
                    massages.push(Message::Read {
                        address: addr,
                        data: unsafe {
                            // read data byte by byte
                            core::slice::from_raw_parts_mut((&data[i] as *const u8) as *mut u8, 1)
                        },
                        flags: ReadFlags::default(),
                    });
                } else {
                    massages.push(Message::Write {
                        address: addr,
                        // write data byte by byte
                        data: &data[i..=i],
                        flags: WriteFlags::default(),
                    })
                }
            }
        }

        match Any::downcast_mut::<I2c>(self) {
            Some(i2c) => i2c.i2c_transfer(&mut massages),
            None => unreachable!(),
        }
    }
}

#[allow(clippy::unreadable_literal)]
pub trait BoardConfig: SendCommand {
    fn jump_to_app(&mut self, addr: u16) -> Result<()> {
        self.send_command(addr, JUMP_FROM_LOADER_TO_APP, None, false)
    }

    fn set_voltage(&mut self, addr: u16, vol: f64) -> Result<()> {
        let vol = (1608.420446 - 170.423497 * vol) as u8;
        self.send_command(addr, SET_VOLTAGE, Some(&mut [vol]), false)
    }

    fn get_voltage(&mut self, addr: u16) -> Result<f64> {
        let mut vol = [0];
        self.send_command(addr, GET_VOLTAGE, Some(&mut vol), true)?;
        let vol = ((1608.4204 - f64::from(vol[0])) / 17.04235).trunc() / 10.0;
        Ok(vol)
    }

    fn enable_voltage(&mut self, addr: u16) -> Result<()> {
        self.send_command(addr, ENABLE_VOLTAGE, Some(&mut [1]), false)
    }

    fn disable_voltage(&mut self, addr: u16) -> Result<()> {
        self.send_command(addr, ENABLE_VOLTAGE, Some(&mut [0]), false)
    }

    fn reset_pic(&mut self, addr: u16) -> Result<()> {
        self.send_command(addr, RESET_PIC, None, false)
    }

    fn get_software_version(&mut self, addr: u16) -> Result<u8> {
        let mut ver = [0];
        self.send_command(addr, GET_PIC_SOFTWARE_VERSION, Some(&mut ver), true)?;
        Ok(ver[0])
    }

    fn set_flash_pointer(&mut self, addr: u16, fptr: u16) -> Result<()> {
        self.send_command(
            addr,
            SET_PIC_FLASH_POINTER,
            Some(&mut fptr.to_be_bytes()),
            false,
        )
    }

    fn get_flash_pointer(&mut self, addr: u16) -> Result<u16> {
        let mut fptr = [0; 2];
        self.send_command(addr, GET_PIC_FLASH_POINTER, Some(&mut fptr), true)?;
        Ok(u16::from_be_bytes(fptr))
    }

    fn read_data_from_flash(&mut self, addr: u16) -> Result<[u8; 16]> {
        let mut data = [0; 16];
        self.send_command(addr, READ_DATA_FROM_IIC, Some(&mut data), true)?;
        Ok(data)
    }

    fn send_heart_beat(&mut self, addr: u16) -> Result<()> {
        self.send_command(addr, SEND_HEART_BEAT, None, false)
    }

    fn get_temp_offset(&mut self, addr: u16) -> Result<u64> {
        let mut offset = [0; 8];
        self.send_command(addr, RD_TEMP_OFFSET_VALUE, Some(&mut offset), true)?;
        Ok(u64::from_be_bytes(offset))
    }

    fn set_temp_offset(&mut self, addr: u16, offset: u64) -> Result<()> {
        self.send_command(
            addr,
            WR_TEMP_OFFSET_VALUE,
            Some(&mut offset.to_be_bytes()),
            false,
        )
    }
}

impl SendCommand for I2c {}

impl BoardConfig for I2c {}
