pub trait BlockDevice {
    type Error;

    fn read(&self, lba: u32, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn write(&self, lba: u32, offset: usize, buf: &[u8]) -> Result<usize, Self::Error>;
    fn flush(&self) -> Result<(), Self::Error>;
    fn count(&self) -> Result<u32, Self::Error>;
    fn lba_size(&self) -> Result<usize, Self::Error>;
}