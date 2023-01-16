use core::convert::{TryFrom, TryInto};

use crate::elf::{ElfParseError, Endian};
use bitflags::bitflags;

use super::Elf64SectionHeaderSummary;

#[derive(Debug, PartialEq)]
pub enum SectionHeaderType {
    /// This value marks the section header as inactive; it does not have an
    /// associated section.
    Null,
    /// The section holds information defined by the program, whose format and
    /// meaning are determined solely by the program.
    ProgramBits,
    /// Currently, an object file may have only one section of each type, but
    /// this restriction may be relaxed in the future. Typically, SHT_SYMTAB
    /// provides symbols for link editing, though it may also be used for
    /// dynamic linking. As a complete symbol table, it may contain many symbols
    /// unnecessary for dynamic linking. Consequently, an object file may also
    /// contain a SHT_DYNSYM section, which holds a minimal set of dynamic
    /// linking symbols, to save space.
    SymbolTable,
    /// The section holds a string table. An object file may have multiple
    /// string table sections.
    StringTable,
    /// The section holds relocation entries with explicit addends.
    RelA,
    /// The section holds a symbol hash table. All objects participating in
    /// dynamic linking must contain a symbol hash table. Currently, an object
    /// file may have only one hash table, but this restriction may be relaxed
    /// in the future
    Hash,
    /// The section holds information for dynamic linking. Currently, an object
    /// file may have only one dynamic section, but this restriction may be
    /// relaxed in the future.
    Dynamic,
    /// The section holds information that marks the file in some way.
    Note,
    /// A section of this type occupies no space in the file but otherwise
    /// resembles SHT_PROGBITS. Although this section contains no bytes, the
    /// sh_offset member contains the conceptual file offset.
    NoBits,
    /// The section holds relocation entries without explicit addends.
    Rel,
    /// This section type is reserved but has unspecified semantics. Programs
    /// that contain a section of this type do not conform to the ABI.
    ShLib,
    /// Holds a minimal set of dynamic linking symbols to save space.
    DynamicSymbols,
    /// This section contains an array of pointers to initialization functions,
    /// as described in "Initialization and Termination Functions" in Chapter 5.
    /// Each pointer in the array is taken as a parameterless procedure with a
    /// void return.
    /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html
    InitArray,
    /// This section contains an array of pointers to termination functions, as
    /// described in "Initialization and Termination Functions" in Chapter 5.
    /// Each pointer in the array is taken as a parameterless procedure with a
    /// void return.
    /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html
    FinishArray,
    /// This section contains an array of pointers to functions that are invoked
    /// before all other initialization functions, as described in
    /// "Initialization and Termination Functions" in Chapter 5. Each pointer in
    /// the array is taken as a parameterless procedure with a void return.
    /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html
    PreinitArray,
    /// This section defines a section group. A section group is a set of
    /// sections that are related and that must be treated specially by the
    /// linker (see below for further details). Sections of type `Group` may
    /// appear only in relocatable objects (objects with the ELF header
    /// `header_type` member set to `Rel`). The section header table entry for a
    /// group section must appear in the section header table before the entries
    /// for any of the sections that are members of the group.
    /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html
    Group,
    ///  This section is associated with a section of type `Symboltable` and is
    /// required if any of the section header indexes referenced by that symbol
    /// table contain the escape value `SHN_XINDEX`. The section is an array of
    /// Elf32_Word values. Each value corresponds one to one with a symbol table
    /// entry and appear in the same order as those entries. The values
    /// represent the section header indexes against which the symbol table
    /// entries are defined. Only if corresponding symbol table entry's
    /// `st_shndx` field contains the escape value `SHN_XINDEX` will the
    /// matching Elf32_Word hold the actual section header index; otherwise, the
    /// entry must be `SHN_UNDEF` (0).
    /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html
    SymbolTableSectionHeaderIndex,
    /// Values in the range from 0x60000000 through 0x6fffffff inclusive are
    /// reserved for operating system-specific semantics.
    OperatingSystemSpecific(u32),
    /// Values in inclusive range from 0x70000000 through 0x7fffffff inclusive
    /// are reserved for processor-specific semantics.
    ProcessorSpecific(u32),
    /// Values in the inclusive range from 0x80000000 through 0xffffffff of
    /// indexes are reserved for application programs. These sections may be
    /// used by the application, without conflicting with current or future
    /// system-defined section types.
    UserApplicationSpecific(u32),
}

impl TryFrom<u32> for SectionHeaderType {
    type Error = ElfParseError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Null),
            1 => Ok(Self::ProgramBits),
            2 => Ok(Self::SymbolTable),
            3 => Ok(Self::StringTable),
            4 => Ok(Self::RelA),
            5 => Ok(Self::Hash),
            6 => Ok(Self::Dynamic),
            7 => Ok(Self::Note),
            8 => Ok(Self::NoBits),
            9 => Ok(Self::Rel),
            10 => Ok(Self::ShLib),
            11 => Ok(Self::DynamicSymbols),
            14 => Ok(Self::InitArray),
            15 => Ok(Self::FinishArray),
            16 => Ok(Self::PreinitArray),
            17 => Ok(Self::Group),
            18 => Ok(Self::SymbolTableSectionHeaderIndex),
            0x60000000..=0x6FFFFFFF => Ok(Self::OperatingSystemSpecific(value)),
            0x70000000..=0x7FFFFFFF => Ok(Self::ProcessorSpecific(value)),
            0x80000000..=0xFFFFFFFF => Ok(Self::UserApplicationSpecific(value)),
            _ => Err(ElfParseError::InvalidSectionHeaderType(value)),
        }
    }
}

