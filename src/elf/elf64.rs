use core::fmt::Debug;

use self::sections::SectionHeaderIterator;

use super::{
    ElfFileClass, ElfHeader, ElfParseError, ElfType, Endian, InstructionSet, Parse64BitAddress,
    ProgramHeaderFlags, ProgramSegmentType,
};

pub mod sections;

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
pub struct Elf64SectionHeaderSummary {
    table_position: u64,
    entry_size: u16,
    entry_count: u16,
    names_index: u16,
}

impl Elf64SectionHeaderSummary {
    fn byte_offset(&self, entry_index: u16) -> Option<usize> {
        if entry_index < self.entry_count {
            Some((self.table_position + (self.entry_size as u64) * (entry_index as u64)) as usize)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Elf64Header {
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

impl ElfHeader<u64> for Elf64Header {
    type AddressParser = Parse64BitAddress;

    const PROG_HEADER_TABLE_POS_INDEX: usize = 32;
    const PROG_HEADER_ENTRY_SIZE_INDEX: usize = 54;
    const PROG_HEADER_ENTRY_COUNT_INDEX: usize = 56;

    const SECTION_HEADER_TABLE_POS_INDEX: usize = 40;
    const SECTION_HEADER_ENTRY_SIZE_INDEX: usize = 58;
    const SECTION_HEADER_ENTRY_COUNT_INDEX: usize = 60;
    const SECTION_HEADER_NAMES_INDEX_INDEX: usize = 62;

    fn new(
        endianness: Endian,
        header_version: u8,
        os_abi: u8,
        elf_type: ElfType,
        instruction_set: InstructionSet,
        elf_version: u32,
        program_entry_position: u64,
        program_header_table_position: u64,
        program_header_entry_size: u16,
        program_header_entry_count: u16,
        section_header_table_position: u64,
        section_header_entry_size: u16,
        section_header_entry_count: u16,
        section_names_index: u16,
    ) -> Self {
        let program_header_summary = if program_header_table_position == 0 {
            None
        } else {
            Some(Elf64ProgramHeaderSummary {
                table_position: program_header_table_position,
                entry_size: program_header_entry_size,
                entry_count: program_header_entry_count,
            })
        };

        Self {
            endianness,
            header_version,
            os_abi,
            elf_type,
            instruction_set,
            elf_version,
            program_entry_position,
            program_header_summary,
            section_header_summary: Elf64SectionHeaderSummary {
                table_position: section_header_table_position,
                entry_size: section_header_entry_size,
                entry_count: section_header_entry_count,
                names_index: section_names_index,
            },
        }
    }
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
        Elf64Header::from_bytes(file_bytes)
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

    pub fn section_headers(&self) -> Result<SectionHeaderIterator, ElfParseError> {
        SectionHeaderIterator::new(
            self.file_bytes,
            self.header.endianness,
            &self.header.section_header_summary,
        )
    }
}

impl<'a> Debug for Elf64File<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.header))
    }
}

#[derive(Debug, PartialEq)]
pub struct Elf64ProgramHeader {
    segment_type: ProgramSegmentType,
    flags: ProgramHeaderFlags,

    /// The offset in the file that the data for this segment can be found.
    p_offset: u64,

    /// Where you should start to put this segment in virtual memory.
    p_vaddr: u64,

    /// Size of the segment in the file.
    p_filesz: u64,

    /// Size of the segment in memory. This can be 0. If the p_filesz and
    /// p_memsz members differ, this indicates that the segment is padded with
    /// zeros. All bytes in memory between the ending offset of the file size,
    /// and the segment's virtual memory size are to be cleared with zeros.
    p_memsz: u64,

    /// Required alignment for this section. Must be a power of 2.
    alignment: u64,
}

impl Elf64ProgramHeader {
    pub fn new(
        segment_type: ProgramSegmentType,
        flags: ProgramHeaderFlags,
        p_offset: u64,
        p_vaddr: u64,
        p_filesz: u64,
        p_memsz: u64,
        alignment: u64,
    ) -> Self {
        Self {
            segment_type,
            flags,
            p_offset,
            p_vaddr,
            p_filesz,
            p_memsz,
            alignment,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Elf64ProgramHeaderIterator<'a> {
    current_index: u16,
    data: &'a [u8],
    endianness: &'a Endian,
    header_summary: &'a Elf64ProgramHeaderSummary,
    program_header_entry_seen: bool,
}

impl<'a> Elf64ProgramHeaderIterator<'a> {
    pub fn new(
        data: &'a [u8],
        endianness: &'a Endian,
        header_summary: &'a Elf64ProgramHeaderSummary,
    ) -> Self {
        Self {
            current_index: 0,
            data,
            endianness,
            header_summary,
            program_header_entry_seen: false,
        }
    }
}

impl<'a> Iterator for Elf64ProgramHeaderIterator<'a> {
    type Item = Result<Elf64ProgramHeader, ElfParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index == self.header_summary.entry_count {
            None
        } else {
            if let Some(byte_offset) = self.header_summary.byte_offset(self.current_index) {
                let segment_type = ProgramSegmentType::from(
                    self.endianness
                        .get_u32(&self.data[byte_offset..])
                        .expect("Failed to parse segment_type"),
                );

                let header = Elf64ProgramHeader {
                    segment_type,
                    flags: ProgramHeaderFlags::from(
                        self.endianness
                            .get_u32(&self.data[byte_offset + 4..])
                            .expect("Failed to parse flags"),
                    ),
                    p_offset: self
                        .endianness
                        .get_u64(&self.data[byte_offset + 8..])
                        .expect("Failed to parse p_offset"),
                    p_vaddr: self
                        .endianness
                        .get_u64(&self.data[byte_offset + 16..])
                        .expect("Failed to parse p_vaddr"),
                    p_filesz: self
                        .endianness
                        .get_u64(&self.data[byte_offset + 32..])
                        .expect("Failed to parse p_filesz"),
                    p_memsz: self
                        .endianness
                        .get_u64(&self.data[byte_offset + 40..])
                        .expect("Failed to parse p_memsz"),
                    alignment: self
                        .endianness
                        .get_u64(&self.data[byte_offset + 48..])
                        .expect("Failed to parse alignment"),
                };

                if !header.alignment.is_power_of_two() {
                    return Some(Err(ElfParseError::InvalidProgramHeaderAlignment));
                }

                if header.segment_type == ProgramSegmentType::ProgramHeader {
                    if self.program_header_entry_seen {
                        return Some(Err(ElfParseError::MultipleProgramHeaderEntriesFound));
                    }

                    self.program_header_entry_seen = true;
                }

                self.current_index += 1;
                Some(Ok(header))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sections::*;
    use super::*;

    #[test_case]
    fn parse_main_return_0_64_bit_relocatable() {
        let file_bytes = include_bytes!("test_files/main_ret0.elf64");
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
        let file_bytes = include_bytes!("test_files/hello_world.elf64");
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

        assert_eq!(
            29,
            elf_file
                .section_headers()
                .expect("Failed to create section iterator")
                .count()
        );

        let mut section_headers = elf_file
            .section_headers()
            .expect("Failed to create section iterator");

        let expected = SectionHeader::new(
            0,
            SectionHeaderType::Null,
            SectionHeaderFlags::empty(),
            None,
            0,
            0,
            None,
            None,
            0,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x1B,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::ALLOC,
            Some(0x4002A8),
            0x2A8,
            0x1C,
            None,
            None,
            1,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x23,
            SectionHeaderType::Note,
            SectionHeaderFlags::ALLOC,
            Some(0x4002C4),
            0x2C4,
            0x20,
            None,
            None,
            4,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x35,
            SectionHeaderType::Hash,
            SectionHeaderFlags::ALLOC,
            Some(0x4002E8),
            0x2E8,
            0x24,
            Some(5),
            None,
            8,
            Some(0x4),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x31,
            SectionHeaderType::OperatingSystemSpecific(0x6FFFFFF6),
            SectionHeaderFlags::ALLOC,
            Some(0x400310),
            0x310,
            0x1C,
            Some(5),
            None,
            8,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x3B,
            SectionHeaderType::DynamicSymbols,
            SectionHeaderFlags::ALLOC,
            Some(0x400330),
            0x330,
            0x60,
            Some(6),
            Some(1),
            8,
            Some(0x18),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x43,
            SectionHeaderType::StringTable,
            SectionHeaderFlags::ALLOC,
            Some(0x400390),
            0x390,
            0x3D,
            None,
            None,
            1,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x4B,
            SectionHeaderType::OperatingSystemSpecific(0x6FFFFFFF),
            SectionHeaderFlags::ALLOC,
            Some(0x4003CE),
            0x3CE,
            0x8,
            Some(5),
            None,
            2,
            Some(0x2),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x58,
            SectionHeaderType::OperatingSystemSpecific(0x6FFFFFFE),
            SectionHeaderFlags::ALLOC,
            Some(0x4003D8),
            0x3D8,
            0x20,
            Some(6),
            Some(1),
            8,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x67,
            SectionHeaderType::RelA,
            SectionHeaderFlags::ALLOC,
            Some(0x4003F8),
            0x3F8,
            0x30,
            Some(5),
            None,
            8,
            Some(0x18),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x71,
            SectionHeaderType::RelA,
            SectionHeaderFlags::ALLOC | SectionHeaderFlags::INFO_LINK,
            Some(0x400428),
            0x428,
            0x18,
            Some(5),
            Some(22),
            8,
            Some(0x18),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x7B,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::ALLOC | SectionHeaderFlags::EXECUTABLE_INSTRUCTIONS,
            Some(0x401000),
            0x1000,
            0x17,
            None,
            None,
            4,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x76,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::ALLOC | SectionHeaderFlags::EXECUTABLE_INSTRUCTIONS,
            Some(0x401020),
            0x1020,
            0x20,
            None,
            None,
            16,
            Some(0x10),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x81,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::ALLOC | SectionHeaderFlags::EXECUTABLE_INSTRUCTIONS,
            Some(0x401040),
            0x1040,
            0x171,
            None,
            None,
            16,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x87,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::ALLOC | SectionHeaderFlags::EXECUTABLE_INSTRUCTIONS,
            Some(0x4011B4),
            0x11B4,
            0x9,
            None,
            None,
            4,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x8D,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::ALLOC,
            Some(0x402000),
            0x2000,
            0x10,
            None,
            None,
            4,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x95,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::ALLOC,
            Some(0x402010),
            0x2010,
            0x3C,
            None,
            None,
            4,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xA3,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::ALLOC,
            Some(0x402050),
            0x2050,
            0x100,
            None,
            None,
            8,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xAD,
            SectionHeaderType::InitArray,
            SectionHeaderFlags::WRITE | SectionHeaderFlags::ALLOC,
            Some(0x403E00),
            0x2E00,
            0x8,
            None,
            None,
            8,
            Some(8),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xB9,
            SectionHeaderType::FinishArray,
            SectionHeaderFlags::WRITE | SectionHeaderFlags::ALLOC,
            Some(0x403E08),
            0x2E08,
            0x8,
            None,
            None,
            8,
            Some(0x8),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xC5,
            SectionHeaderType::Dynamic,
            SectionHeaderFlags::WRITE | SectionHeaderFlags::ALLOC,
            Some(0x403E10),
            0x2E10,
            0x1E0,
            Some(6),
            None,
            8,
            Some(0x10),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xCE,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::WRITE | SectionHeaderFlags::ALLOC,
            Some(0x403FF0),
            0x2FF0,
            0x10,
            None,
            None,
            8,
            Some(0x8),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xD3,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::WRITE | SectionHeaderFlags::ALLOC,
            Some(0x404000),
            0x3000,
            0x20,
            None,
            None,
            8,
            Some(0x8),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xDC,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::WRITE | SectionHeaderFlags::ALLOC,
            Some(0x404020),
            0x3020,
            0x10,
            None,
            None,
            8,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xE2,
            SectionHeaderType::NoBits,
            SectionHeaderFlags::WRITE | SectionHeaderFlags::ALLOC,
            Some(0x404030),
            0x3030,
            0x8,
            None,
            None,
            1,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0xE7,
            SectionHeaderType::ProgramBits,
            SectionHeaderFlags::MERGE | SectionHeaderFlags::STRINGS,
            None,
            0x3030,
            0x12,
            None,
            None,
            1,
            Some(0x1),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x1,
            SectionHeaderType::SymbolTable,
            SectionHeaderFlags::empty(),
            None,
            0x3048,
            0x5B8,
            Some(27),
            Some(43),
            8,
            Some(0x18),
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x9,
            SectionHeaderType::StringTable,
            SectionHeaderFlags::empty(),
            None,
            0x3600,
            0x1C4,
            None,
            None,
            1,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        let expected = SectionHeader::new(
            0x11,
            SectionHeaderType::StringTable,
            SectionHeaderFlags::empty(),
            None,
            0x37C4,
            0xF0,
            None,
            None,
            1,
            None,
        );
        assert_eq!(
            expected,
            section_headers
                .next()
                .unwrap()
                .expect("Failed to parse section header")
        );

        assert_eq!(0, section_headers.count());
    }
}
