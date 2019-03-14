use std::any::Any;
use std::fs::File;
use std::io::Result;

use i2c_linux::{Message, ReadFlags, WriteFlags};

type I2c = i2c_linux::I2c<File>;

use self::Command::*;

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
                        data: &data[i..i],
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
        Ok((1608.420446 - f64::from(vol[0])) / 170.423497)
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
}

impl SendCommand for I2c {}

impl BoardConfig for I2c {}