bitflags! {
    /// Definitions sourced from
    /// https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html,
    /// https://www.freebsd.org/cgi/man.cgi?elf(5), and
    /// http://www.skyfree.org/linux/references/ELF_Format.pdf.
    #[repr(transparent)]
    pub struct SectionHeaderFlags: u64 {
        /// The section contains data that should be writable during process
        /// execution.
        const WRITE = 1;
        /// The section occupies memory during process execution. Some control
        /// sections do not reside in the memory image of an object file; this
        /// attribute is off for those sections.
        const ALLOC = 1 << 1;
        /// All bits included in this mask are reserved for processor-specific
        /// semantics.
        const EXECUTABLE_INSTRUCTIONS = 1 << 2;
        /// The data in the section may be merged to eliminate duplication.
        /// Unless the `STRINGS` flag is also set, the data elements in the
        /// section are of a uniform size. The size of each element is specified
        /// in the section header's sh_entsize field. If the `STRINGS` flag is
        /// also set, the data elements consist of null-terminated character
        /// strings. The size of each character is specified in the section
        /// header's `section_entry_size` field.
        ///
        /// Each element in the section is compared against other elements in
        /// sections with the same name, type and flags. Elements that would
        /// have identical values at program run-time may be merged. Relocations
        /// referencing elements of such sections must be resolved to the merged
        /// locations of the referenced values. Note that any relocatable
        /// values, including values that would result in run-time relocations,
        /// must be analyzed to determine whether the run-time values would
        /// actually be identical. An ABI-conforming object file may not depend
        /// on specific elements being merged, and an ABI-conforming link editor
        /// may choose not to merge specific elements.
        /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html.
        const MERGE = 1 << 4;
        /// The data elements in the section consist of null-terminated
        /// character strings. The size of each character is specified in the
        /// section header's `section_entry_size` field.
        ///
        /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html.
        const STRINGS = 1 << 5;
        /// The `info` field of this section header holds a section header table
        /// index.
        const INFO_LINK = 1 << 6;
        /// This flag adds special ordering requirements for link editors. The
        /// requirements apply if the sh_link field of this section's header
        /// references another section (the linked-to section). If this section
        /// is combined with other sections in the output file, it must appear
        /// in the same relative order with respect to those sections, as the
        /// linked-to section appears with respect to sections the linked-to
        /// section is combined with.
        ///
        /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html.
        const LINK_ORDER = 1 << 7;
        /// This section requires special OS-specific processing (beyond the
        /// standard linking rules) to avoid incorrect behavior. If this section
        /// has either a `type` value or contains `flags` bits in the
        /// OS-specific ranges for those fields, and a link editor processing
        /// this section does not recognize those values, then the link editor
        /// should reject the object file containing this section with an error.
        /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html.
        const OS_NONCONFORMING = 1 << 8;
        /// This section is a member (perhaps the only one) of a section group.
        /// The section must be referenced by a section of type `GROUP`. The
        /// `GROUP` flag may be set only for sections contained in relocatable
        /// objects (objects with the ELF header type `Rel`).
        /// Source: https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html.
        const GROUP = 1 << 9;
        /// This section holds thread-local storage meaning that each separate
        /// execution flow has its own distinct instance of this data.
        /// Implementations need not support this flag. Source:
        /// https://refspecs.linuxbase.org/elf/gabi4+/ch4.sheader.html.
        const TLS = 1 << 10;
        /// This section's data is compressed. Source:
        /// https://www.freebsd.org/cgi/man.cgi?elf(5)
        const COMPRESSED = 1 << 11;
        const OS_SPECIFIC_MASK = 0xFF000000;
        /// All bits included in this mask are reserved for processor-specific
        /// semantics.
        const PROCESSOR_SPECIFIC_MASK = 0xF0000000;
    }
}

