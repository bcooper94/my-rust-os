use core::{array::TryFromSliceError, convert::TryInto, fmt::Debug};

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
    FailedToParseValue,

    InvalidProgramSegmentType(u32),
    InvalidProgramHeaderFlags(u32),
    InvalidProgramHeaderAlignment,
    MultipleProgramHeaderEntriesFound,

    InvalidSectionHeaderType(u32),
    MissingStringTable,
}

impl From<TryFromSliceError> for ElfParseError {
    fn from(_: TryFromSliceError) -> Self {
        Self::FailedToParseValue
    }
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

    fn get_u16(&self, bytes: &[u8]) -> Result<u16, TryFromSliceError> {
        match self {
            Endian::Big => Ok(u16::from_be_bytes(bytes[..2].try_into()?)),
            Endian::Little => Ok(u16::from_le_bytes(bytes[..2].try_into()?)),
        }
    }

    fn get_u32(&self, bytes: &[u8]) -> Result<u32, TryFromSliceError> {
        match self {
            Endian::Big => Ok(u32::from_be_bytes(bytes[..4].try_into()?)),
            Endian::Little => Ok(u32::from_le_bytes(bytes[..4].try_into()?)),
        }
    }

    fn get_u64(&self, bytes: &[u8]) -> Result<u64, TryFromSliceError> {
        match self {
            Endian::Big => Ok(u64::from_be_bytes(bytes[..8].try_into()?)),
            Endian::Little => Ok(u64::from_le_bytes(bytes[..8].try_into()?)),
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

trait ParseAddress<Address> {
    fn parse_address(endianness: Endian, data: &[u8]) -> Result<Address, ElfParseError>;
}

struct Parse32BitAddress {}

impl ParseAddress<u32> for Parse32BitAddress {
    fn parse_address(endianness: Endian, data: &[u8]) -> Result<u32, ElfParseError> {
        Ok(endianness.get_u32(data)?)
    }
}

struct Parse64BitAddress {}

impl ParseAddress<u64> for Parse64BitAddress {
    fn parse_address(endianness: Endian, data: &[u8]) -> Result<u64, ElfParseError> {
        Ok(endianness.get_u64(data)?)
    }
}

trait ElfHeader<AddressSize> {
    type AddressParser: ParseAddress<AddressSize>;

    const PROG_HEADER_TABLE_POS_INDEX: usize;
    const PROG_HEADER_ENTRY_SIZE_INDEX: usize;
    const PROG_HEADER_ENTRY_COUNT_INDEX: usize;

    const SECTION_HEADER_TABLE_POS_INDEX: usize;
    const SECTION_HEADER_ENTRY_SIZE_INDEX: usize;
    const SECTION_HEADER_ENTRY_COUNT_INDEX: usize;
    const SECTION_HEADER_NAMES_INDEX_INDEX: usize;

    fn new(
        endianness: Endian,
        header_version: u8,
        os_abi: u8,
        elf_type: ElfType,
        instruction_set: InstructionSet,
        elf_version: u32,
        program_entry_position: AddressSize,
        program_header_table_position: AddressSize,
        program_header_entry_size: u16,
        program_header_entry_count: u16,
        section_header_table_position: AddressSize,
        section_header_entry_size: u16,
        section_header_entry_count: u16,
        section_names_index: u16,
    ) -> Self;

    fn from_bytes(file_bytes: &[u8]) -> Result<Self, ElfParseError>
    where
        Self: Sized,
    {
        let endianness = Endian::from_byte(file_bytes[5])?;
        let elf_type = ElfType::try_from(endianness.get_u16(&file_bytes[16..])?)?;
        let instruction_set = InstructionSet::try_from(endianness.get_u16(&file_bytes[18..])?)?;

        let program_entry_position =
            Self::AddressParser::parse_address(endianness, &file_bytes[24..])?;

        Ok(Self::new(
            endianness,
            file_bytes[6],
            file_bytes[7],
            elf_type,
            instruction_set,
            endianness.get_u32(&file_bytes[20..])?,
            program_entry_position,
            Self::AddressParser::parse_address(
                endianness,
                &file_bytes[Self::PROG_HEADER_TABLE_POS_INDEX..],
            )?,
            endianness.get_u16(&file_bytes[Self::PROG_HEADER_ENTRY_SIZE_INDEX..])?,
            endianness.get_u16(&file_bytes[Self::PROG_HEADER_ENTRY_COUNT_INDEX..])?,
            Self::AddressParser::parse_address(
                endianness,
                &file_bytes[Self::SECTION_HEADER_TABLE_POS_INDEX..],
            )?,
            endianness.get_u16(&file_bytes[Self::SECTION_HEADER_ENTRY_SIZE_INDEX..])?,
            endianness.get_u16(&file_bytes[Self::SECTION_HEADER_ENTRY_COUNT_INDEX..])?,
            endianness.get_u16(&file_bytes[Self::SECTION_HEADER_NAMES_INDEX_INDEX..])?,
        ))
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
