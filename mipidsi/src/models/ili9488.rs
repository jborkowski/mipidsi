use display_interface::{DataFormat, WriteOnlyDataCommand};
use embedded_graphics_core::{
    pixelcolor::{Rgb565, Rgb666},
    prelude::{IntoStorage, RgbColor},
};
use embedded_hal::{blocking::delay::DelayUs, digital::v2::OutputPin};

use crate::{
    dcs::{
        BitsPerPixel, Dcs, EnterNormalMode, ExitSleepMode, PixelFormat, SetAddressMode,
        SetDisplayOn, SetInvertMode, SetPixelFormat, SoftReset, WriteMemoryStart,
    },
    error::InitError,
    Builder, Error, ModelOptions,
};

use super::Model;

/// ILI9488 display in Rgb565 color mode.
pub struct ILI9488Rgb565;

/// ILI9488 display in Rgb666 color mode.
pub struct ILI9488Rgb666;

impl Model for ILI9488Rgb565 {
    type ColorFormat = Rgb565;

    fn init<RST, DELAY, DI>(
        &mut self,
        dcs: &mut Dcs<DI>,
        delay: &mut DELAY,
        options: &ModelOptions,
        rst: &mut Option<RST>,
    ) -> Result<SetAddressMode, InitError<RST::Error>>
    where
        RST: OutputPin,
        DELAY: DelayUs<u32>,
        DI: WriteOnlyDataCommand,
    {
        match rst {
            Some(ref mut rst) => self.hard_reset(rst, delay)?,
            None => dcs.write_command(SoftReset)?,
        }
        delay.delay_us(120_000);

        let pf = PixelFormat::with_all(BitsPerPixel::from_rgb_color::<Self::ColorFormat>());
        Ok(init_common(dcs, delay, options, pf)?)
    }

    fn write_pixels<DI, I>(&mut self, dcs: &mut Dcs<DI>, colors: I) -> Result<(), Error>
    where
        DI: WriteOnlyDataCommand,
        I: IntoIterator<Item = Self::ColorFormat>,
    {
        dcs.write_command(WriteMemoryStart)?;
        let mut iter = colors.into_iter().map(|c| c.into_storage());

        let buf = DataFormat::U16BEIter(&mut iter);
        dcs.di.send_data(buf)
    }

    fn default_options() -> ModelOptions {
        ModelOptions::with_sizes((320, 480), (320, 480))
    }
}

impl Model for ILI9488Rgb666 {
    type ColorFormat = Rgb666;

    fn init<RST, DELAY, DI>(
        &mut self,
        dcs: &mut Dcs<DI>,
        delay: &mut DELAY,
        options: &ModelOptions,
        rst: &mut Option<RST>,
    ) -> Result<SetAddressMode, InitError<RST::Error>>
    where
        RST: OutputPin,
        DELAY: DelayUs<u32>,
        DI: WriteOnlyDataCommand,
    {
        match rst {
            Some(ref mut rst) => self.hard_reset(rst, delay)?,
            None => dcs.write_command(SoftReset)?,
        };

        delay.delay_us(120_000);

        let pf = PixelFormat::with_all(BitsPerPixel::from_rgb_color::<Self::ColorFormat>());
        Ok(init_common(dcs, delay, options, pf)?)
    }

    fn write_pixels<DI, I>(&mut self, dcs: &mut Dcs<DI>, colors: I) -> Result<(), Error>
    where
        DI: WriteOnlyDataCommand,
        I: IntoIterator<Item = Self::ColorFormat>,
    {
        dcs.write_command(WriteMemoryStart)?;
        let mut iter = colors.into_iter().flat_map(|c| {
            let red = c.r() << 2;
            let green = c.g() << 2;
            let blue = c.b() << 2;
            [red, green, blue]
        });

        let buf = DataFormat::U8Iter(&mut iter);
        dcs.di.send_data(buf)
    }

    fn default_options() -> ModelOptions {
        ModelOptions::with_sizes((320, 480), (320, 480))
    }
}

