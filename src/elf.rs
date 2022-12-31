use core::{convert::TryInto, fmt::Debug};

use self::program_header::{Elf32ProgramHeaderIterator, Elf64ProgramHeaderIterator};

pub mod program_header;

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
pub struct Elf32ProgramHeaderSummary {
    table_position: u32,
    entry_size: u16,
    entry_count: u16,
}

impl Elf32ProgramHeaderSummary {
    // TODO: is usize correct?
    fn byte_offset(&self, entry_index: u16) -> Option<usize> {
        if entry_index < self.entry_count {
            Some((self.table_position + (self.entry_size as u32) * (entry_index as u32)) as usize)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
struct Elf32SectionHeaderSummary {
    table_position: u32,
    entry_size: u16,
    entry_count: u16,
    names_index: u16,
}

#[derive(Debug, PartialEq)]
struct Elf32Header {
    endianness: Endian,
    header_version: u8,
    os_abi: u8,
    elf_type: ElfType,
    instruction_set: InstructionSet,
    elf_version: u32,
    program_entry_position: u32,

    /// Required for elf_type Executable, but not for Relocatable
    program_header_summary: Option<Elf32ProgramHeaderSummary>,
    section_header_summary: Elf32SectionHeaderSummary,
}

#[derive(Debug, PartialEq)]
pub struct Elf64ProgramHeaderSummary {
    table_position: u64,
    entry_size: u16,
    entry_count: u16,
}

impl Elf64ProgramHeaderSummary {
    // TODO: is usize correct?
    fn byte_offset(&self, entry_index: u16) -> Option<usize> {
        if entry_index < self.entry_count {
            Some((self.table_position + (self.entry_size as u64) * (entry_index as u64)) as usize)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
struct Elf64SectionHeaderSummary {
    table_position: u64,
    entry_size: u16,
    entry_count: u16,
    names_index: u16,
}

#[derive(Debug, PartialEq)]
struct Elf64Header {
    endianness: Endian,
    header_version: u8,
    os_abi: u8,
    elf_type: ElfType,
    instruction_set: InstructionSet,
    elf_version: u32,
    program_entry_position: u64,

    /// Required for elf_type Executable, but not for Relocatable
    program_header_summary: Option<Elf64ProgramHeaderSummary>,
    section_header_summary: Elf64SectionHeaderSummary,
}

#[derive(PartialEq)]
pub struct Elf64File<'a> {
    file_bytes: &'a [u8],
    header: Elf64Header,
}

impl<'a> Elf64File<'a> {
    pub fn from_bytes(file_bytes: &'a [u8]) -> Result<Self, ElfParseError> {
        let class = ElfFileClass::from_bytes(file_bytes)?;
        if class != ElfFileClass::Elf64 {
            return Err(ElfParseError::WrongElfClass);
        }

        Ok(Self {
            file_bytes,
            header: Self::parse_header(file_bytes)?,
        })
    }

    fn parse_header(file_bytes: &'a [u8]) -> Result<Elf64Header, ElfParseError> {
        let endianness = Endian::from_byte(file_bytes[5])?;
        let instruction_set =
            InstructionSet::try_from(endianness.get_u16(&file_bytes[18..=19].try_into().unwrap()))?;
        let elf_type =
            ElfType::try_from(endianness.get_u16(&file_bytes[16..=17].try_into().unwrap()))?;
        let program_entry_position = endianness.get_u64(&file_bytes[24..=31].try_into().unwrap());

        Ok(Elf64Header {
            endianness,
            header_version: file_bytes[6],
            os_abi: file_bytes[7],
            elf_type,
            instruction_set,
            elf_version: endianness.get_u32(&file_bytes[20..=23].try_into().unwrap()),
            program_entry_position,
            program_header_summary: Self::parse_program_header_summary(file_bytes, &endianness),
            section_header_summary: Elf64SectionHeaderSummary {
                table_position: endianness.get_u64(&file_bytes[40..=47].try_into().unwrap()),
                entry_size: endianness.get_u16(&file_bytes[58..=59].try_into().unwrap()),
                entry_count: endianness.get_u16(&file_bytes[60..=61].try_into().unwrap()),
                names_index: endianness.get_u16(&file_bytes[62..=63].try_into().unwrap()),
            },
        })
    }

    fn parse_program_header_summary(
        file_bytes: &'a [u8],
        endianness: &Endian,
    ) -> Option<Elf64ProgramHeaderSummary> {
        let table_position = endianness.get_u64(&file_bytes[32..=39].try_into().unwrap());

        if table_position == 0 {
            None
        } else {
            Some(Elf64ProgramHeaderSummary {
                table_position,
                entry_size: endianness.get_u16(&file_bytes[54..=55].try_into().unwrap()),
                entry_count: endianness.get_u16(&file_bytes[56..=57].try_into().unwrap()),
            })
        }
    }

    pub fn program_headers(&self) -> Option<Elf64ProgramHeaderIterator> {
        self.header
            .program_header_summary
            .as_ref()
            .and_then(|header_summary| {
                Some(Elf64ProgramHeaderIterator::new(
                    self.file_bytes,
                    &self.header.endianness,
                    header_summary,
                ))
            })
    }
}

impl<'a> Debug for Elf64File<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.header))
    }
}

#[derive(PartialEq)]
pub struct Elf32File<'a> {
    file_bytes: &'a [u8],
    header: Elf32Header,
}

impl<'a> Elf32File<'a> {
    pub fn from_bytes(file_bytes: &'a [u8]) -> Result<Self, ElfParseError> {
        if !is_elf_file(&file_bytes) {
            return Err(ElfParseError::NotValidElfFile);
        }

        let header = Self::parse_header(file_bytes)?;

        Ok(Self { file_bytes, header })
    }

    fn parse_header(file_bytes: &'a [u8]) -> Result<Elf32Header, ElfParseError> {
        let endianness = Endian::from_byte(file_bytes[5])?;
        let instruction_set =
            InstructionSet::try_from(endianness.get_u16(&file_bytes[18..=19].try_into().unwrap()))?;
        let elf_type =
            ElfType::try_from(endianness.get_u16(&file_bytes[16..=17].try_into().unwrap()))?;
        let program_entry_position = endianness.get_u32(&file_bytes[24..=27].try_into().unwrap());

        Ok(Elf32Header {
            endianness,
            header_version: file_bytes[6],
            os_abi: file_bytes[7],
            elf_type,
            instruction_set,
            elf_version: endianness.get_u32(&file_bytes[20..=23].try_into().unwrap()),
            program_entry_position,
            program_header_summary: Self::parse_program_header_summary(file_bytes, &endianness),
            section_header_summary: Elf32SectionHeaderSummary {
                table_position: endianness.get_u32(&file_bytes[32..=35].try_into().unwrap()),
                entry_size: endianness.get_u16(&file_bytes[46..=47].try_into().unwrap()),
                entry_count: endianness.get_u16(&file_bytes[48..=49].try_into().unwrap()),
                names_index: endianness.get_u16(&file_bytes[50..=51].try_into().unwrap()),
            },
        })
    }

    fn parse_program_header_summary(
        file_bytes: &'a [u8],
        endianness: &Endian,
    ) -> Option<Elf32ProgramHeaderSummary> {
        let table_position = endianness.get_u32(&file_bytes[28..=31].try_into().unwrap());

        if table_position == 0 {
            None
        } else {
            Some(Elf32ProgramHeaderSummary {
                table_position,
                entry_size: endianness.get_u16(&file_bytes[42..=43].try_into().unwrap()),
                entry_count: endianness.get_u16(&file_bytes[44..=45].try_into().unwrap()),
            })
        }
    }

    pub fn program_headers(&self) -> Option<Elf32ProgramHeaderIterator> {
        self.header
            .program_header_summary
            .as_ref()
            .and_then(|header_summary| {
                Some(Elf32ProgramHeaderIterator::new(
                    self.file_bytes,
                    &self.header.endianness,
                    header_summary,
                ))
            })
    }
}

impl<'a> Debug for Elf32File<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.header))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf::program_header::*;

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
    }

