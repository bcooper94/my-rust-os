use core::{convert::TryInto, fmt::Debug};

use super::{
    ElfFileClass, ElfParseError, ElfType, Endian, InstructionSet, ProgramHeaderFlags,
    ProgramSegmentType,
};

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
                        .get_u32(&self.data[byte_offset..=byte_offset + 3].try_into().unwrap()),
                );

                let header = Elf64ProgramHeader {
                    segment_type,
                    flags: ProgramHeaderFlags::from(
                        self.endianness.get_u32(
                            &self.data[byte_offset + 4..=byte_offset + 7]
                                .try_into()
                                .unwrap(),
                        ),
                    ),
                    p_offset: self.endianness.get_u64(
                        &self.data[byte_offset + 8..=byte_offset + 15]
                            .try_into()
                            .unwrap(),
                    ),
                    p_vaddr: self.endianness.get_u64(
                        &self.data[byte_offset + 16..=byte_offset + 23]
                            .try_into()
                            .unwrap(),
                    ),
                    p_filesz: self.endianness.get_u64(
                        &self.data[byte_offset + 32..=byte_offset + 39]
                            .try_into()
                            .unwrap(),
                    ),
                    p_memsz: self.endianness.get_u64(
                        &self.data[byte_offset + 40..=byte_offset + 47]
                            .try_into()
                            .unwrap(),
                    ),
                    alignment: self.endianness.get_u64(
                        &self.data[byte_offset + 48..=byte_offset + 55]
                            .try_into()
                            .unwrap(),
                    ),
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
    }
}
