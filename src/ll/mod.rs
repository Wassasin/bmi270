use device_driver::AsyncRegisterInterface;
use embedded_hal_async::i2c::I2c;

const I2C_ADDRESS: u8 = 0x68;
const MAX_WRITE_SIZE: usize = 513;

#[derive(Debug)]
pub struct DeviceInterface<I2C: I2c> {
    i2c: I2C,
}

device_driver::create_device!(
    device_name: Device,
    manifest: "src/ll/ll.yaml"
);

impl<I2C: I2c> DeviceInterface<I2C> {
    /// Construct a new instance of the device.
    ///
    /// I2C max frequency 400kHz.
    pub const fn new(i2c: I2C) -> Self {
        Self { i2c }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum DeviceError<T> {
    Interface(T),
    BufferTooSmall,
}

impl<T> From<T> for DeviceError<T> {
    fn from(value: T) -> Self {
        DeviceError::Interface(value)
    }
}

impl<I2C: I2c> device_driver::AsyncRegisterInterface for DeviceInterface<I2C> {
    type Error = DeviceError<I2C::Error>;

    type AddressType = u8;

    async fn write_register(
        &mut self,
        address: Self::AddressType,
        _size_bits: u32,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        let mut vec = heapless::Vec::<u8, MAX_WRITE_SIZE>::new();
        vec.push(address).map_err(|_| DeviceError::BufferTooSmall)?;
        vec.extend_from_slice(data)
            .map_err(|_| DeviceError::BufferTooSmall)?;
        Ok(self.i2c.write(I2C_ADDRESS, &vec).await?)
    }

    async fn read_register(
        &mut self,
        address: Self::AddressType,
        _size_bits: u32,
        data: &mut [u8],
    ) -> Result<(), Self::Error> {
        Ok(self.i2c.write_read(I2C_ADDRESS, &[address], data).await?)
    }
}

impl<I2C: I2c> device_driver::BufferInterfaceError for DeviceInterface<I2C> {
    type Error = DeviceError<I2C::Error>;
}

impl<I2C: I2c> device_driver::AsyncBufferInterface for DeviceInterface<I2C> {
    type AddressType = u8;

    async fn write(
        &mut self,
        address: Self::AddressType,
        buf: &[u8],
    ) -> Result<usize, Self::Error> {
        self.write_register(address, buf.len() as u32 * 8, buf)
            .await?;
        Ok(buf.len())
    }

    async fn flush(&mut self, address: Self::AddressType) -> Result<(), Self::Error> {
        // No-op
        let _ = address;
        Ok(())
    }

    #[allow(unused)]
    async fn read(
        &mut self,
        address: Self::AddressType,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        unimplemented!()
    }
}
