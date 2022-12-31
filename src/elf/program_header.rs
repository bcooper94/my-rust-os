use core::convert::TryInto;

use super::{Elf32ProgramHeaderSummary, Elf64ProgramHeaderSummary, ElfParseError, Endian};

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

impl ProgramSegmentType {
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

impl ProgramHeaderFlags {
    fn from(value: u32) -> Self {
        Self {
            executable: (value & 1) == 1,
            writable: (value & 2) == 2,
            readable: (value & 4) == 4,
        }
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
                        .get_u32(&self.data[byte_offset..=byte_offset + 3].try_into().unwrap()),
                );

                let header = Elf32ProgramHeader {
                    segment_type,
                    flags: ProgramHeaderFlags::from(
                        self.endianness.get_u32(
                            &self.data[byte_offset + 24..=byte_offset + 27]
                                .try_into()
                                .unwrap(),
                        ),
                    ),
                    p_offset: self.endianness.get_u32(
                        &self.data[byte_offset + 4..=byte_offset + 7]
                            .try_into()
                            .unwrap(),
                    ),
                    p_vaddr: self.endianness.get_u32(
                        &self.data[byte_offset + 8..=byte_offset + 11]
                            .try_into()
                            .unwrap(),
                    ),
                    p_filesz: self.endianness.get_u32(
                        &self.data[byte_offset + 16..=byte_offset + 19]
                            .try_into()
                            .unwrap(),
                    ),
                    p_memsz: self.endianness.get_u32(
                        &self.data[byte_offset + 20..=byte_offset + 23]
                            .try_into()
                            .unwrap(),
                    ),
                    alignment: self.endianness.get_u32(
                        &self.data[byte_offset + 28..=byte_offset + 31]
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