    #[test_case]
    fn parse_main_return_0_64_bit_relocatable() {
        let file_bytes = include_bytes!("elf/test_files/main_ret0.elf64");
        let elf_file =
            Elf64File::from_bytes(file_bytes).expect("The file should be correctly parsed");
        let expected_elf = Elf64File {
            file_bytes,
            header: Elf64Header {
                endianness: Endian::Little,
                header_version: 1,
                os_abi: 0,
                elf_type: ElfType::Relocatable,
                instruction_set: InstructionSet::X86_64,
                elf_version: 1,
                program_entry_position: 0,
                program_header_summary: None,
                section_header_summary: Elf64SectionHeaderSummary {
                    table_position: 552,
                    entry_size: 64,
                    entry_count: 11,
                    names_index: 1,
                },
            },
        };

        assert_eq!(expected_elf, elf_file);
        assert_eq!(None, elf_file.program_headers());
    }

    #[test_case]
    fn parse_hello_world_64_bit_executable() {
        let file_bytes = include_bytes!("elf/test_files/hello_world.elf64");
        let elf_file =
            Elf64File::from_bytes(file_bytes).expect("The file should be correctly parsed");
        let expected_elf = Elf64File {
            file_bytes,
            header: Elf64Header {
                endianness: Endian::Little,
                header_version: 1,
                os_abi: 0,
                elf_type: ElfType::Executable,
                instruction_set: InstructionSet::X86_64,
                elf_version: 1,
                program_entry_position: 0x401040,
                program_header_summary: Some(Elf64ProgramHeaderSummary {
                    table_position: 64,
                    entry_size: 56,
                    entry_count: 11,
                }),
                section_header_summary: Elf64SectionHeaderSummary {
                    table_position: 14520,
                    entry_size: 64,
                    entry_count: 29,
                    names_index: 28,
                },
            },
        };

        assert_eq!(expected_elf, elf_file);
        assert_eq!(11, elf_file.program_headers().unwrap().count());

        let mut headers = elf_file.program_headers().unwrap();
        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::ProgramHeader,
            ProgramHeaderFlags::new(false, false, true),
            0x40,
            0x400040,
            0x268,
            0x268,
            0x8,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::Interpret,
            ProgramHeaderFlags::new(false, false, true),
            0x2A8,
            0x4002A8,
            0x1C,
            0x1C,
            0x1,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::Load,
            ProgramHeaderFlags::new(false, false, true),
            0,
            0x400000,
            0x440,
            0x440,
            0x1000,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::Load,
            ProgramHeaderFlags::new(true, false, true),
            0x1000,
            0x401000,
            0x1BD,
            0x1BD,
            0x1000,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::Load,
            ProgramHeaderFlags::new(false, false, true),
            0x2000,
            0x402000,
            0x150,
            0x150,
            0x1000,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::Load,
            ProgramHeaderFlags::new(false, true, true),
            0x2E00,
            0x403E00,
            0x230,
            0x238,
            0x1000,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::Dynamic,
            ProgramHeaderFlags::new(false, true, true),
            0x2E10,
            0x403E10,
            0x1E0,
            0x1E0,
            0x8,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::Note,
            ProgramHeaderFlags::new(false, false, true),
            0x2C4,
            0x4002C4,
            0x20,
            0x20,
            0x4,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::ProcessorSpecific(1685382480),
            ProgramHeaderFlags::new(false, false, true),
            0x2010,
            0x402010,
            0x3C,
            0x3C,
            0x4,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::ProcessorSpecific(1685382481),
            ProgramHeaderFlags::new(false, true, true),
            0,
            0,
            0,
            0,
            0x10,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf64ProgramHeader::new(
            ProgramSegmentType::ProcessorSpecific(1685382482),
            ProgramHeaderFlags::new(false, false, true),
            0x2E00,
            0x403E00,
            0x200,
            0x200,
            0x1,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        assert_eq!(0, headers.count());
    }

    #[test_case]
    fn parse_hello_world_32_bit_executable() {
        let file_bytes = include_bytes!("elf/test_files/hello_world.elf32");
        let elf_file =
            Elf32File::from_bytes(file_bytes).expect("The file should be correctly parsed");
        let expected_elf = Elf32File {
            file_bytes,
            header: Elf32Header {
                endianness: Endian::Little,
                header_version: 1,
                os_abi: 0,
                elf_type: ElfType::Executable,
                instruction_set: InstructionSet::NoSpecific,
                elf_version: 1,
                program_entry_position: 0x401040,
                program_header_summary: Some(Elf32ProgramHeaderSummary {
                    table_position: 52,
                    entry_size: 32,
                    entry_count: 10,
                }),
                section_header_summary: Elf32SectionHeaderSummary {
                    table_position: 13624,
                    entry_size: 40,
                    entry_count: 29,
                    names_index: 28,
                },
            },
        };

        assert_eq!(expected_elf, elf_file);
        assert_eq!(10, elf_file.program_headers().unwrap().count());

        let mut headers = elf_file.program_headers().unwrap();
        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::ProgramHeader,
            ProgramHeaderFlags::new(false, false, true),
            0x34,
            0x400034,
            0x140,
            0x140,
            0x4,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::Interpret,
            ProgramHeaderFlags::new(false, false, true),
            0x2A8,
            0x4002A8,
            0x1C,
            0x1C,
            0x1,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::Load,
            ProgramHeaderFlags::new(false, false, true),
            0,
            0x400000,
            0x440,
            0x440,
            0x1000,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::Load,
            ProgramHeaderFlags::new(true, false, true),
            0x1000,
            0x401000,
            0x1BD,
            0x1BD,
            0x1000,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::Load,
            ProgramHeaderFlags::new(false, false, true),
            0x2000,
            0x402000,
            0x150,
            0x150,
            0x1000,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::Load,
            ProgramHeaderFlags::new(false, true, true),
            0x2E00,
            0x403E00,
            0x230,
            0x238,
            0x1000,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::Dynamic,
            ProgramHeaderFlags::new(false, true, true),
            0x2E10,
            0x403E10,
            0x1E0,
            0x1E0,
            0x8,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::Note,
            ProgramHeaderFlags::new(false, false, true),
            0x2C4,
            0x4002C4,
            0x20,
            0x20,
            0x4,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::ProcessorSpecific(1685382480),
            ProgramHeaderFlags::new(false, false, true),
            0x2010,
            0x402010,
            0x3C,
            0x3C,
            0x4,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        let expected_program_header = Elf32ProgramHeader::new(
            ProgramSegmentType::ProcessorSpecific(1685382481),
            ProgramHeaderFlags::new(false, true, true),
            0,
            0,
            0,
            0,
            0x4,
        );
        assert_eq!(
            expected_program_header,
            headers
                .next()
                .unwrap()
                .expect("Failed to parse program header")
        );

        assert_eq!(0, headers.count());
    }
}
