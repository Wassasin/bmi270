#[cfg(test)]
mod test;

pub mod i2c;

device_driver::create_device!(
    device_name: Device,
    manifest: "src/ll/ll.yaml"
);

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