// simplified constructor for Display

impl<DI> Builder<DI, ILI9488Rgb565>
where
    DI: WriteOnlyDataCommand,
{
    /// Creates a new display builder for an ILI9488 display in Rgb565 color mode.
    ///
    /// The default framebuffer size and display size is 320x480 pixels.
    ///
    /// # Limitations
    ///
    /// The Rgb565 color mode is not supported for displays with SPI connection.
    ///
    /// # Arguments
    ///
    /// * `di` - a [display interface](WriteOnlyDataCommand) for communicating with the display
    ///
    pub fn ili9488_rgb565(di: DI) -> Self {
        Self::with_model(di, ILI9488Rgb565)
    }
}

impl<DI> Builder<DI, ILI9488Rgb666>
where
    DI: WriteOnlyDataCommand,
{
    /// Creates a new display builder for ILI9488 displays in Rgb666 color mode.
    ///
    /// The default framebuffer size and display size is 320x480 pixels.
    ///
    /// # Arguments
    ///
    /// * `di` - a [display interface](WriteOnlyDataCommand) for communicating with the display
    ///
    pub fn ili9488_rgb666(di: DI) -> Self {
        Self::with_model(di, ILI9488Rgb666)
    }
}

// common init for all color format models
fn init_common<DELAY, DI>(
    dcs: &mut Dcs<DI>,
    delay: &mut DELAY,
    options: &ModelOptions,
    pixel_format: PixelFormat,
) -> Result<SetAddressMode, Error>
where
    DELAY: DelayUs<u32>,
    DI: WriteOnlyDataCommand,
{
    let madctl = SetAddressMode::from(options);
    dcs.write_command(madctl)?; // left -> right, bottom -> top RGB

    dcs.write_command(SetInvertMode(options.invert_colors))?;

    dcs.write_command(SetPixelFormat::new(pixel_format))?; // pixel format
    dcs.write_raw(Instruction::PWCTR1 as u8, &[0x17, 0x15])?;
    dcs.write_raw(Instruction::PWCTR2 as u8, &[0x41])?;

    dcs.write_raw(Instruction::VMCTR1 as u8, &[0x00, 0x12, 0x80])?;
    dcs.write_raw(Instruction::FRMCTR1 as u8, &[0xA0])?;
    dcs.write_raw(Instruction::SIMFUNC as u8, &[0x00])?; // set image function

    // gamma setup

    dcs.write_raw(
        Instruction::GMCTRP1 as u8,
        &[
            0x00, 0x03, 0x09, 0x08, 0x16, 0x0A, 0x3F, 0x78, 0x4C, 0x09, 0x0A, 0x08, 0x16, 0x1A,
            0x0F,
        ],
    )?;
    dcs.write_raw(
        Instruction::GMCTRN1 as u8,
        &[
            0x00, 0x0E, 0x14, 0x03, 0x11, 0x07, 0x31, 0xC1, 0x48, 0x08, 0x0F, 0x0C, 0x31, 0x36,
            0x0F,
        ],
    )?;

    dcs.write_raw(Instruction::DFUNCTR as u8, &[0x02, 0x02, 0x3B])?; // DFC
    dcs.write_command(ExitSleepMode)?; // turn off sleep
    dcs.write_command(EnterNormalMode)?; // turn to normal mode

    // DISPON requires some time otherwise we risk SPI data issues
    delay.delay_us(120_000);
    dcs.write_command(SetDisplayOn)?; // turn on display

    Ok(madctl)
}

enum Instruction {
    GMCTRP1 = 0xE0, // Positive gamma correction
    GMCTRN1 = 0xE1, // Negative gamma correction
    PWCTR1 = 0xC0,  // Power control 1
    PWCTR2 = 0xC1,  // Power control 2
    VMCTR1 = 0xC5,  // VCOM control 1
    FRMCTR1 = 0xB1, // Frame rate control (In normal mode/full colors)
    DFUNCTR = 0xB6, // Display function control
    SIMFUNC = 0xE9, // Driver timing control A
}
