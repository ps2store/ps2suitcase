pub trait BinWriter {
    fn write(&self) -> std::io::Result<Vec<u8>>;
}

pub trait BinReader<T> {
    fn read(data: &[u8]) -> std::io::Result<T>;
}