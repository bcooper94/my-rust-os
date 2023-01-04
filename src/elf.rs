use core::fmt::Debug;

pub mod elf32;
pub mod elf64;

#[derive(Debug, PartialEq)]
pub enum ElfParseError {
    NotValidElfFile,
    InvalidClass,
    WrongElfClass,
    InvalidEndianness,
    InvalidElfType,
    InvalidInstructionSetValue,

    InvalidProgramSegmentType(u32),
    InvalidProgramHeaderFlags(u32),
    InvalidProgramHeaderAlignment,
    MultipleProgramHeaderEntriesFound,

    InvalidSectionHeaderType(u32),
    MissingStringTable,
}

#[derive(Debug, PartialEq)]
enum InstructionSet {
    NoSpecific,
    Sparc,
    X86,
    MIPS,
    PowerPC,
    ARM,
    SuperH,
    Ia64,
    X86_64,
    AArch64,
    RiscV,
}

impl InstructionSet {
    fn try_from(value: u16) -> Result<Self, ElfParseError> {
        match value {
            0 => Ok(Self::NoSpecific),
            2 => Ok(Self::Sparc),
            3 => Ok(Self::X86),
            8 => Ok(Self::MIPS),
            0x14 => Ok(Self::PowerPC),
            0x28 => Ok(Self::ARM),
            0x2A => Ok(Self::SuperH),
            0x32 => Ok(Self::Ia64),
            0x3E => Ok(Self::X86_64),
            0xB7 => Ok(Self::AArch64),
            0xFE => Ok(Self::RiscV),
            _ => Err(ElfParseError::InvalidInstructionSetValue),
        }
    }
}

#[derive(Debug, PartialEq)]
enum ElfType {
    Relocatable,
    Executable,
    Shared,
    Core,
}

impl ElfType {
    fn try_from(value: u16) -> Result<Self, ElfParseError> {
        match value {
            1 => Ok(Self::Relocatable),
            2 => Ok(Self::Executable),
            3 => Ok(Self::Shared),
            4 => Ok(Self::Core),
            _ => Err(ElfParseError::InvalidElfType),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    fn from_byte(byte: u8) -> Result<Self, ElfParseError> {
        match byte {
            1 => Ok(Self::Little),
            2 => Ok(Self::Big),
            _ => Err(ElfParseError::InvalidEndianness),
        }
    }

    fn get_u16(&self, bytes: &[u8; 2]) -> u16 {
        match self {
            Endian::Big => u16::from_be_bytes(*bytes),
            Endian::Little => u16::from_le_bytes(*bytes),
        }
    }

    fn get_u32(&self, bytes: &[u8; 4]) -> u32 {
        match self {
            Endian::Big => u32::from_be_bytes(*bytes),
            Endian::Little => u32::from_le_bytes(*bytes),
        }
    }

    fn get_u64(&self, bytes: &[u8; 8]) -> u64 {
        match self {
            Endian::Big => u64::from_be_bytes(*bytes),
            Endian::Little => u64::from_le_bytes(*bytes),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ElfFileClass {
    Elf32,
    Elf64,
}

fn is_elf_file(file_bytes: &[u8]) -> bool {
    // 0x7F followed by "ELF" in ASCII
    file_bytes.starts_with(&[0x7F, 0x45, 0x4C, 0x46])
}

impl ElfFileClass {
    pub fn from_bytes(file_bytes: &[u8]) -> Result<Self, ElfParseError> {
        if !is_elf_file(file_bytes) {
            return Err(ElfParseError::NotValidElfFile);
        }

        match file_bytes.get(4) {
            Some(1) => Ok(Self::Elf32),
            Some(2) => Ok(Self::Elf64),
            _ => Err(ElfParseError::InvalidClass),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ProgramSegmentType {
    Null,
    Load,
    Dynamic,
    Interpret,
    Note,
    SharedLibrary,
    ProgramHeader,
    // TODO: Figure out how to parse this for intel x86_64
    ProcessorSpecific(u32),
}

impl From<u32> for ProgramSegmentType {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Null,
            1 => Self::Load,
            2 => Self::Dynamic,
            3 => Self::Interpret,
            4 => Self::Note,
            5 => Self::SharedLibrary,
            6 => Self::ProgramHeader,
            _ => Self::ProcessorSpecific(value),
        }
    }
}

// TODO: use bitflags for this struct
#[derive(Debug, PartialEq)]
pub struct ProgramHeaderFlags {
    executable: bool,
    writable: bool,
    readable: bool,
}

impl ProgramHeaderFlags {
    pub fn new(executable: bool, writable: bool, readable: bool) -> Self {
        Self {
            executable,
            writable,
            readable,
        }
    }
}

impl From<u32> for ProgramHeaderFlags {
    fn from(value: u32) -> Self {
        Self {
            executable: (value & 1) == 1,
            writable: (value & 2) == 2,
            readable: (value & 4) == 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: How to create an executable ELF: use gcc docker image to compile a C
    // program that is ELF formatted by default

    #[test_case]
    fn elf_file_class() {
        let file_bytes = include_bytes!("elf/test_files/main_ret0.elf64");
        assert_eq!(
            ElfFileClass::Elf64,
            ElfFileClass::from_bytes(file_bytes).expect("Expected a valid Elf64 file")
        );

        let file_bytes = include_bytes!("elf/test_files/hello_world.elf64");
        assert_eq!(
            ElfFileClass::Elf64,
            ElfFileClass::from_bytes(file_bytes).expect("Expected a valid Elf64 file")
        );

        let file_bytes = include_bytes!("elf/test_files/hello_world.elf32");
        assert_eq!(
            ElfFileClass::Elf32,
            ElfFileClass::from_bytes(file_bytes).expect("Expected a valid Elf32 file")
        );
    }
}