#[derive(Debug, PartialEq)]
pub struct SectionHeader {
    /// Specifies the index into the section header string table section for
    /// this section's name, giving the location of a null- terminated string.
    name_index: u32,
    /// This member categorizes the section’s contents and semantics. Section
    /// types and their descriptions appear below.
    header_type: SectionHeaderType,
    /// Sections support 1-bit flags that describe miscellaneous attributes. See
    /// `SectionHeaderFlags` for more information.
    flags: SectionHeaderFlags,
    /// If the section will appear in the memory image of a process,
    /// this member gives the address at which the section’s first byte should
    /// reside.
    address: Option<u64>,
    /// The byte offset from the beginning of the file to the first byte in the
    /// section. The `NoBits` section type occupies no space in the file, and
    /// its `section_file_offset` locates the conceptual placement in the file.
    section_file_offset: u64,
    /// Unless the section type is `NoBits`, the section occupies `section_size`
    /// bytes in the file. A section of type NoBits may have a non-zero size,
    /// but it occupies no space in the file.
    section_size: u64,
    /// Section header table index link, whose interpretation depends on the
    /// section type.
    /// `Dynamic`: The section header index of the string table used by entries
    /// in the section.
    /// `Hash`: The section header index of the symbol table to which the hash
    /// table applies.
    /// `Rel` and `RelA`: The section header index of the associated symbol table.
    /// `SymbolTable` and `DynamicSymbols`: The section header index of the
    /// associated string table.
    /// Other: None.
    section_link_index: Option<u32>,
    /// This member holds extra information, whose interpretation depends on the
    /// section type.
    /// `Rel` and `RelA`: The section header index of the section to which the
    /// relocation applies
    /// `SymbolTable` and `DynamicSymbols`: One greater than the symbol table
    /// index of the last local symbol (binding STB_LOCAL).
    /// Other: None.
    info: Option<u32>,
    /// Some sections have address alignment constraints. For example, if a
    /// section holds a doubleword, the system must ensure doubleword alignment
    /// for the entire section. That is, the value of `address` must be
    /// congruent to 0, modulo the value of `address_alignment`. Currently, only
    /// 0 and positive integral powers of two are allowed. Values 0 and 1 mean
    /// the section has no alignment constraints.
    address_alignment: u64,
    /// Some sections hold a table of fixed-size entries, such as a symbol
    /// table. For such a section, this member gives the size in bytes of each
    /// entry.
    section_entry_size: Option<u64>,
}

impl SectionHeader {
    pub fn new(
        name_index: u32,
        header_type: SectionHeaderType,
        flags: SectionHeaderFlags,
        address: Option<u64>,
        section_file_offset: u64,
        section_size: u64,
        section_link_index: Option<u32>,
        info: Option<u32>,
        address_alignment: u64,
        section_entry_size: Option<u64>,
    ) -> Self {
        Self {
            name_index,
            header_type,
            flags,
            address,
            section_file_offset,
            section_size,
            section_link_index,
            info,
            address_alignment,
            section_entry_size,
        }
    }
}

struct StringTable<'a> {
    data: &'a [u8],
    section_header: SectionHeader,
}

impl<'a> StringTable<'a> {
    fn get_string(&self, index: u32) -> Option<&'a str> {
        Some("")
    }
}

pub struct SectionHeaderIterator<'a> {
    current_index: u16,
    data: &'a [u8],
    endianness: Endian,
    section_header_summary: &'a Elf64SectionHeaderSummary,
}

trait GenericSectionHeaderIterator<'a>: Iterator {
    type Address;

    const ENDIANNESS: Endian;

    fn parse_address(&self, data: &'a [u8]) -> Result<Self::Address, ElfParseError>;
}

impl<'a> SectionHeaderIterator<'a> {
    pub fn new(
        data: &'a [u8],
        endianness: Endian,
        section_header_summary: &'a Elf64SectionHeaderSummary,
    ) -> Result<Self, ElfParseError> {
        Ok(Self {
            current_index: 0,
            data,
            endianness,
            section_header_summary,
        })
    }

    fn parse_section_header(&self) -> Result<SectionHeader, ElfParseError> {
        let byte_offset = self
            .section_header_summary
            .byte_offset(self.current_index)
            .unwrap();
        let name_index = self.endianness.get_u32(&self.data[byte_offset..])?;
        let header_type =
            SectionHeaderType::try_from(self.endianness.get_u32(&self.data[byte_offset + 4..])?)?;
        let flags = self.endianness.get_u64(&self.data[byte_offset + 8..])?;
        let address = match self.endianness.get_u64(&self.data[byte_offset + 16..])? {
            0 => None,
            value => Some(value),
        };
        let section_file_offset = self.endianness.get_u64(&self.data[byte_offset + 24..])?;
        let section_size = self.endianness.get_u64(&self.data[byte_offset + 32..])?;
        let section_link_index = match self.endianness.get_u32(&self.data[byte_offset + 40..])? {
            0 => None,
            value => Some(value),
        };
        let info = match self.endianness.get_u32(&self.data[byte_offset + 44..])? {
            0 => None,
            value => Some(value),
        };
        let address_alignment = self.endianness.get_u64(&self.data[byte_offset + 48..])?;
        let section_entry_size = match self.endianness.get_u64(&self.data[byte_offset + 56..])? {
            0 => None,
            value => Some(value),
        };

        Ok(SectionHeader {
            name_index,
            header_type,
            flags: SectionHeaderFlags::from_bits_truncate(flags),
            address,
            section_file_offset,
            section_size,
            section_link_index,
            info,
            address_alignment,
            section_entry_size,
        })
    }
}

impl<'a> Iterator for SectionHeaderIterator<'a> {
    type Item = Result<SectionHeader, ElfParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index == self.section_header_summary.entry_count {
            None
        } else {
            let header = self.parse_section_header();
            self.current_index += 1;
            Some(header)
        }
    }
}
