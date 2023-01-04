use core::fmt::Debug;

use super::{
    is_elf_file, ElfHeader, ElfParseError, ElfType, Endian, InstructionSet, Parse32BitAddress,
    ProgramHeaderFlags, ProgramSegmentType,
};

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

impl ElfHeader<u32> for Elf32Header {
    type AddressParser = Parse32BitAddress;

    const PROG_HEADER_TABLE_POS_INDEX: usize = 28;
    const PROG_HEADER_ENTRY_SIZE_INDEX: usize = 42;
    const PROG_HEADER_ENTRY_COUNT_INDEX: usize = 44;

    const SECTION_HEADER_TABLE_POS_INDEX: usize = 32;
    const SECTION_HEADER_ENTRY_SIZE_INDEX: usize = 46;
    const SECTION_HEADER_ENTRY_COUNT_INDEX: usize = 48;
    const SECTION_HEADER_NAMES_INDEX_INDEX: usize = 50;

    fn new(
        endianness: Endian,
        header_version: u8,
        os_abi: u8,
        elf_type: ElfType,
        instruction_set: InstructionSet,
        elf_version: u32,
        program_entry_position: u32,
        program_header_table_position: u32,
        program_header_entry_size: u16,
        program_header_entry_count: u16,
        section_header_table_position: u32,
        section_header_entry_size: u16,
        section_header_entry_count: u16,
        section_names_index: u16,
    ) -> Self {
        let program_header_summary = if program_header_table_position == 0 {
            None
        } else {
            Some(Elf32ProgramHeaderSummary {
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
            section_header_summary: Elf32SectionHeaderSummary {
                table_position: section_header_table_position,
                entry_size: section_header_entry_size,
                entry_count: section_header_entry_count,
                names_index: section_names_index,
            },
        }
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
        Elf32Header::from_bytes(file_bytes)
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
pub struct Elf32ProgramHeader {
    segment_type: ProgramSegmentType,
    flags: ProgramHeaderFlags,

    /// The offset in the file that the data for this segment can be found.
    p_offset: u32,

    /// Where you should start to put this segment in virtual memory.
    p_vaddr: u32,

    /// Size of the segment in the file.
    p_filesz: u32,

    /// Size of the segment in memory. This can be 0. If the p_filesz and
    /// p_memsz members differ, this indicates that the segment is padded with
    /// zeros. All bytes in memory between the ending offset of the file size,
    /// and the segment's virtual memory size are to be cleared with zeros.
    p_memsz: u32,

    /// Required alignment for this section. Must be a power of 2.
    alignment: u32,
}

impl Elf32ProgramHeader {
    pub fn new(
        segment_type: ProgramSegmentType,
        flags: ProgramHeaderFlags,
        p_offset: u32,
        p_vaddr: u32,
        p_filesz: u32,
        p_memsz: u32,
        alignment: u32,
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
pub struct Elf32ProgramHeaderIterator<'a> {
    current_index: u16,
    data: &'a [u8],
    endianness: &'a Endian,
    header_summary: &'a Elf32ProgramHeaderSummary,
    program_header_entry_seen: bool,
}

impl<'a> Elf32ProgramHeaderIterator<'a> {
    pub fn new(
        data: &'a [u8],
        endianness: &'a Endian,
        header_summary: &'a Elf32ProgramHeaderSummary,
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

impl<'a> Iterator for Elf32ProgramHeaderIterator<'a> {
    type Item = Result<Elf32ProgramHeader, ElfParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index == self.header_summary.entry_count {
            None
        } else {
            if let Some(byte_offset) = self.header_summary.byte_offset(self.current_index) {
                let segment_type = ProgramSegmentType::from(
                    self.endianness
                        .get_u32(&self.data[byte_offset..])
                        .expect("Failed to parse segment type"),
                );

                let header = Elf32ProgramHeader {
                    segment_type,
                    flags: ProgramHeaderFlags::from(
                        self.endianness
                            .get_u32(&self.data[byte_offset + 24..])
                            .expect("Failed to parse flags"),
                    ),
                    p_offset: self
                        .endianness
                        .get_u32(&self.data[byte_offset + 4..])
                        .expect("Failed to parse p_offset"),
                    p_vaddr: self
                        .endianness
                        .get_u32(&self.data[byte_offset + 8..])
                        .expect("Failed to parse p_vaddr"),
                    p_filesz: self
                        .endianness
                        .get_u32(&self.data[byte_offset + 16..])
                        .expect("Failed to parse p_filesz"),
                    p_memsz: self
                        .endianness
                        .get_u32(&self.data[byte_offset + 20..])
                        .expect("Failed to parse p_memsz"),
                    alignment: self
                        .endianness
                        .get_u32(&self.data[byte_offset + 28..])
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
    use super::*;

    #[test_case]
    fn parse_hello_world_32_bit_executable() {
        let file_bytes = include_bytes!("test_files/hello_world.elf32");
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
